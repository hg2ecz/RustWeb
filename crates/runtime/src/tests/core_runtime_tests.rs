use crate::test_support::*;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn urlencoded_limits() {
        assert!(decode_urlencoded_limited(b"a=1&b=2", 1, 10).is_err());
    }
    #[test]
    fn html_escape() {
        let mut s = String::new();
        escape_html_into("<script>", &mut s);
        assert_eq!(s, "&lt;script&gt;");
    }
}

#[cfg(test)]
mod m10_tests {
    use super::*;
    use compiler::compile_source;
    use data::{BindSet, DbConfig, PreparedSql};

    const APP: &str = r#"
model Product {
    id: Int
    name: String
    price: Int
}
query fn listProducts(db: Db, limit: Int, offset: Int) -> Result<List<Product>, DbError> sql {
    SELECT id, name, price FROM products ORDER BY id LIMIT :limit OFFSET :offset
}
page fn products(ctx: PageContext, db: Db, page: Int, pageSize: Int) -> Result<Html, PageError> {
    let offset = (page - 1) * pageSize;
    let products = listProducts(db, pageSize, offset)?;
    return Ok(html {<ul>@for product in products {<li><a @href(product, product.id)>{{ product.name }}</a></li>}</ul><a @href(products, page + 1, pageSize)>next</a>});
}
page fn product(ctx: PageContext, id: Int) -> Result<Html, PageError> { return Ok(html {product}); }
route products GET "/products" query page<Int> pageSize<Int> validate page range 1 100 pageSize range 1 2 => products;
route product GET "/products/:id<Int>" => product;
"#;

    #[tokio::test]
    async fn pagination_and_typed_urls_are_enforced() {
        let program = compile_source(APP).unwrap();
        let mut cfg = DbConfig::secure_default("sqlite::memory:");
        cfg.max_connections = 1;
        let db = Database::connect(cfg).await.unwrap();
        db.execute(&PreparedSql::compile("CREATE TABLE products(id INTEGER PRIMARY KEY, name TEXT NOT NULL, price INTEGER NOT NULL)").unwrap(),&BindSet::new()).await.unwrap();
        db.execute(
            &PreparedSql::compile(
                "INSERT INTO products(id,name,price) VALUES (1,'A&B',10),(2,'B',20),(3,'C',30)",
            )
            .unwrap(),
            &BindSet::new(),
        )
        .await
        .unwrap();
        let response = execute_request_with_query_context(
            &program,
            HttpMethod::Get,
            "/products",
            &[("page".into(), "1".into()), ("pageSize".into(), "2".into())],
            &[],
            &ExecutionLimits::default(),
            &[],
            Some(&db),
        )
        .await
        .unwrap();
        match response {
            AppResponse::Html(h) => {
                assert!(h.as_str().contains("href=\"/products/1\""));
                assert!(h.as_str().contains("A&amp;B"));
                assert!(
                    h.as_str()
                        .contains("href=\"/products?page=2&amp;pageSize=2\"")
                        || h.as_str().contains("href=\"/products?page=2&pageSize=2\"")
                );
                assert!(!h.as_str().contains(">C<"));
            }
            _ => panic!("expected html"),
        }
        assert_eq!(
            execute_request_with_query_context(
                &program,
                HttpMethod::Get,
                "/products",
                &[("page".into(), "0".into()), ("pageSize".into(), "2".into())],
                &[],
                &ExecutionLimits::default(),
                &[],
                Some(&db)
            )
            .await,
            Err(AppError::BadRequest)
        );
        assert_eq!(
            execute_request_with_query_context(
                &program,
                HttpMethod::Get,
                "/products",
                &[
                    ("page".into(), "1".into()),
                    ("pageSize".into(), "2".into()),
                    ("extra".into(), "1".into())
                ],
                &[],
                &ExecutionLimits::default(),
                &[],
                Some(&db)
            )
            .await,
            Err(AppError::BadRequest)
        );
    }
}

#[cfg(test)]
mod resource_budget_tests {
    use super::*;
    use compiler::compile_source;

    #[tokio::test]
    async fn allocation_budget_stops_large_runtime_value() {
        let source = r#"
page fn home(ctx: PageContext) -> Result<Html, PageError> {
    let x = "abcdefghijklmnopqrstuvwxyz";
    return Ok(html {<p>{{ x }}</p>});
}
route home GET "/" => home;
"#;
        let program = compile_source(source).unwrap();
        let limits = ExecutionLimits {
            max_instructions: 1000,
            max_allocated_bytes: 8,
        };
        let result = execute_request_with_query_context(
            &program,
            HttpMethod::Get,
            "/",
            &[],
            &[],
            &limits,
            &[],
            None,
        )
        .await;
        assert_eq!(result, Err(AppError::MemoryLimit));
    }
}

#[cfg(test)]
mod named_resource_profile_tests {
    use super::*;
    use compiler::compile_source;

    #[tokio::test]
    async fn named_profile_can_raise_scope_budget_but_not_request_ceiling() {
        let src = r#"
page fn home(ctx: PageContext) -> Result<Html, PageError> {
    with resource compute {
        let x = 40 + 2;
        return Ok(html {<p>{{ x }}</p>});
    }
}
route home GET "/" => home;
"#;
        let program = compile_source(src).unwrap();
        let request = ExecutionLimits {
            max_instructions: 100,
            max_allocated_bytes: 1024,
        };
        let mut named = HashMap::new();
        named.insert(
            "compute".into(),
            ResourceProfileConfig {
                max_instructions: 50,
                max_allocated_bytes: 1024,
                max_concurrent: 1,
            },
        );
        let profiles = ResourceProfiles::new(
            ResourceProfileConfig {
                max_instructions: 1,
                max_allocated_bytes: 1024,
                max_concurrent: 100,
            },
            named,
        )
        .unwrap();
        let response = execute_request_with_profiles(
            &program,
            HttpMethod::Get,
            "/",
            &[],
            &[],
            &request,
            &profiles,
            &[],
            None,
        )
        .await
        .unwrap();
        match response {
            AppResponse::Html(h) => assert!(h.as_str().contains("42")),
            _ => panic!("expected html"),
        }

        let hard = ExecutionLimits {
            max_instructions: 3,
            max_allocated_bytes: 1024,
        };
        let result = execute_request_with_profiles(
            &program,
            HttpMethod::Get,
            "/",
            &[],
            &[],
            &hard,
            &profiles,
            &[],
            None,
        )
        .await;
        assert_eq!(result, Err(AppError::InstructionLimit));
    }
}

#[cfg(test)]
mod resource_profile_error_tests {
    use crate::{ResourceProfileConfig, ResourceProfileError, ResourceProfiles};
    use std::collections::HashMap;

    #[test]
    fn invalid_profile_configuration_returns_typed_error() {
        let default = ResourceProfileConfig {
            max_instructions: 1,
            max_allocated_bytes: 1,
            max_concurrent: 1,
        };
        let mut named = HashMap::new();
        named.insert(
            "bad-name".to_string(),
            ResourceProfileConfig {
                max_instructions: 1,
                max_allocated_bytes: 1,
                max_concurrent: 1,
            },
        );
        let error = match ResourceProfiles::new(default, named) {
            Ok(_) => panic!("invalid profile name must fail"),
            Err(error) => error,
        };
        assert_eq!(
            error,
            ResourceProfileError::InvalidName("bad-name".to_string())
        );
    }
}
