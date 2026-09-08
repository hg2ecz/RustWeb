use super::*;

mod m29_component_layout_compile_tests {
    use super::*;

    #[test]
    fn compiles_typed_component_and_layout() {
        let src = r#"
model Article {
    id: Int
    title: String
}
component fn ArticleCard(article: Article) -> Html {
    html {<article><h2>{{ article.title }}</h2></article>}
}
layout fn Main(title: String) -> Html {
    html {<html><head><title>{{ title }}</title></head><body><main>@content</main></body></html>}
}
query fn loadArticle(db: Db, id: Int) -> Result<Article, DbError> sql {
    SELECT id, title FROM articles WHERE id = :id
}
page fn article(ctx: PageContext, db: Db, id: Int) -> Result<Html, PageError> {
    let article = loadArticle(db, id)?;
    return Ok(html {
        @layout(Main, "Article") {
            @component(ArticleCard, article)
        }
    });
}
route article GET "/articles/:id<Int>" public => article;
"#;
        let p = compile_source(src).unwrap();
        assert_eq!(p.components.len(), 1);
        assert_eq!(p.layouts.len(), 1);
    }

    #[test]
    fn component_accepts_validated_request_string() {
        let src = r#"
component fn Badge(text: String) -> Html { html {<strong>{{ text }}</strong>} }
page fn home(ctx: PageContext, name: String) -> Result<Html, PageError> {
    return Ok(html {@component(Badge, name)});
}
route home GET "/" query name<String> public => home;
"#;
        compile_source(src).expect("validated request strings should be valid presentation arguments");
    }

    #[test]
    fn component_cannot_launder_secret_scalar() {
        let src = r#"
model Credential {
    id: Int
    token: Secret<String>
}
component fn Badge(text: String) -> Html { html {<strong>{{ text }}</strong>} }
query fn load(db: Db, id: Int) -> Result<Credential, DbError> sql {
    SELECT id, token FROM credentials WHERE id = :id
}
page fn home(ctx: PageContext, db: Db, id: Int) -> Result<Html, PageError> {
    let credential = load(db, id)?;
    return Ok(html {@component(Badge, credential.token)});
}
route home GET "/" query id<Int> public => home;
"#;
        let err = compile_source(src).expect_err("template calls must not declassify secrets");
        assert!(err.to_string().contains("SEC-DATA-002"));
    }

    #[test]
    fn rejects_layout_without_exactly_one_content_slot() {
        let src = r#"
layout fn Bad(title: String) -> Html { html {<main>{{ title }}</main>} }
page fn home(ctx: PageContext) -> Result<Html, PageError> { return Ok(html {x}); }
route home GET "/" public => home;
"#;
        assert!(compile_source(src).is_err());
    }

    #[test]
    fn rejects_component_inside_attribute_context() {
        let src = r#"
component fn Badge(text: String) -> Html { html {<b>{{ text }}</b>} }
page fn home(ctx: PageContext) -> Result<Html, PageError> {
    return Ok(html {<div class="@component(Badge, "x")">x</div>});
}
route home GET "/" public => home;
"#;
        assert!(matches!(
            compile_source(src),
            Err(CompileError::UnsafeHtml(_))
        ));
    }

    #[test]
    fn rejects_template_cycles() {
        let src = r#"
component fn A(x: String) -> Html { html {@component(B, x)} }
component fn B(x: String) -> Html { html {@component(A, x)} }
page fn home(ctx: PageContext) -> Result<Html, PageError> { return Ok(html {x}); }
route home GET "/" public => home;
"#;
        assert!(compile_source(src).is_err());
    }
}

#[cfg(test)]
mod m32_markdown_compiler_tests {
    use super::*;

    #[test]
    fn accepts_markdown_string_in_content_position() {
        let src = r#"
page fn article(ctx: PageContext, body: String) -> Result<Html, PageError> {
    return Ok(html {<article>@markdown(body)</article>});
}
route article GET "/" query body<String> public => article;
"#;
        let p = compile_source(src).unwrap();
        assert_eq!(p.routes.len(), 1);
    }

    #[test]
    fn rejects_markdown_non_string() {
        let src = r#"
page fn article(ctx: PageContext, id: Int) -> Result<Html, PageError> {
    return Ok(html {<article>@markdown(id)</article>});
}
route article GET "/" query id<Int> public => article;
"#;
        assert!(compile_source(src).is_err());
    }

    #[test]
    fn rejects_markdown_inside_attribute() {
        let src = r#"
page fn article(ctx: PageContext, body: String) -> Result<Html, PageError> {
    return Ok(html {<div class="@markdown(body)">x</div>});
}
route article GET "/" query body<String> public => article;
"#;
        assert!(matches!(
            compile_source(src),
            Err(CompileError::UnsafeHtml(_))
        ));
    }
}

#[cfg(test)]
mod m33_image_compiler_tests {
    use super::*;
    #[test]
    fn accepts_typed_image_upload_and_renderer() {
        let src = r#"
action fn save(ctx: ActionContext, hero: Image) -> Result<Json, PageError> { return Ok(json(hero)); }
route save POST "/save" upload hero<Image> to "media" publish auth user => save;
page fn show(ctx: PageContext, hero: Image) -> Result<Html, PageError> { return Ok(html {<figure>@image(hero, "Hero image")</figure>}); }
route show GET "/show" query hero<Image> public => show;
"#;
        let p = compile_source(src).unwrap();
        assert!(
            p.routes
                .iter()
                .find(|r| r.name == "save")
                .unwrap()
                .upload
                .as_ref()
                .unwrap()
                .image
        );
    }
    #[test]
    fn rejects_image_direct_interpolation_and_attribute_context() {
        let direct = r#"
page fn show(ctx: PageContext, hero: Image) -> Result<Html, PageError> { return Ok(html {<div>{{ hero }}</div>}); }
route show GET "/" query hero<Image> public => show;
"#;
        assert!(compile_source(direct).is_err());
        let attr = r#"
page fn show(ctx: PageContext, hero: Image) -> Result<Html, PageError> { return Ok(html {<div class="@image(hero, \"x\")">x</div>}); }
route show GET "/" query hero<Image> public => show;
"#;
        assert!(compile_source(attr).is_err());
    }
    #[test]
    fn image_destination_must_be_url_safe() {
        let src = r#"
action fn save(ctx: ActionContext, hero: Image) -> Result<Json, PageError> { return Ok(json(hero)); }
route save POST "/save" upload hero<Image> to "media files" publish auth user => save;
"#;
        assert!(compile_source(src).is_err());
    }
    #[test]
    fn image_upload_requires_explicit_publish_transition() {
        let src = r#"
action fn save(ctx: ActionContext, hero: Image) -> Result<Json, PageError> { return Ok(json(hero)); }
route save POST "/save" upload hero<Image> to "media" auth user => save;
"#;
        let err = compile_source(src).expect_err("Image must not become public implicitly");
        assert!(err.to_string().contains("SEC-FILE-001"));
    }

    #[test]
    fn raw_upload_cannot_be_published() {
        let src = r#"
action fn save(ctx: ActionContext, file: Upload) -> Result<Json, PageError> { return Ok(json(true)); }
route save POST "/save" upload file<Upload> to "private" publish auth user => save;
"#;
        let err = compile_source(src).expect_err("raw Upload must remain private");
        assert!(err.to_string().contains("SEC-FILE-002"));
    }

}
