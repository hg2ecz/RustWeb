use crate::test_support::*;

use compiler::compile_source;

    const APP: &str = r#"
page fn tags(ctx: PageContext, tags: List<String>) -> Result<Json, PageError> {
    return Ok(json(len(tags)));
}
route tags GET "/tags" query tags<List<String>> validate tags items 1 3 public => tags;
"#;

    #[tokio::test]
    async fn repeated_query_values_form_bounded_string_list() {
        let program = compile_source(APP).unwrap();
        let limits = ExecutionLimits::default();
        let ok = execute_request_with_query_context(
            &program,
            HttpMethod::Get,
            "/tags",
            &[("tags".into(), "rust".into()), ("tags".into(), "web".into())],
            &[],
            &limits,
            &[],
            None,
        )
        .await
        .unwrap();
        match ok {
            AppResponse::Json(body) => assert_eq!(body, "2"),
            _ => panic!("expected json"),
        }
        let too_many = execute_request_with_query_context(
            &program,
            HttpMethod::Get,
            "/tags",
            &[
                ("tags".into(), "a".into()),
                ("tags".into(), "b".into()),
                ("tags".into(), "c".into()),
                ("tags".into(), "d".into()),
            ],
            &[],
            &limits,
            &[],
            None,
        )
        .await;
        assert_eq!(too_many, Err(AppError::BadRequest));
    }
