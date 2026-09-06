use super::*;

mod m38_enum_compile_tests {
    use super::*;

    #[test]
    fn compiles_enum_across_model_route_query_and_literal() {
        let src = r#"
enum ArticleStatus {
    Draft
    Review
    Published
    Archived
}

model Article {
    id: Int
    status: ArticleStatus
}

query fn setStatus(tx: Transaction, id: Int, status: ArticleStatus) -> Result<Void, DbError> sql {
    UPDATE articles SET status = :status WHERE id = :id
}

action fn publish(ctx: ActionContext, db: Db, id: Int) -> Result<Json, PageError> {
    let status = ArticleStatus.Published;
    transaction db {
        setStatus(tx, id, status)?;
    }
    return Ok(json(status));
}

route publish POST "/article/:id<Int>/publish" auth user => publish;
"#;
        let p = compile_source(src).unwrap();
        let (enum_id, def) = p.enum_by_name("ArticleStatus").unwrap();
        assert_eq!(
            def.variants,
            vec!["Draft", "Review", "Published", "Archived"]
        );
        assert_eq!(
            p.model("Article").unwrap().fields[1].ty,
            ValueType::Enum(enum_id)
        );
        assert_eq!(
            p.query("setStatus").unwrap().params[1].ty,
            ValueType::Enum(enum_id)
        );
    }

    #[test]
    fn rejects_unknown_enum_variant_and_unknown_wire_type() {
        let bad_variant = r#"
enum State { Draft Published }
page fn x(ctx: PageContext) -> Result<Json, PageError> {
    let s = State.Missing;
    return Ok(json(s));
}
route x GET "/" => x;
"#;
        assert!(compile_source(bad_variant).is_err());

        let bad_type = r#"
page fn x(ctx: PageContext, status: MissingState) -> Result<Json, PageError> { return Ok(json(status)); }
route x GET "/:status<MissingState>" => x;
"#;
        assert!(compile_source(bad_type).is_err());
    }
}

#[cfg(test)]
mod m39_optimistic_lock_compile_tests {
    use super::*;

    #[test]
    fn compiles_exactly_one_changed_query() {
        let src = r#"
model Article {
    id: Int
    title: String
    version: Int
}
query fn updateArticle(tx: Transaction, id: Int, title: String, version: Int) -> Result<Changed, DbError> sql {
    UPDATE articles SET title = :title, version = version + 1 WHERE id = :id AND version = :version
}
action fn update(ctx: ActionContext, db: Db, id: Int, title: String, version: Int) -> Result<Redirect, PageError> {
    transaction db {
        updateArticle(tx, id, title, version)?;
    }
    return Ok(redirect("/articles"));
}
route update POST "/articles/:id<Int>" form title<String> version<Int> auth user => update;
"#;
        let p = compile_source(src).unwrap();
        assert!(matches!(
            p.query("updateArticle").unwrap().return_type,
            QueryReturn::Changed
        ));
    }

    #[test]
    fn changed_is_mutation_only_and_has_no_returning() {
        let read = r#"
model Article {
    id: Int
}
query fn bad(db: Db, id: Int) -> Result<Changed, DbError> sql {
    SELECT id FROM articles WHERE id = :id
}
"#;
        assert!(compile_source(read).is_err());

        let returning = r#"
model Article {
    id: Int
}
query fn bad(tx: Transaction, id: Int) -> Result<Changed, DbError> sql {
    UPDATE articles SET id = :id WHERE id = :id RETURNING id
}
"#;
        assert!(compile_source(returning).is_err());
    }
}

#[cfg(test)]
mod m40_business_audit_compile_tests {
    use super::*;

    #[test]
    fn compiles_transactional_business_audit() {
        let src = r#"
enum ArticleStatus {
    Review
    Published
}
model Article {
    id: Int
    status: ArticleStatus
    version: Int
}
query fn publish(tx: Transaction, id: Int, version: Int) -> Result<Changed, DbError> sql {
    UPDATE articles SET status = 'Published', version = version + 1 WHERE id = :id AND version = :version
}
action fn publishAction(ctx: ActionContext, db: Db, id: Int, version: Int) -> Result<Json, PageError> {
    let article = articleById(db, id)?;
    transaction db {
        publish(tx, id, version)?;
        audit Article id action publish from article.status to ArticleStatus.Published;
    }
    return Ok(json(true));
}
query fn articleById(db: Db, id: Int) -> Result<Article, DbError> sql {
    SELECT id, status, version FROM articles WHERE id = :id
}
route publishRoute POST "/articles/:id<Int>/publish" form version<Int> auth role Publisher => publishAction;
"#;
        let p = compile_source(src).unwrap();
        let action = p.action("publishAction").unwrap();
        let ActionBody::Statements(stmts) = &action.body;
        let audit = stmts
            .iter()
            .find_map(|s| match s {
                ActionStatement::Transaction { statements } => {
                    statements.iter().find_map(|t| match t {
                        TxStatement::BusinessAudit(a) => Some(a),
                        _ => None,
                    })
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(audit.object_type, "Article");
        assert_eq!(audit.action, "publish");
        assert_eq!(audit.source_action, "publishAction");
    }

    #[test]
    fn audit_requires_authenticated_route_and_transaction() {
        let public = r#"
model Article {
    id: Int
}
query fn touch(tx: Transaction, id: Int) -> Result<Changed, DbError> sql { UPDATE articles SET id = id WHERE id = :id }
action fn x(ctx: ActionContext, db: Db, id: Int) -> Result<Json, PageError> {
    transaction db {
        touch(tx, id)?;
        audit Article id action touch;
    }
    return Ok(json(true));
}
route x POST "/:id<Int>" => x;
"#;
        assert!(compile_source(public).is_err());

        let outside = r#"
model Article {
    id: Int
}
action fn x(ctx: ActionContext, id: Int) -> Result<Json, PageError> {
    audit Article id action touch;
    return Ok(json(true));
}
route x POST "/:id<Int>" auth user => x;
"#;
        assert!(compile_source(outside).is_err());

        let reserved = r#"
model X {
    id: Int
}
query fn bad(db: Db, id: Int) -> Result<X, DbError> sql { SELECT id FROM _rw_business_audit WHERE id = :id }
"#;
        assert!(compile_source(reserved).is_err());
    }
}

#[cfg(test)]
mod m41_domain_validation_compile_tests {
    use super::*;
    use language_core::ValidationKind;

    #[test]
    fn compiles_email_type_and_cross_field_same_validation() {
        let src = r#"
model Contact {
    id: Int
    email: Email
}
query fn byEmail(db: Db, email: Email) -> Result<Contact, DbError> sql {
    SELECT id, email FROM contacts WHERE email = :email
}
form ContactForm {
    email<Email>
    confirmEmail<Email>
    validate confirmEmail same email
}
action fn save(ctx: ActionContext, email: Email, confirmEmail: Email) -> Result<Json, PageError> {
    return Ok(json(email));
}
route save POST "/contact" form ContactForm auth user => save;
"#;
        let p = compile_source(src).unwrap();
        assert_eq!(p.model("Contact").unwrap().fields[1].ty, ValueType::Email);
        assert_eq!(p.query("byEmail").unwrap().params[0].ty, ValueType::Email);
        assert!(
            matches!(p.form("ContactForm").unwrap().validations[0].kind, ValidationKind::SameAs{ref other} if other=="email")
        );
    }

    #[test]
    fn same_validation_requires_existing_matching_types() {
        let missing = r#"
form X {
    email<Email>
    validate email same missing
}
"#;
        assert!(compile_source(missing).is_err());

        let mismatch = r#"
form X {
    email<Email>
    count<Int>
    validate email same count
}
"#;
        assert!(compile_source(mismatch).is_err());
    }
}

#[cfg(test)]
mod m42_domain_validation_compile_tests {
    use super::*;
    use language_core::ValidationKind;

    #[test]
    fn compiles_url_and_static_pattern_validation() {
        let src = r#"
model Link {
    id: Int
    target: Url
}
form LinkForm {
    target<Url>
    code<String>
    validate code pattern "^[A-Z]{3}-[0-9]{4}$"
}
action fn save(ctx: ActionContext, target: Url, code: String) -> Result<Json, PageError> {
    return Ok(json(target));
}
route save POST "/links" form LinkForm => save;
"#;
        let p = compile_source(src).unwrap();
        assert_eq!(p.model("Link").unwrap().fields[1].ty, ValueType::Url);
        assert!(matches!(
            p.form("LinkForm").unwrap().validations[0].kind,
            ValidationKind::Pattern { ref regex } if regex == "^[A-Z]{3}-[0-9]{4}$"
        ));
    }

    #[test]
    fn pattern_is_string_only_bounded_and_compile_time_validated() {
        let wrong_type = r#"
form X {
    email<Email>
    validate email pattern "example"
}
"#;
        assert!(compile_source(wrong_type).is_err());

        let invalid_regex = r#"
form X {
    value<String>
    validate value pattern "["
}
"#;
        assert!(compile_source(invalid_regex).is_err());
    }
}

#[cfg(test)]
mod m50_projection_alias_compile_tests {
    use super::*;

    #[test]
    fn model_query_accepts_explicit_projection_aliases() {
        let src = r#"
model Article {
    id: Int
    slug: Slug
    title: String
}
query fn resolveBySlug(db: Db, slug: Slug) -> Result<Article, DbError> sql {
    SELECT a.id AS id, a.slug AS slug, a.title AS title
    FROM articles a
    LEFT JOIN article_slug_aliases old ON old.article_id = a.id
    WHERE a.slug = :slug OR old.slug = :slug
}
page fn home(ctx: PageContext) -> Result<Html, PageError> {
    return Ok(html {ok});
}
route home GET "/" => home;
"#;
        let result = compile_source(src);
        assert!(
            result.is_ok(),
            "expected aliased projection to compile, got {result:?}"
        );
    }

    #[test]
    fn model_query_still_rejects_wrong_projection_alias() {
        let src = r#"
model Article {
    id: Int
    slug: Slug
    title: String
}
query fn resolveBySlug(db: Db, slug: Slug) -> Result<Article, DbError> sql {
    SELECT a.id AS id, a.slug AS old_slug, a.title AS title
    FROM articles a
    WHERE a.slug = :slug
}
page fn home(ctx: PageContext) -> Result<Html, PageError> {
    return Ok(html {ok});
}
route home GET "/" => home;
"#;
        let err = compile_source(src).expect_err("wrong projection alias must be rejected");
        assert!(err.to_string().contains("must exactly match model"));
    }
}
