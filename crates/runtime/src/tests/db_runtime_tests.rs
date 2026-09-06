use crate::test_support::*;

#[cfg(test)]
mod db_integration_tests {
    use super::*;
    use compiler::compile_source;
    use data::{DbConfig, PreparedSql};

    const APP: &str = r#"
model Product {
    id: Int
    name: String
    price: Int
}
query fn loadProduct(db: Db, id: Int) -> Result<Product, DbError> sql {
    SELECT id, name, price FROM products WHERE id = :id
}
page fn product(ctx: PageContext, db: Db, id: Int) -> Result<Html, PageError> {
    let product = loadProduct(db, id)?;
    return Ok(html {<h1>{{ product.name }}</h1><p>{{ product.price }}</p>});
}
route product GET "/products/:id<Int>" => product;
"#;

    #[tokio::test]
    async fn compiler_to_sqlite_to_html_is_typed() {
        let program = compile_source(APP).unwrap();
        let mut cfg = DbConfig::secure_default("sqlite::memory:");
        cfg.max_connections = 1;
        let db = Database::connect(cfg).await.unwrap();
        db.execute(&PreparedSql::compile("CREATE TABLE products(id INTEGER PRIMARY KEY, name TEXT NOT NULL, price INTEGER NOT NULL)").unwrap(), &BindSet::new()).await.unwrap();
        db.execute(
            &PreparedSql::compile(
                "INSERT INTO products(id,name,price) VALUES (1,'<b>Keyboard</b>',19900)",
            )
            .unwrap(),
            &BindSet::new(),
        )
        .await
        .unwrap();
        let response = execute_request(&program, HttpMethod::Get, "/products/1", &[], Some(&db))
            .await
            .unwrap();
        match response {
            AppResponse::Html(html) => {
                assert!(html.as_str().contains("&lt;b&gt;Keyboard&lt;/b&gt;"));
                assert!(html.as_str().contains("19900"));
            }
            _ => panic!("expected html"),
        }
    }

    const CRUD_APP: &str = r#"
model Product {
    id: Int
    name: String
    price: Int
}
query fn listProducts(db: Db) -> Result<List<Product>, DbError> sql {
    SELECT id, name, price FROM products ORDER BY id
}
query fn findProduct(db: Db, id: Int) -> Result<Product?, DbError> sql {
    SELECT id, name, price FROM products WHERE id = :id
}
query fn createProduct(tx: Transaction, name: String, price: Int) -> Result<Void, DbError> sql {
    INSERT INTO products(name, price) VALUES (:name, :price)
}
query fn updateProduct(tx: Transaction, id: Int, name: String, price: Int) -> Result<Void, DbError> sql {
    UPDATE products SET name = :name, price = :price WHERE id = :id
}
query fn deleteProduct(tx: Transaction, id: Int) -> Result<Void, DbError> sql {
    DELETE FROM products WHERE id = :id
}
page fn products(ctx: PageContext, db: Db) -> Result<Html, PageError> {
    let products = listProducts(db)?;
    return Ok(html {<ul>@for product in products {<li>{{ product.name }}:{{ product.price }}</li>}</ul>});
}
page fn product(ctx: PageContext, db: Db, id: Int) -> Result<Html, PageError> {
    let product = findProduct(db, id)?;
    return Ok(html {@if product {<h1>{{ product.name }}</h1>}});
}
action fn create(ctx: ActionContext, db: Db, name: String, price: Int) -> Result<Redirect, PageError> {
    transaction db { createProduct(tx, name, price)?; }
    return Ok(redirect("/products"));
}
action fn update(ctx: ActionContext, db: Db, id: Int, name: String, price: Int) -> Result<Redirect, PageError> {
    transaction db { updateProduct(tx, id, name, price)?; }
    return Ok(redirect("/products"));
}
action fn delete(ctx: ActionContext, db: Db, id: Int) -> Result<Redirect, PageError> {
    transaction db { deleteProduct(tx, id)?; }
    return Ok(redirect("/products"));
}
route products GET "/products" => products;
route product GET "/products/:id<Int>" => product;
route create POST "/products" form name<String> price<Int> => create;
route update POST "/products/:id<Int>" form name<String> price<Int> => update;
route delete POST "/products/:id<Int>/delete" => delete;
"#;

    const TX_RETURN_APP: &str = r#"
model Product {
    id: Int
    name: String
    price: Int
}
query fn createReturning(tx: Transaction, name: String, price: Int) -> Result<Product, DbError> sql {
    INSERT INTO products(name, price) VALUES (:name, :price) RETURNING id, name, price
}
query fn renameById(tx: Transaction, id: Int, name: String) -> Result<Void, DbError> sql {
    UPDATE products SET name = :name WHERE id = :id
}
action fn createAndRename(ctx: ActionContext, db: Db, name: String, price: Int) -> Result<Redirect, PageError> {
    transaction db {
        let created = createReturning(tx, name, price)?;
        renameById(tx, created.id, "Committed")?;
    }
    return Ok(redirect("/done"));
}
page fn done(ctx: PageContext) -> Result<Html, PageError> { return Ok(html {done}); }
route create POST "/create" form name<String> price<Int> => createAndRename;
route done GET "/done" => done;
"#;

    #[tokio::test]
    async fn list_optional_and_crud_are_executed() {
        let program = compile_source(CRUD_APP).unwrap();
        let mut cfg = DbConfig::secure_default("sqlite::memory:");
        cfg.max_connections = 1;
        let db = Database::connect(cfg).await.unwrap();
        db.execute(&PreparedSql::compile("CREATE TABLE products(id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, price INTEGER NOT NULL)").unwrap(), &BindSet::new()).await.unwrap();
        db.execute(
            &PreparedSql::compile(
                "INSERT INTO products(name,price) VALUES ('<b>Keyboard</b>',19900)",
            )
            .unwrap(),
            &BindSet::new(),
        )
        .await
        .unwrap();

        let list = execute_request(&program, HttpMethod::Get, "/products", &[], Some(&db))
            .await
            .unwrap();
        match list {
            AppResponse::Html(h) => {
                assert!(h.as_str().contains("&lt;b&gt;Keyboard&lt;/b&gt;:19900"))
            }
            _ => panic!("expected html"),
        }

        let missing = execute_request(&program, HttpMethod::Get, "/products/999", &[], Some(&db))
            .await
            .unwrap();
        match missing {
            AppResponse::Html(h) => assert!(!h.as_str().contains("<h1>")),
            _ => panic!("expected html"),
        }

        execute_request(
            &program,
            HttpMethod::Post,
            "/products",
            &[
                ("name".into(), "Mouse".into()),
                ("price".into(), "9900".into()),
            ],
            Some(&db),
        )
        .await
        .unwrap();
        let created = execute_request(&program, HttpMethod::Get, "/products/2", &[], Some(&db))
            .await
            .unwrap();
        match created {
            AppResponse::Html(h) => assert!(h.as_str().contains("Mouse")),
            _ => panic!("expected html"),
        }

        execute_request(
            &program,
            HttpMethod::Post,
            "/products/2",
            &[
                ("name".into(), "Gaming Mouse".into()),
                ("price".into(), "12900".into()),
            ],
            Some(&db),
        )
        .await
        .unwrap();
        let updated = execute_request(&program, HttpMethod::Get, "/products/2", &[], Some(&db))
            .await
            .unwrap();
        match updated {
            AppResponse::Html(h) => assert!(h.as_str().contains("Gaming Mouse")),
            _ => panic!("expected html"),
        }

        execute_request(
            &program,
            HttpMethod::Post,
            "/products/2/delete",
            &[],
            Some(&db),
        )
        .await
        .unwrap();
        let deleted = execute_request(&program, HttpMethod::Get, "/products/2", &[], Some(&db))
            .await
            .unwrap();
        match deleted {
            AppResponse::Html(h) => assert!(!h.as_str().contains("Gaming Mouse")),
            _ => panic!("expected html"),
        }
    }

    #[tokio::test]
    async fn transaction_query_result_can_feed_next_query() {
        let program = compile_source(TX_RETURN_APP).unwrap();
        let mut cfg = DbConfig::secure_default("sqlite::memory:");
        cfg.max_connections = 1;
        let db = Database::connect(cfg).await.unwrap();
        db.execute(&PreparedSql::compile("CREATE TABLE products(id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, price INTEGER NOT NULL)").unwrap(),&BindSet::new()).await.unwrap();
        execute_request(
            &program,
            HttpMethod::Post,
            "/create",
            &[
                ("name".into(), "Temp".into()),
                ("price".into(), "100".into()),
            ],
            Some(&db),
        )
        .await
        .unwrap();
        let rows = db
            .fetch_all(
                &PreparedSql::compile("SELECT id, name, price FROM products").unwrap(),
                &BindSet::new(),
                &RowShape {
                    columns: vec![
                        ColumnSpec {
                            name: "id".into(),
                            ty: DbScalarType::Int,
                        },
                        ColumnSpec {
                            name: "name".into(),
                            ty: DbScalarType::String,
                        },
                        ColumnSpec {
                            name: "price".into(),
                            ty: DbScalarType::Int,
                        },
                    ],
                },
            )
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].get("name"),
            Some(&DbValue::String("Committed".into()))
        );
    }
}

#[cfg(test)]
mod m39_optimistic_lock_runtime_tests {
    use super::*;
    use crate::db_execution::execute_tx_query;
    use compiler::compile_source;
    use data::{DbConfig, PreparedSql};

    const APP: &str = r#"
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
route update POST "/articles/:id<Int>" form title<String> version<Int> => update;
"#;

    #[tokio::test]
    async fn stale_version_becomes_conflict() {
        let p = compile_source(APP).unwrap();
        let mut cfg = DbConfig::secure_default("sqlite::memory:");
        cfg.max_connections = 1;
        let db = Database::connect(cfg).await.unwrap();
        db.execute(
            &PreparedSql::compile("CREATE TABLE articles(id INTEGER PRIMARY KEY, title TEXT NOT NULL, version INTEGER NOT NULL)").unwrap(),
            &BindSet::new(),
        ).await.unwrap();
        db.execute(
            &PreparedSql::compile("INSERT INTO articles(id,title,version) VALUES (1,'old',3)")
                .unwrap(),
            &BindSet::new(),
        )
        .await
        .unwrap();

        let q = p.query("updateArticle").unwrap();
        let call = QueryCall {
            query: q.name.clone(),
            args: vec![Expr::Int(1), Expr::String("new".into()), Expr::Int(3)],
        };
        let env = HashMap::new();
        let limits = ExecutionLimits::default();
        let default_profile = ResourceProfileConfig {
            max_instructions: limits.max_instructions,
            max_allocated_bytes: limits.max_allocated_bytes,
            max_concurrent: 1,
        };
        let mut budget = Budget::new(&limits, default_profile);
        let mut tx = db.begin().await.unwrap();
        assert!(matches!(
            execute_tx_query(&p, &call, &env, &mut budget, &mut tx).await,
            Ok(Value::Bool(true))
        ));
        tx.commit().await.unwrap();

        let stale = QueryCall {
            query: q.name.clone(),
            args: vec![Expr::Int(1), Expr::String("stale".into()), Expr::Int(3)],
        };
        let mut budget = Budget::new(&limits, default_profile);
        let mut tx = db.begin().await.unwrap();
        assert!(matches!(
            execute_tx_query(&p, &stale, &env, &mut budget, &mut tx).await,
            Err(AppError::Conflict)
        ));
        tx.rollback().await.unwrap();
    }
}

#[cfg(test)]
mod m40_business_audit_runtime_tests {
    use super::*;
    use compiler::compile_source;
    use data::{ColumnSpec, DbConfig, DbScalarType, PreparedSql, RowShape};

    const APP: &str = r#"
enum ArticleStatus {
    Review
    Published
}
model Article {
    id: Int
    status: ArticleStatus
    version: Int
}
query fn articleById(db: Db, id: Int) -> Result<Article, DbError> sql {
    SELECT id, status, version FROM articles WHERE id = :id
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
route publishRoute POST "/articles/:id<Int>/publish" form version<Int> auth role Publisher => publishAction;
"#;

    async fn database(with_audit: bool) -> Database {
        let mut cfg = DbConfig::secure_default("sqlite::memory:");
        cfg.max_connections = 1;
        let db = Database::connect(cfg).await.unwrap();
        db.execute(&PreparedSql::compile("CREATE TABLE articles(id INTEGER PRIMARY KEY, status TEXT NOT NULL, version INTEGER NOT NULL)").unwrap(),&BindSet::new()).await.unwrap();
        db.execute(
            &PreparedSql::compile("INSERT INTO articles(id,status,version) VALUES(1,'Review',4)")
                .unwrap(),
            &BindSet::new(),
        )
        .await
        .unwrap();
        if with_audit {
            db.execute(&PreparedSql::compile("CREATE TABLE _rw_business_audit(event_id VARCHAR(36) PRIMARY KEY, occurred_at VARCHAR(35) NOT NULL, request_id VARCHAR(128) NOT NULL, actor VARCHAR(255) NOT NULL, source_action VARCHAR(128) NOT NULL, object_type VARCHAR(64) NOT NULL, object_id VARCHAR(255) NOT NULL, action VARCHAR(64) NOT NULL, previous_value VARCHAR(255) NOT NULL, new_value VARCHAR(255) NOT NULL)").unwrap(),&BindSet::new()).await.unwrap();
        }
        db
    }

    #[tokio::test]
    async fn business_change_and_audit_commit_together() {
        let p = compile_source(APP).unwrap();
        let db = database(true).await;
        let system = vec![
            ("authPrincipal".into(), Value::String("alice".into())),
            ("authMfaVerified".into(), Value::Bool(true)),
            (
                "__authRoles".into(),
                Value::List(vec![Value::String("Publisher".into())]),
            ),
            ("__requestId".into(), Value::String("req-123".into())),
        ];
        let r = execute_request_with_context(
            &p,
            HttpMethod::Post,
            "/articles/1/publish",
            &[("version".into(), "4".into())],
            &ExecutionLimits::default(),
            &system,
            Some(&db),
        )
        .await
        .unwrap();
        assert_eq!(r, AppResponse::Json("true".into()));
        let rows=db.fetch_all(&PreparedSql::compile("SELECT actor,object_type,object_id,action,previous_value,new_value,request_id FROM _rw_business_audit").unwrap(),&BindSet::new(),&RowShape{columns:vec![
            ColumnSpec{name:"actor".into(),ty:DbScalarType::String},ColumnSpec{name:"object_type".into(),ty:DbScalarType::String},ColumnSpec{name:"object_id".into(),ty:DbScalarType::String},ColumnSpec{name:"action".into(),ty:DbScalarType::String},ColumnSpec{name:"previous_value".into(),ty:DbScalarType::String},ColumnSpec{name:"new_value".into(),ty:DbScalarType::String},ColumnSpec{name:"request_id".into(),ty:DbScalarType::String}
        ]}).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get("actor"), Some(&DbValue::String("alice".into())));
        assert_eq!(
            rows[0].get("previous_value"),
            Some(&DbValue::String("Review".into()))
        );
        assert_eq!(
            rows[0].get("new_value"),
            Some(&DbValue::String("Published".into()))
        );
        assert_eq!(
            rows[0].get("request_id"),
            Some(&DbValue::String("req-123".into()))
        );
    }

    #[tokio::test]
    async fn missing_audit_table_rolls_back_business_change() {
        let p = compile_source(APP).unwrap();
        let db = database(false).await;
        let system = vec![
            ("authPrincipal".into(), Value::String("alice".into())),
            (
                "__authRoles".into(),
                Value::List(vec![Value::String("Publisher".into())]),
            ),
        ];
        assert!(matches!(
            execute_request_with_context(
                &p,
                HttpMethod::Post,
                "/articles/1/publish",
                &[("version".into(), "4".into())],
                &ExecutionLimits::default(),
                &system,
                Some(&db)
            )
            .await,
            Err(AppError::Database)
        ));
        let rows = db
            .fetch_all(
                &PreparedSql::compile("SELECT status,version FROM articles WHERE id=1").unwrap(),
                &BindSet::new(),
                &RowShape {
                    columns: vec![
                        ColumnSpec {
                            name: "status".into(),
                            ty: DbScalarType::String,
                        },
                        ColumnSpec {
                            name: "version".into(),
                            ty: DbScalarType::Int,
                        },
                    ],
                },
            )
            .await
            .unwrap();
        assert_eq!(
            rows[0].get("status"),
            Some(&DbValue::String("Review".into()))
        );
        assert_eq!(rows[0].get("version"), Some(&DbValue::Int(4)));
    }
}
