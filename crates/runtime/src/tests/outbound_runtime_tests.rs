use crate::test_support::*;
use compiler::compile_source;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct FakeOutbound {
    calls: Arc<Mutex<Vec<String>>>,
}

impl OutboundRuntime for FakeOutbound {
    fn get_status<'a>(&'a self, target: &'a str, path: &'a str) -> OutboundFuture<'a> {
        let calls = Arc::clone(&self.calls);
        Box::pin(async move {
            calls.lock().unwrap().push(format!("GET {target} {path}"));
            Ok(OutboundOutcome {
                status: 204,
                transferred_bytes: 128,
            })
        })
    }

    fn post_json_status<'a>(
        &'a self,
        target: &'a str,
        path: &'a str,
        body: &'a [u8],
    ) -> OutboundFuture<'a> {
        let calls = Arc::clone(&self.calls);
        Box::pin(async move {
            calls.lock().unwrap().push(format!(
                "POST {target} {path} {}",
                std::str::from_utf8(body).unwrap()
            ));
            Ok(OutboundOutcome {
                status: 202,
                transferred_bytes: 256,
            })
        })
    }
}

#[tokio::test]
async fn page_outbound_get_executes_only_through_runtime_capability() {
    let source = r#"
integration Catalog { egress catalog_api }
page fn show(ctx: PageContext) -> Result<Json, PageError> uses Catalog {
    let status = Catalog.get("/v1/items")?;
    return Ok(json(status));
}
route show GET "/items" public => show;
"#;
    let program = compile_source(source).unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let outbound = FakeOutbound {
        calls: Arc::clone(&calls),
    };
    let limits = ExecutionLimits::default();
    let profiles = ResourceProfiles::default_for_limits(&limits);
    let response = execute_request_with_profiles_and_outbound(
        &program,
        HttpMethod::Get,
        "/items",
        &[],
        &[],
        &limits,
        &profiles,
        &[],
        None,
        Some(&outbound),
    )
    .await
    .unwrap();
    assert_eq!(response, AppResponse::Json("204".into()));
    assert_eq!(
        calls.lock().unwrap().as_slice(),
        ["GET catalog_api /v1/items"]
    );
}

#[tokio::test]
async fn action_outbound_post_json_serializes_bounded_public_body() {
    let source = r#"
integration Billing { egress payments }
action fn charge(ctx: ActionContext, amount: Int) -> Result<Json, PageError> uses Billing {
    let status = Billing.postJson("/v1/charges", amount)?;
    return Ok(json(status));
}
route charge POST "/charge" form amount<Int> public => charge;
"#;
    let program = compile_source(source).unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let outbound = FakeOutbound {
        calls: Arc::clone(&calls),
    };
    let limits = ExecutionLimits::default();
    let profiles = ResourceProfiles::default_for_limits(&limits);
    let response = execute_request_with_profiles_and_outbound(
        &program,
        HttpMethod::Post,
        "/charge",
        &[],
        &[("amount".into(), "7".into())],
        &limits,
        &profiles,
        &[],
        None,
        Some(&outbound),
    )
    .await
    .unwrap();
    assert_eq!(response, AppResponse::Json("202".into()));
    assert_eq!(
        calls.lock().unwrap().as_slice(),
        ["POST payments /v1/charges 7"]
    );
}

#[tokio::test]
async fn missing_outbound_runtime_fails_closed() {
    let source = r#"
integration Catalog { egress catalog_api }
page fn show(ctx: PageContext) -> Result<Json, PageError> uses Catalog {
    let status = Catalog.get("/v1/items")?;
    return Ok(json(status));
}
route show GET "/items" public => show;
"#;
    let program = compile_source(source).unwrap();
    let result = execute_request(&program, HttpMethod::Get, "/items", &[], None).await;
    assert_eq!(result, Err(AppError::Internal));
}

#[tokio::test]
async fn cumulative_outbound_io_budget_fails_closed() {
    #[derive(Clone)]
    struct HugeOutbound;
    impl OutboundRuntime for HugeOutbound {
        fn get_status<'a>(&'a self, _target: &'a str, _path: &'a str) -> OutboundFuture<'a> {
            Box::pin(async move {
                Ok(OutboundOutcome {
                    status: 204,
                    transferred_bytes: 100 * 1024 * 1024,
                })
            })
        }
        fn post_json_status<'a>(
            &'a self,
            _target: &'a str,
            _path: &'a str,
            _body: &'a [u8],
        ) -> OutboundFuture<'a> {
            Box::pin(async move {
                Ok(OutboundOutcome {
                    status: 204,
                    transferred_bytes: 100 * 1024 * 1024,
                })
            })
        }
    }

    let source = r#"
integration Catalog { egress catalog_api }
page fn show(ctx: PageContext) -> Result<Json, PageError> uses Catalog {
    let status = Catalog.get("/v1/items")?;
    return Ok(json(status));
}
route show GET "/items" public => show;
"#;
    let program = compile_source(source).unwrap();
    let limits = ExecutionLimits::default();
    let profiles = ResourceProfiles::default_for_limits(&limits);
    let result = execute_request_with_profiles_and_outbound(
        &program,
        HttpMethod::Get,
        "/items",
        &[],
        &[],
        &limits,
        &profiles,
        &[],
        None,
        Some(&HugeOutbound),
    )
    .await;
    assert_eq!(result, Err(AppError::ExternalIoLimit));
}
