use crate::test_support::*;
use compiler::compile_source;

#[tokio::test]
async fn exhaustive_sum_match_dispatches_selected_variant() {
    let source = r#"
enum Outcome { Success Conflict }
page fn result(ctx: PageContext, outcome: Outcome) -> Result<Json, PageError> {
    match outcome {
        Success => { return Ok(json("ok")); }
        Conflict => { fail conflict; }
    }
}
route result GET "/:outcome<Outcome>" public => result;
"#;
    let program = compile_source(source).unwrap();
    let limits = ExecutionLimits::default();
    let ok = execute_request_with_query_context(
        &program,
        HttpMethod::Get,
        "/Success",
        &[],
        &[],
        &limits,
        &[],
        None,
    )
    .await
    .unwrap();
    assert!(matches!(ok, AppResponse::Json(ref value) if value == "\"ok\""));

    let conflict = execute_request_with_query_context(
        &program,
        HttpMethod::Get,
        "/Conflict",
        &[],
        &[],
        &limits,
        &[],
        None,
    )
    .await;
    assert_eq!(conflict, Err(AppError::Conflict));
}
