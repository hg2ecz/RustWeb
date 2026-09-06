use crate::test_support::*;

#[cfg(test)]
mod m19_json_runtime_tests {
    use super::*;
    use compiler::compile_source;

    #[tokio::test]
    async fn renders_typed_json_and_decodes_json_route_pairs() {
        let src = r#"
page fn api(ctx: PageContext) -> Result<Json, PageError> {
    let ok = true;
    return Ok(json(ok));
}
action fn echo(ctx: ActionContext, name: String, age: Int, active: Bool) -> Result<Json, PageError> {
    return Ok(json(name));
}
route api GET "/api" => api;
route echo POST "/api/echo" json name<String> age<Int> active<Bool> => echo;
"#;
        let program = compile_source(src).unwrap();
        let get = execute_request_with_query_context(
            &program,
            HttpMethod::Get,
            "/api",
            &[],
            &[],
            &ExecutionLimits::default(),
            &[],
            None,
        )
        .await
        .unwrap();
        assert_eq!(get, AppResponse::Json("true".into()));
        let body = vec![
            ("name".into(), "Alice".into()),
            ("age".into(), "42".into()),
            ("active".into(), "true".into()),
        ];
        let post = execute_request_with_query_context(
            &program,
            HttpMethod::Post,
            "/api/echo",
            &[],
            &body,
            &ExecutionLimits::default(),
            &[],
            None,
        )
        .await
        .unwrap();
        assert_eq!(post, AppResponse::Json("\"Alice\"".into()));
    }
}

#[cfg(test)]
mod m19_json_serialization_tests {
    use super::*;

    #[test]
    fn serializes_records_lists_null_and_escaping() {
        let mut record = HashMap::new();
        record.insert("name".into(), Value::String("<script>\"x\"".into()));
        record.insert("active".into(), Value::Bool(true));
        let value = Value::List(vec![Value::Record(record), Value::Null]);
        let json = serialize_json_value(&value).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed[0]["name"], "<script>\"x\"");
        assert_eq!(parsed[0]["active"], true);
        assert!(parsed[1].is_null());
    }
}

#[cfg(test)]
mod m26_value_type_tests {
    use super::*;

    #[test]
    fn parses_canonical_business_types() {
        assert!(matches!(
            decode_scalar(&Program::default(), "2026-09-04", ValueType::Date),
            Ok(Value::Date(_))
        ));
        assert!(decode_scalar(&Program::default(), "2026-02-30", ValueType::Date).is_err());
        match decode_scalar(
            &Program::default(),
            "2026-09-04T07:12:13+02:00",
            ValueType::DateTime,
        )
        .unwrap()
        {
            Value::DateTime(v) => assert_eq!(
                v.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                "2026-09-04T05:12:13Z"
            ),
            _ => panic!("datetime expected"),
        }
        assert!(matches!(
            decode_scalar(
                &Program::default(),
                "550e8400-e29b-41d4-a716-446655440000",
                ValueType::Uuid
            ),
            Ok(Value::Uuid(_))
        ));
        assert!(decode_scalar(&Program::default(), "not-a-uuid", ValueType::Uuid).is_err());
        match decode_scalar(&Program::default(), "1234567890.1250", ValueType::Decimal).unwrap() {
            Value::Decimal(v) => assert_eq!(v.normalize().to_string(), "1234567890.125"),
            _ => panic!("decimal expected"),
        }
        assert!(decode_scalar(&Program::default(), "NaN", ValueType::Decimal).is_err());
    }

    #[test]
    fn serializes_business_types_as_json_strings() {
        let d = decode_scalar(&Program::default(), "2026-09-04", ValueType::Date).unwrap();
        assert_eq!(serialize_json_value(&d).unwrap(), "\"2026-09-04\"");
        let x = decode_scalar(&Program::default(), "19.9900", ValueType::Decimal).unwrap();
        assert_eq!(serialize_json_value(&x).unwrap(), "\"19.99\"");
    }
}

#[cfg(test)]
mod m36_slug_runtime_tests {
    use super::*;
    use crate::scalars::{is_canonical_slug, slugify_ascii};

    #[test]
    fn validates_canonical_slug() {
        assert!(is_canonical_slug("rust-web-language"));
        assert!(is_canonical_slug("hir-2026"));
        for bad in ["", "Hello", "-x", "x-", "x--y", "../admin", "x/y", "árvíz"] {
            assert!(!is_canonical_slug(bad), "unexpected valid slug: {bad}");
        }
    }

    #[test]
    fn slugify_is_deterministic_for_hungarian_titles() {
        assert_eq!(
            slugify_ascii("Árvíztűrő tükörfúrógép"),
            "arvizturo-tukorfurogep"
        );
        assert_eq!(
            slugify_ascii("  Rust & Web: biztonságosan!  "),
            "rust-web-biztonsagosan"
        );
    }

    #[test]
    fn slug_path_input_is_fail_closed() {
        assert!(
            matches!(decode_scalar(&Program::default(), "rust-web", ValueType::Slug), Ok(Value::String(v)) if v == "rust-web")
        );
        assert!(decode_scalar(&Program::default(), "Rust Web", ValueType::Slug).is_err());
        assert!(decode_scalar(&Program::default(), "../admin", ValueType::Slug).is_err());
    }
}

#[cfg(test)]
mod m38_enum_runtime_tests {
    use super::*;
    use crate::db_execution::db_to_value;
    use language_core::EnumDef;

    fn program() -> Program {
        let mut p = Program::default();
        p.enums.push(EnumDef {
            name: "ArticleStatus".into(),
            variants: vec![
                "Draft".into(),
                "Review".into(),
                "Published".into(),
                "Archived".into(),
            ],
        });
        p
    }

    #[test]
    fn enum_wire_value_is_closed_and_canonical() {
        let p = program();
        assert!(
            matches!(decode_scalar(&p,"Published",ValueType::Enum(0)),Ok(Value::Enum{enum_id:0,variant}) if variant=="Published")
        );
        assert!(decode_scalar(&p, "published", ValueType::Enum(0)).is_err());
        assert!(decode_scalar(&p, "Missing", ValueType::Enum(0)).is_err());
    }

    #[test]
    fn enum_database_value_is_validated() {
        let p = program();
        assert!(
            matches!(db_to_value(&p,&DbValue::String("Draft".into()),ValueType::Enum(0)),Ok(Value::Enum{enum_id:0,variant}) if variant=="Draft")
        );
        assert!(matches!(
            db_to_value(&p, &DbValue::String("Deleted".into()), ValueType::Enum(0)),
            Err(AppError::Database)
        ));
    }
}
