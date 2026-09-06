use crate::test_support::*;

#[cfg(test)]
mod m35_object_authorization_runtime_tests {
    use super::*;
    use language_core::ObjectAuthorization;

    fn budget() -> Budget {
        let limits = ExecutionLimits {
            max_instructions: 100,
            max_allocated_bytes: 4096,
        };
        Budget::new(
            &limits,
            ResourceProfileConfig {
                max_instructions: 100,
                max_allocated_bytes: 4096,
                max_concurrent: 1,
            },
        )
    }
    fn env(principal: &str, roles: &[&str], owner: &str) -> HashMap<String, Value> {
        let mut env = HashMap::new();
        env.insert("authPrincipal".into(), Value::String(principal.into()));
        env.insert(
            "__authRoles".into(),
            Value::List(roles.iter().map(|r| Value::String((*r).into())).collect()),
        );
        env.insert(
            "article".into(),
            Value::Record(HashMap::from([
                ("id".into(), Value::Int(7)),
                ("authorUsername".into(), Value::String(owner.into())),
            ])),
        );
        env
    }
    fn rule() -> ObjectAuthorization {
        ObjectAuthorization {
            object: "article".into(),
            owner_field: "authorUsername".into(),
            allow_roles: vec!["Publisher".into()],
        }
    }

    #[test]
    fn owner_is_allowed() {
        assert!(authorize_object(&rule(), &env("alice", &[], "alice"), &mut budget()).is_ok());
    }
    #[test]
    fn configured_role_is_allowed() {
        assert!(
            authorize_object(&rule(), &env("bob", &["Publisher"], "alice"), &mut budget()).is_ok()
        );
    }
    #[test]
    fn unrelated_user_is_denied() {
        assert!(matches!(
            authorize_object(&rule(), &env("bob", &["Editor"], "alice"), &mut budget()),
            Err(AppError::Forbidden)
        ));
    }
}

#[cfg(test)]
mod m41_domain_validation_runtime_tests {
    use super::*;
    use crate::db_execution::db_to_value;
    use compiler::compile_source;

    #[test]
    fn email_wire_and_db_values_are_canonical_and_closed() {
        assert!(
            matches!(decode_scalar(&Program::default(), "User.Name+tag@Example.COM", ValueType::Email), Ok(Value::Email(v)) if v=="User.Name+tag@example.com")
        );
        assert!(decode_scalar(&Program::default(), "bad@@example.com", ValueType::Email).is_err());
        assert!(decode_scalar(&Program::default(), ".bad@example.com", ValueType::Email).is_err());
        assert!(decode_scalar(&Program::default(), "bad@-example.com", ValueType::Email).is_err());
        assert!(
            matches!(db_to_value(&Program::default(), &DbValue::String("user@example.com".into()), ValueType::Email), Ok(Value::Email(v)) if v=="user@example.com")
        );
        assert!(matches!(
            db_to_value(
                &Program::default(),
                &DbValue::String("user@Example.com".into()),
                ValueType::Email
            ),
            Err(AppError::Database)
        ));
    }

    #[tokio::test]
    async fn same_validation_is_enforced_after_typed_decode() {
        let src = r#"
form ContactForm {
    email<Email>
    confirmEmail<Email>
    validate confirmEmail same email
}
action fn save(ctx: ActionContext, email: Email, confirmEmail: Email) -> Result<Json, PageError> {
    return Ok(json(email));
}
route save POST "/contact" form ContactForm => save;
"#;
        let p = compile_source(src).unwrap();
        let ok = execute_request(
            &p,
            HttpMethod::Post,
            "/contact",
            &[
                ("email".into(), "a@example.com".into()),
                ("confirmEmail".into(), "a@example.com".into()),
            ],
            None,
        )
        .await;
        assert!(matches!(ok,Ok(AppResponse::Json(ref v)) if v=="\"a@example.com\""));
        let bad = execute_request(
            &p,
            HttpMethod::Post,
            "/contact",
            &[
                ("email".into(), "a@example.com".into()),
                ("confirmEmail".into(), "b@example.com".into()),
            ],
            None,
        )
        .await;
        assert!(matches!(bad, Err(AppError::FormInvalid(_))));
    }
}

#[cfg(test)]
mod m42_domain_validation_runtime_tests {
    use super::*;
    use crate::db_execution::db_to_value;
    use compiler::compile_source;
    use data::{DbConfig, PreparedSql};

    #[test]
    fn url_wire_and_db_values_are_canonical_and_closed() {
        assert!(matches!(
            decode_scalar(&Program::default(), "HTTPS://Example.COM/a", ValueType::Url),
            Ok(Value::Url(v)) if v == "https://example.com/a"
        ));
        assert!(decode_scalar(&Program::default(), "javascript:alert(1)", ValueType::Url).is_err());
        assert!(
            decode_scalar(
                &Program::default(),
                "https://user:pass@example.com/",
                ValueType::Url
            )
            .is_err()
        );
        assert!(matches!(
            db_to_value(
                &Program::default(),
                &DbValue::String("HTTPS://Example.COM/a".into()),
                ValueType::Url
            ),
            Err(AppError::Database)
        ));
    }

    #[tokio::test]
    async fn pattern_validation_runs_after_string_decode() {
        let src = r#"
form CodeForm {
    code<String>
    validate code pattern "^[A-Z]{3}-[0-9]{4}$"
}
action fn save(ctx: ActionContext, code: String) -> Result<Json, PageError> {
    return Ok(json(code));
}
route save POST "/code" form CodeForm => save;
"#;
        let p = compile_source(src).unwrap();
        let ok = execute_request(
            &p,
            HttpMethod::Post,
            "/code",
            &[("code".into(), "ABC-1234".into())],
            None,
        )
        .await;
        assert!(matches!(ok, Ok(AppResponse::Json(ref v)) if v == "\"ABC-1234\""));

        let bad = execute_request(
            &p,
            HttpMethod::Post,
            "/code",
            &[("code".into(), "abc-1234".into())],
            None,
        )
        .await;
        assert!(matches!(bad, Err(AppError::FormInvalid(_))));
    }

    #[tokio::test]
    async fn unique_constraint_violation_becomes_conflict() {
        let src = r#"
query fn createContact(tx: Transaction, email: Email) -> Result<Void, DbError> sql {
    INSERT INTO contacts(email) VALUES (:email)
}
action fn create(ctx: ActionContext, db: Db, email: Email) -> Result<Json, PageError> {
    transaction db { createContact(tx, email)?; }
    return Ok(json(true));
}
route create POST "/contacts" form email<Email> => create;
"#;
        let p = compile_source(src).unwrap();
        let mut cfg = DbConfig::secure_default("sqlite::memory:");
        cfg.max_connections = 1;
        let db = Database::connect(cfg).await.unwrap();
        db.execute(
            &PreparedSql::compile("CREATE TABLE contacts(email TEXT NOT NULL UNIQUE)").unwrap(),
            &BindSet::new(),
        )
        .await
        .unwrap();

        let first = execute_request(
            &p,
            HttpMethod::Post,
            "/contacts",
            &[("email".into(), "a@example.com".into())],
            Some(&db),
        )
        .await;
        assert!(matches!(first, Ok(AppResponse::Json(_))));

        let second = execute_request(
            &p,
            HttpMethod::Post,
            "/contacts",
            &[("email".into(), "a@example.com".into())],
            Some(&db),
        )
        .await;
        assert!(matches!(second, Err(AppError::Conflict)));
    }
}

#[cfg(test)]
mod m43_canonical_url_runtime_tests {
    use super::*;
    use compiler::compile_source;

    #[tokio::test]
    async fn stale_slug_redirects_to_same_typed_route_with_301() {
        let src = r#"
page fn show(ctx: PageContext, slug: Slug, title: String) -> Result<Html, PageError> {
    let canonical = slug(title);
    canonical slug slug from canonical;
    return Ok(html {ok});
}
route article GET "/articles/:slug<Slug>" query title<String> => show;
"#;
        let p = compile_source(src).unwrap();
        let response = execute_request_with_query_context(
            &p,
            HttpMethod::Get,
            "/articles/old-title",
            &[("title".into(), "New Title".into())],
            &[],
            &ExecutionLimits::default(),
            &[],
            None,
        )
        .await
        .unwrap();
        match response {
            AppResponse::Redirect(redirect) => {
                assert_eq!(redirect.status().code(), 301);
                assert_eq!(redirect.location(), "/articles/new-title?title=New%20Title");
            }
            other => panic!("expected permanent redirect, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn canonical_slug_does_not_redirect_when_already_canonical() {
        let src = r#"
page fn show(ctx: PageContext, slug: Slug) -> Result<Html, PageError> {
    canonical slug slug from slug;
    return Ok(html {ok});
}
route article GET "/articles/:slug<Slug>" => show;
"#;
        let p = compile_source(src).unwrap();
        let response = execute_request(&p, HttpMethod::Get, "/articles/current", &[], None)
            .await
            .unwrap();
        assert!(matches!(response, AppResponse::Html(_)));
    }

    #[test]
    fn post_redirects_remain_303() {
        let redirect = Redirect::new("/done".into());
        assert_eq!(redirect.status().code(), 303);
        assert_eq!(redirect.status().reason(), "See Other");
    }
}
