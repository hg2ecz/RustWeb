use super::*;

mod m35_object_authorization_compile_tests {
    use super::*;

    fn source(route_auth: &str) -> String {
        format!(
            r#"
model Article {{
    id: Int
    authorUsername: String
    title: String
}}
query fn loadArticle(db: Db, id: Int) -> Result<Article, DbError> sql {{
    SELECT id, authorUsername, title FROM articles WHERE id = :id
}}
query fn updateTitle(tx: Transaction, id: Int, title: String) -> Result<Void, DbError> sql {{
    UPDATE articles SET title = :title WHERE id = :id
}}
action fn edit(ctx: ActionContext, db: Db, id: Int, title: String) -> Result<Redirect, PageError> {{
    let article = loadArticle(db, id)?;
    authorize article owner authorUsername or role Publisher or role Admin;
    transaction db {{
        updateTitle(tx, id, title)?;
    }}
    return Ok(redirect("/done"));
}}
route edit POST "/articles/:id<Int>" form title<String> {route_auth} => edit;
"#
        )
    }

    #[test]
    fn compiles_owner_or_role_guard_and_action_read_query() {
        let p = compile_source(&source("auth user")).unwrap();
        let action = p.action("edit").unwrap();
        let ActionBody::Statements(stmts) = &action.body;
        assert!(
            stmts
                .iter()
                .any(|s| matches!(s, ActionStatement::LetQuery { .. }))
        );
        let rule = stmts
            .iter()
            .find_map(|s| match s {
                ActionStatement::Authorize(r) => Some(r),
                _ => None,
            })
            .unwrap();
        assert_eq!(rule.object, "article");
        assert_eq!(rule.owner_field, "authorUsername");
        assert_eq!(rule.allow_roles, vec!["Publisher", "Admin"]);
    }

    #[test]
    fn rejects_object_authorization_on_public_route() {
        assert!(compile_source(&source("")).is_err());
    }

    #[test]
    fn rejects_non_string_owner_field() {
        let src = r#"
model Article {
    id: Int
    ownerId: Int
}
query fn loadArticle(db: Db, id: Int) -> Result<Article, DbError> sql {
    SELECT id, ownerId FROM articles WHERE id = :id
}
page fn show(ctx: PageContext, db: Db, id: Int) -> Result<Json, PageError> {
    let article = loadArticle(db, id)?;
    authorize article owner ownerId;
    return Ok(json(article));
}
route show GET "/articles/:id<Int>" auth user => show;
"#;
        assert!(compile_source(src).is_err());
    }
}

#[cfg(test)]
mod m37_domain_object_compile_tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_app() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("rwlang-m37-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn compiles_domain_object_and_namespaced_route_handler() {
        let src = r#"
object Article {
    model {
        id: Int
        slug: Slug
        title: String
        authorUsername: String
    }

    query fn bySlug(db: Db, slug: Slug) -> Result<Article, DbError> sql {
        SELECT id, slug, title, authorUsername FROM articles WHERE slug = :slug
    }

    page fn show(ctx: PageContext, db: Db, slug: Slug) -> Result<Html, PageError> {
        let article = Article.bySlug(db, slug)?;
        return Ok(html {<h1>{{ article.title }}</h1>});
    }
}
route articleShow GET "/cikk/:slug<Slug>" => Article.show;
"#;
        let p = compile_source(src).unwrap();
        assert!(p.model("Article").is_some());
        assert!(p.query("Article__bySlug").is_some());
        assert!(p.page("Article__show").is_some());
        assert_eq!(p.routes[0].handler, "Article__show");
    }

    #[test]
    fn domain_object_members_work_across_modules() {
        let dir = temp_app();
        fs::write(dir.join("main.rw"), "mod article;\nmod routes;\n").unwrap();
        fs::write(
            dir.join("article.rw"),
            r#"
object Article {
    model {
        id: Int
        slug: Slug
        title: String
    }
    query fn bySlug(db: Db, slug: Slug) -> Result<Article, DbError> sql {
        SELECT id, slug, title FROM articles WHERE slug = :slug
    }
    page fn show(ctx: PageContext, db: Db, slug: Slug) -> Result<Json, PageError> {
        let article = Article.bySlug(db, slug)?;
        return Ok(json(article));
    }
}
"#,
        )
        .unwrap();
        fs::write(
            dir.join("routes.rw"),
            r#"
route articleShow GET "/cikk/:slug<Slug>" => article::Article.show;
"#,
        )
        .unwrap();
        let p = compile_file(dir.join("main.rw")).unwrap();
        assert!(p.page("article::Article__show").is_some());
        assert_eq!(p.routes[0].handler, "article::Article__show");
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn same_domain_object_name_isolated_by_module_namespace() {
        let dir = temp_app();
        fs::write(
            dir.join("main.rw"),
            r#"mod a;
mod b;
page fn home(ctx: PageContext) -> Result<Html, PageError> { return Ok(html {ok}); }
route home GET "/" => home;
"#,
        )
        .unwrap();
        let obj = r#"
object Article {
    model {
        id: Int
    }
}
"#;
        fs::write(dir.join("a.rw"), obj).unwrap();
        fs::write(dir.join("b.rw"), obj).unwrap();
        let p = compile_file(dir.join("main.rw")).unwrap();
        assert!(p.model("a::Article").is_some());
        assert!(p.model("b::Article").is_some());
        assert!(p.model("Article").is_none());
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn rejects_nested_domain_objects() {
        let src = r#"
object Article {
    model {
        id: Int
    }
    object Hidden {
        model {
            id: Int
        }
    }
}
route x GET "/" => missing;
"#;
        assert!(compile_source(src).is_err());
    }
}
