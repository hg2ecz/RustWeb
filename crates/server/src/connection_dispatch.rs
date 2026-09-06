use crate::check_route_rate_limit;
use crate::connection::ConnectionServices;
use crate::http_dispatch;
use crate::http_io::{HttpRequest, ParsedHead, Response, read_buffered_body};
use crate::presentation::{
    accepts_media, app_error_response, authorize_route, read_error_response, route_returns_json,
};
use crate::request_input::build_upload_runtime_value;
use crate::web_security::validate_browser_state_change;
use auth::SessionSnapshot;
use language_core::{HttpMethod, Program, Route, ServerConfig, UploadField, Value};
use observability::server_log;
use runtime::{AppResponse, ExecutionLimits, ResourceProfiles, execute_request_with_profiles};
use std::io::Cursor;
use std::time::Duration;
use storage::{AppFs, UploadError, multipart_boundary, store_single_multipart_file};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use tokio::time::timeout;

pub(super) struct DispatchOutcome {
    pub(super) response: Response,
    pub(super) force_close: bool,
}

impl DispatchOutcome {
    fn keep_connection(response: Response) -> Self {
        Self {
            response,
            force_close: false,
        }
    }

    fn close_connection(response: Response) -> Self {
        Self {
            response,
            force_close: true,
        }
    }
}

pub(super) struct RequestExecutionContext<'a, 's> {
    pub(super) program: &'a Program,
    pub(super) config: &'a ServerConfig,
    pub(super) appfs: Option<&'a AppFs>,
    pub(super) resource_profiles: &'a ResourceProfiles,
    pub(super) max_image_pixels: u64,
    pub(super) request_stub: &'a HttpRequest,
    pub(super) request_accept: Option<&'a str>,
    pub(super) path: &'a str,
    pub(super) request_id: &'a str,
    pub(super) request_is_https: bool,
    pub(super) expected_host: Option<&'a str>,
    pub(super) effective_peer: &'a str,
    pub(super) domain_namespace: &'a str,
    pub(super) session: &'a SessionSnapshot,
    pub(super) services: &'a ConnectionServices<'s>,
}

pub(super) async fn dispatch_upload<S>(
    stream: &mut S,
    buffer: &mut Vec<u8>,
    head: &ParsedHead,
    route: &Route,
    upload: &UploadField,
    ctx: &RequestExecutionContext<'_, '_>,
) -> DispatchOutcome
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    let json_api = route_returns_json(ctx.program, route);
    if head.content_length as u64
        > ctx
            .appfs
            .map(|fs| fs.limits().max_file_bytes)
            .unwrap_or(0)
            .saturating_add(64 * 1024)
    {
        return DispatchOutcome::keep_connection(Response::text(
            413,
            "Content Too Large",
            b"upload too large\n",
        ));
    }
    let Some(fs) = ctx.appfs else {
        return DispatchOutcome::keep_connection(Response::text(
            503,
            "Service Unavailable",
            b"upload storage unavailable\n",
        ));
    };
    if !ctx.request_is_https && !ctx.config.insecure_dev_cookies {
        return DispatchOutcome::keep_connection(Response::text(
            426,
            "Upgrade Required",
            b"HTTPS required\n",
        ));
    }
    if let Err(response) = validate_browser_state_change(
        ctx.request_stub,
        ctx.request_is_https,
        ctx.expected_host,
        ctx.services.web,
    ) {
        return DispatchOutcome::keep_connection(response);
    }
    if let Some(response) = authorize_route(&route.auth, ctx.session, json_api) {
        return DispatchOutcome::keep_connection(response);
    }
    if let Some(response) = check_route_rate_limit(
        ctx.services.route_rate_limiter,
        route,
        ctx.effective_peer,
        ctx.session.principal.as_deref(),
        json_api,
    )
    .await
    {
        return DispatchOutcome::keep_connection(response);
    }

    let boundary = match multipart_boundary(ctx.request_stub.header("content-type").unwrap_or("")) {
        Ok(value) => value,
        Err(_) => {
            return DispatchOutcome::close_connection(Response::text(
                415,
                "Unsupported Media Type",
                b"expected multipart/form-data\n",
            ));
        }
    };
    let destination = match fs.random_upload_destination(&upload.destination) {
        Ok(value) => value,
        Err(_) => {
            return DispatchOutcome::close_connection(Response::text(
                500,
                "Internal Server Error",
                b"upload destination error\n",
            ));
        }
    };

    let initial_len = buffer.len().min(head.content_length);
    let initial: Vec<u8> = buffer.drain(..initial_len).collect();
    let remaining = head.content_length - initial_len;
    let reader = Cursor::new(initial).chain((&mut *stream).take(remaining as u64));
    let stored = timeout(
        Duration::from_millis(ctx.config.request_timeout_ms),
        store_single_multipart_file(
            reader,
            &boundary,
            fs,
            &destination,
            head.content_length as u64,
            &ctx.session.csrf_token,
        ),
    )
    .await;

    let response = match stored {
        Err(_) => Response::text(503, "Service Unavailable", b"upload timeout\n"),
        Ok(Err(UploadError::Fs(storage::FsError::FileTooLarge))) => {
            Response::text(413, "Content Too Large", b"upload too large\n")
        }
        Ok(Err(_)) => Response::text(400, "Bad Request", b"invalid multipart upload\n"),
        Ok(Ok(info)) => {
            let uploaded = match build_upload_runtime_value(
                fs,
                upload,
                &destination,
                info,
                ctx.max_image_pixels,
            )
            .await
            {
                Ok(value) => value,
                Err(_) => {
                    let _ = fs.remove(&destination);
                    return DispatchOutcome::close_connection(Response::text(
                        415,
                        "Unsupported Media Type",
                        b"unsupported or invalid image\n",
                    ));
                }
            };
            let system_values = vec![
                (
                    "csrfToken".to_string(),
                    Value::String(ctx.session.csrf_token.clone()),
                ),
                (
                    "authPrincipal".to_string(),
                    Value::String(ctx.session.principal.clone().unwrap_or_default()),
                ),
                (
                    "authMfaVerified".to_string(),
                    Value::Bool(ctx.session.mfa_verified),
                ),
                (
                    "__authRoles".to_string(),
                    Value::List(
                        ctx.session
                            .roles
                            .iter()
                            .cloned()
                            .map(Value::String)
                            .collect(),
                    ),
                ),
                (
                    "__requestId".to_string(),
                    Value::String(ctx.request_id.to_string()),
                ),
                (upload.name.clone(), uploaded),
            ];
            match timeout(
                Duration::from_millis(ctx.config.request_timeout_ms),
                execute_request_with_profiles(
                    ctx.program,
                    HttpMethod::Post,
                    ctx.path,
                    &[],
                    &[],
                    &ExecutionLimits {
                        max_instructions: ctx.config.max_instructions,
                        max_allocated_bytes: ctx.config.max_runtime_alloc_bytes,
                    },
                    ctx.resource_profiles,
                    &system_values,
                    ctx.services.database,
                ),
            )
            .await
            {
                Err(_) => {
                    let _ = fs.remove(&destination);
                    Response::text(503, "Service Unavailable", b"request execution timeout\n")
                }
                Ok(Ok(AppResponse::Html(html))) => {
                    if accepts_media(ctx.request_accept, "text/html") {
                        Response::new(
                            200,
                            "OK",
                            "text/html; charset=utf-8",
                            html.as_str().as_bytes(),
                        )
                    } else {
                        Response::text(406, "Not Acceptable", b"text/html is not acceptable\n")
                    }
                }
                Ok(Ok(AppResponse::Json(json))) => {
                    if accepts_media(ctx.request_accept, "application/json") {
                        Response::new(
                            200,
                            "OK",
                            "application/json; charset=utf-8",
                            json.as_bytes(),
                        )
                    } else {
                        Response::text(
                            406,
                            "Not Acceptable",
                            b"application/json is not acceptable\n",
                        )
                    }
                }
                Ok(Ok(AppResponse::Redirect(redirect))) => {
                    if let Some(flash) = redirect.flash() {
                        if ctx
                            .services
                            .sessions
                            .set_flash(&ctx.session.id, flash.kind.as_str(), &flash.message)
                            .await
                            .is_err()
                        {
                            server_log("{\"event\":\"flash_store_failed\"}");
                        }
                    }
                    Response::redirect(
                        redirect.status().code(),
                        redirect.status().reason(),
                        redirect.location(),
                    )
                }
                Ok(Err(error)) => {
                    let _ = fs.remove(&destination);
                    app_error_response(error, json_api)
                }
            }
        }
    };
    DispatchOutcome::keep_connection(response)
}

pub(super) async fn dispatch_buffered<S>(
    stream: &mut S,
    buffer: &mut Vec<u8>,
    head: ParsedHead,
    ctx: &RequestExecutionContext<'_, '_>,
) -> DispatchOutcome
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    if head.content_length > ctx.config.max_body_bytes {
        return DispatchOutcome::keep_connection(Response::text(
            413,
            "Content Too Large",
            b"request body too large\n",
        ));
    }
    let body = match timeout(
        Duration::from_millis(ctx.config.read_timeout_ms),
        read_buffered_body(stream, buffer, head.content_length),
    )
    .await
    {
        Err(_) => {
            return DispatchOutcome::close_connection(Response::text(
                408,
                "Request Timeout",
                b"request timeout\n",
            ));
        }
        Ok(Ok(body)) => body,
        Ok(Err(error)) => {
            return DispatchOutcome::close_connection(read_error_response(error));
        }
    };
    let request = HttpRequest {
        method: head.method,
        target: head.target,
        headers: head.headers,
        body,
    };
    let response = match timeout(
        Duration::from_millis(ctx.config.request_timeout_ms),
        http_dispatch::dispatch(
            ctx.program,
            &request,
            ctx.config,
            ctx.services.sessions,
            ctx.session,
            ctx.services.database,
            ctx.services.auth_runtime,
            ctx.resource_profiles,
            ctx.services.route_rate_limiter,
            ctx.services.public_cache,
            ctx.services.metrics,
            ctx.request_id,
            ctx.effective_peer,
            ctx.domain_namespace,
            ctx.request_is_https,
            ctx.expected_host,
            ctx.services.web,
        ),
    )
    .await
    {
        Ok(response) => response,
        Err(_) => Response::text(503, "Service Unavailable", b"request execution timeout\n"),
    };
    DispatchOutcome::keep_connection(response)
}
