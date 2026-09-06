use crate::test_support::*;

#[cfg(test)]
mod m27_form_runtime_tests {
    use super::*;
    use compiler::compile_source;

    #[tokio::test]
    async fn named_form_returns_field_errors_and_retains_values() {
        let src = r#"
form ProductForm {
    name<String>
    price<Int>
    published<Bool>
    validate name length 2 20 price range 0 1000
}
action fn create(ctx: ActionContext, name: String, price: Int, published: Bool) -> Result<Redirect, PageError> {
    return Ok(redirect("/done"));
}
route create POST "/products" form ProductForm => create;
"#;
        let p = compile_source(src).unwrap();
        let pairs = vec![("name".into(), "A".into()), ("price".into(), "12".into())];
        let err = execute_request_with_query_context(
            &p,
            HttpMethod::Post,
            "/products",
            &[],
            &pairs,
            &ExecutionLimits::default(),
            &[],
            None,
        )
        .await
        .unwrap_err();
        match err {
            AppError::FormInvalid(f) => {
                assert_eq!(f.schema, "ProductForm");
                assert!(f.values.iter().any(|(k, v)| k == "name" && v == "A"));
                assert!(
                    f.values
                        .iter()
                        .any(|(k, v)| k == "published" && v == "false")
                );
                assert!(
                    f.issues
                        .iter()
                        .any(|i| i.field == "name" && i.code == "length")
                );
            }
            other => panic!("expected form invalid, got {other:?}"),
        }
    }
}

#[cfg(test)]
mod m29_component_layout_runtime_tests {
    use super::*;
    use compiler::compile_source;

    #[tokio::test]
    async fn renders_component_and_layout_with_escaped_values() {
        let src = r#"
component fn Badge(text: String) -> Html { html {<strong>{{ text }}</strong>} }
layout fn Main(title: String) -> Html { html {<html><head><title>{{ title }}</title></head><body>@content</body></html>} }
page fn home(ctx: PageContext, name: String) -> Result<Html, PageError> {
    return Ok(html {@layout(Main, "Welcome") {<p>@component(Badge, name)</p>}});
}
route home GET "/" query name<String> => home;
"#;
        let p = compile_source(src).unwrap();
        let q = vec![("name".into(), "<admin>".into())];
        let r = execute_request_with_query_context(
            &p,
            HttpMethod::Get,
            "/",
            &q,
            &[],
            &ExecutionLimits::default(),
            &[],
            None,
        )
        .await
        .unwrap();
        match r {
            AppResponse::Html(h) => {
                assert!(h.as_str().contains("<title>Welcome</title>"));
                assert!(h.as_str().contains("<strong>&lt;admin&gt;</strong>"));
            }
            _ => panic!("expected html"),
        }
    }
}

#[cfg(test)]
mod m32_markdown_runtime_tests {
    use super::*;
    use compiler::compile_source;

    #[tokio::test]
    async fn markdown_renders_allowlisted_markup_and_escapes_raw_html() {
        let src = r#"
page fn article(ctx: PageContext, body: String) -> Result<Html, PageError> {
    return Ok(html {<article>@markdown(body)</article>});
}
route article GET "/" query body<String> => article;
"#;
        let p = compile_source(src).unwrap();
        let body =
            "# Hello\n\n**bold** and [safe](https://example.com)\n\n<script>alert(1)</script>";
        let q = vec![("body".into(), body.into())];
        let r = execute_request_with_query_context(
            &p,
            HttpMethod::Get,
            "/",
            &q,
            &[],
            &ExecutionLimits::default(),
            &[],
            None,
        )
        .await
        .unwrap();
        match r {
            AppResponse::Html(h) => {
                assert!(h.as_str().contains("<h1>Hello</h1>"));
                assert!(h.as_str().contains("<strong>bold</strong>"));
                assert!(h.as_str().contains("href=\"https://example.com\""));
                assert!(!h.as_str().contains("<script>"));
                assert!(h.as_str().contains("&lt;script&gt;"));
            }
            _ => panic!("expected html"),
        }
    }

    #[tokio::test]
    async fn markdown_does_not_activate_unsafe_or_protocol_relative_links() {
        let src = r#"
page fn article(ctx: PageContext, body: String) -> Result<Html, PageError> {
    return Ok(html {@markdown(body)});
}
route article GET "/" query body<String> => article;
"#;
        let p = compile_source(src).unwrap();
        for body in ["[x](javascript:alert(1))", "[x](//evil.example/path)"] {
            let q = vec![("body".into(), body.into())];
            let r = execute_request_with_query_context(
                &p,
                HttpMethod::Get,
                "/",
                &q,
                &[],
                &ExecutionLimits::default(),
                &[],
                None,
            )
            .await
            .unwrap();
            match r {
                AppResponse::Html(h) => assert!(!h.as_str().contains("<a href=")),
                _ => panic!("expected html"),
            }
        }
    }
}

#[cfg(test)]
mod m44_flash_runtime_tests {
    use super::*;
    use compiler::compile_source;

    #[tokio::test]
    async fn redirect_carries_compiler_owned_flash() {
        let src = r#"
action fn save(ctx: ActionContext) -> Result<Redirect, PageError> {
    flash success "Saved";
    return Ok(redirect("/done"));
}
route save POST "/save" => save;
"#;
        let p = compile_source(src).unwrap();
        let response = execute_request(&p, HttpMethod::Post, "/save", &[], None)
            .await
            .unwrap();
        let AppResponse::Redirect(redirect) = response else {
            panic!("expected redirect");
        };
        assert_eq!(redirect.status().code(), 303);
        let flash = redirect.flash().unwrap();
        assert_eq!(flash.kind.as_str(), "success");
        assert_eq!(flash.message, "Saved");
    }

    #[tokio::test]
    async fn flash_directive_escapes_message() {
        let src = r#"
page fn index(ctx: PageContext) -> Result<Html, PageError> {
    return Ok(html {@flash()});
}
route index GET "/" => index;
"#;
        let p = compile_source(src).unwrap();
        let system = vec![
            ("__flashKind".into(), Value::String("success".into())),
            (
                "__flashMessage".into(),
                Value::String("<b>saved</b>".into()),
            ),
        ];
        let response = execute_request_with_query_context(
            &p,
            HttpMethod::Get,
            "/",
            &[],
            &[],
            &ExecutionLimits::default(),
            &system,
            None,
        )
        .await
        .unwrap();
        let AppResponse::Html(html) = response else {
            panic!("expected html");
        };
        assert!(html.as_str().contains("&lt;b&gt;saved&lt;/b&gt;"));
        assert!(!html.as_str().contains("<b>saved</b>"));
    }
}
