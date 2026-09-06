use auth::{AuthError, SessionBackend, SessionSnapshot, authenticate_ldap, verify_totp_redis};
use language_core::ServerConfig;
use observability::{ActivityEvent, audit_log, json_line, utc_timestamp};
use runtime::decode_urlencoded_limited;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use super::auth_setup::canonical_username;
use super::http_io::{HttpRequest, Response};
use super::{AuthRuntime, WebSecurityCliConfig};

fn audit_auth_activity(request_id: &str, actor: &str, outcome: &str, client_ip: &str) {
    if let Ok(line) = json_line(&ActivityEvent {
        schema_version: 1,
        timestamp: utc_timestamp(),
        event: "user_activity",
        request_id,
        actor,
        action: "login",
        target: "authentication",
        outcome,
        client_ip,
    }) {
        audit_log(&line);
    }
}

pub(super) async fn auth_login(
    request: &HttpRequest,
    config: &ServerConfig,
    sessions: &SessionBackend,
    session: &SessionSnapshot,
    auth: &AuthRuntime,
    request_id: &str,
    peer_key: &str,
    web: &WebSecurityCliConfig,
) -> Response {
    if request.method == "GET" {
        let body = format!(
            "<!doctype html><html><body><h1>Login</h1><form method=\"post\" action=\"/__rw/auth/login\"><input type=\"hidden\" name=\"_csrf\" value=\"{}\"><label>User <input name=\"username\" autocomplete=\"username\"></label><label>Password <input type=\"password\" name=\"password\" autocomplete=\"current-password\"></label><label>Second factor <input name=\"totp\" autocomplete=\"one-time-code\"></label><button>Login</button></form></body></html>",
            session.csrf_token
        );
        return Response::new(200, "OK", "text/html; charset=utf-8", body.as_bytes());
    }
    if request.method != "POST" {
        return Response::text(405, "Method Not Allowed", b"method not allowed\n");
    }
    if auth.ldap.is_none() && auth.local.is_none() {
        return Response::text(503, "Service Unavailable", b"authentication unavailable\n");
    }
    if !request.header("content-type").is_some_and(|v| {
        v.split(';')
            .next()
            .unwrap_or("")
            .trim()
            .eq_ignore_ascii_case("application/x-www-form-urlencoded")
    }) {
        return Response::text(415, "Unsupported Media Type", b"expected form body\n");
    }
    let pairs = match decode_urlencoded_limited(&request.body, 8, 4096) {
        Ok(v) => v,
        Err(_) => return Response::text(400, "Bad Request", b"bad form body\n"),
    };
    let mut map = HashMap::new();
    for (k, v) in pairs {
        if map.insert(k, v).is_some() {
            return Response::text(400, "Bad Request", b"duplicate form field\n");
        }
    }
    if map.len() != 4
        || !map.contains_key("_csrf")
        || !map.contains_key("username")
        || !map.contains_key("password")
        || !map.contains_key("totp")
    {
        return Response::text(400, "Bad Request", b"invalid login form\n");
    }
    if !matches!(
        sessions
            .verify_csrf(&session.id, map.get("_csrf").unwrap())
            .await,
        Ok(true)
    ) {
        return Response::text(403, "Forbidden", b"CSRF validation failed\n");
    }
    let username = match canonical_username(map.get("username").unwrap()) {
        Some(v) => v,
        None => return Response::text(400, "Bad Request", b"invalid login form\n"),
    };
    let password = map.get("password").unwrap();
    let code = map.get("totp").unwrap();
    if password.len() > 4096 || code.len() > 64 {
        return Response::text(400, "Bad Request", b"invalid login form\n");
    }
    let rate_key = format!("{}:{}", peer_key, username);
    match auth.limiter.hit(&rate_key).await {
        Ok(()) => {}
        Err(AuthError::RateLimited) => {
            audit_auth_activity(request_id, &username, "rate_limited", peer_key);
            return Response::text(
                429,
                "Too Many Requests",
                b"too many authentication attempts\n",
            );
        }
        Err(_) => {
            return Response::text(503, "Service Unavailable", b"authentication unavailable\n");
        }
    }
    let (canonical_principal, roles, secret, auth_generation, local_backend) = if let Some(local) =
        auth.local.as_ref()
    {
        match local.authenticate(&username, password).await {
            Ok(user) => (
                user.username,
                user.roles,
                user.totp_secret,
                user.auth_generation,
                true,
            ),
            Err(AuthError::StoreUnavailable) => {
                return Response::text(503, "Service Unavailable", b"authentication unavailable\n");
            }
            Err(_) => {
                audit_auth_activity(request_id, &username, "invalid_credentials", peer_key);
                return Response::text(401, "Unauthorized", b"invalid credentials\n");
            }
        }
    } else {
        let ldap = auth.ldap.as_ref().expect("checked above");
        if authenticate_ldap(ldap, &username, password).await.is_err() {
            audit_auth_activity(request_id, &username, "invalid_credentials", peer_key);
            return Response::text(401, "Unauthorized", b"invalid credentials\n");
        }
        let roles = auth
            .roles
            .get(&username)
            .cloned()
            .unwrap_or_else(|| vec!["User".into()]);
        (
            username.clone(),
            roles,
            auth.totp_secrets.get(&username).cloned(),
            0,
            false,
        )
    };
    let mfa = if let Some(secret) = secret.as_deref() {
        let unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|v| v.as_secs())
            .unwrap_or(0);
        let totp_ok = code.len() == 6
            && code.bytes().all(|b| b.is_ascii_digit())
            && if let Some(redis) = &auth.redis {
                verify_totp_redis(redis, &canonical_principal, secret, unix, code)
                    .await
                    .is_ok()
            } else {
                auth.local_totp
                    .verify(&canonical_principal, secret, unix, code)
                    .is_ok()
            };
        let recovery_ok = if !totp_ok && local_backend {
            match auth
                .local
                .as_ref()
                .expect("local backend")
                .consume_recovery_code(&canonical_principal, code)
                .await
            {
                Ok(v) => v,
                Err(_) => {
                    return Response::text(
                        503,
                        "Service Unavailable",
                        b"authentication unavailable\n",
                    );
                }
            }
        } else {
            false
        };
        if !totp_ok && !recovery_ok {
            audit_auth_activity(
                request_id,
                &canonical_principal,
                "invalid_second_factor",
                peer_key,
            );
            return Response::text(401, "Unauthorized", b"invalid credentials\n");
        }
        true
    } else if auth.require_totp {
        audit_auth_activity(
            request_id,
            &canonical_principal,
            "second_factor_required",
            peer_key,
        );
        return Response::text(401, "Unauthorized", b"invalid credentials\n");
    } else {
        false
    };
    let _ = auth.limiter.clear(&rate_key).await;
    let rotated = match sessions
        .rotate_authenticated(
            &session.id,
            canonical_principal.clone(),
            mfa,
            roles,
            auth_generation,
        )
        .await
    {
        Ok(v) => v,
        Err(_) => {
            return Response::text(503, "Service Unavailable", b"authentication unavailable\n");
        }
    };
    let mut response = Response::redirect(303, "See Other", "/");
    response.headers.push((
        "Set-Cookie".into(),
        session_cookie(config, &rotated.id, web.cors_allow_credentials),
    ));
    audit_auth_activity(request_id, &canonical_principal, "success", peer_key);
    response
}

pub(super) async fn auth_logout(
    request: &HttpRequest,
    config: &ServerConfig,
    sessions: &SessionBackend,
    session: &SessionSnapshot,
    request_id: &str,
    peer_key: &str,
    web: &WebSecurityCliConfig,
) -> Response {
    if request.method != "POST" {
        return Response::text(405, "Method Not Allowed", b"method not allowed\n");
    };
    let pairs = match decode_urlencoded_limited(&request.body, 4, 4096) {
        Ok(v) => v,
        Err(_) => return Response::text(400, "Bad Request", b"bad form body\n"),
    };
    let csrf = pairs
        .iter()
        .filter(|(k, _)| k == "_csrf")
        .collect::<Vec<_>>();
    if csrf.len() != 1
        || !matches!(
            sessions.verify_csrf(&session.id, &csrf[0].1).await,
            Ok(true)
        )
    {
        return Response::text(403, "Forbidden", b"CSRF validation failed\n");
    };
    if sessions.invalidate(&session.id).await.is_err() {
        return Response::text(503, "Service Unavailable", b"authentication unavailable\n");
    };
    let fresh = match sessions.create().await {
        Ok(v) => v,
        Err(_) => {
            return Response::text(503, "Service Unavailable", b"authentication unavailable\n");
        }
    };
    let mut response = Response::redirect(303, "See Other", "/");
    response.headers.push((
        "Set-Cookie".into(),
        session_cookie(config, &fresh.id, web.cors_allow_credentials),
    ));
    let actor = session.principal.as_deref().unwrap_or("anonymous");
    if let Ok(line) = json_line(&ActivityEvent {
        schema_version: 1,
        timestamp: utc_timestamp(),
        event: "user_activity",
        request_id,
        actor,
        action: "logout",
        target: "authentication",
        outcome: "success",
        client_ip: peer_key,
    }) {
        audit_log(&line);
    }
    response
}

pub(super) fn session_cookie_name(config: &ServerConfig) -> &'static str {
    if config.insecure_dev_cookies {
        "rw_session"
    } else {
        "__Host-rw_session"
    }
}

pub(super) fn session_cookie(config: &ServerConfig, id: &str, cors_credentials: bool) -> String {
    let secure = if config.insecure_dev_cookies {
        ""
    } else {
        "; Secure"
    };
    let same_site = if cors_credentials { "None" } else { "Lax" };
    format!(
        "{}={}; Path=/; HttpOnly; SameSite={}{}; Max-Age={}",
        session_cookie_name(config),
        id,
        same_site,
        secure,
        config.session_ttl_secs
    )
}

pub(super) fn parse_cookie<'a>(header: &'a str, wanted: &str) -> Option<&'a str> {
    let mut found = None;
    for pair in header.split(';') {
        let (name, value) = pair.trim().split_once('=')?;
        if name == wanted {
            if found.is_some() || value.is_empty() {
                return None;
            }
            found = Some(value);
        }
    }
    found
}
