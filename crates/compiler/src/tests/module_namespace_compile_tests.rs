use super::*;

mod m36_modules_slug_compile_tests {
    use super::*;
    use std::collections::HashSet;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_app() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("rwlang-m36-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn compiles_multi_file_application_and_preserves_source_file() {
        let dir = temp_app();
        fs::write(dir.join("main.rw"), "mod pages;\n").unwrap();
        fs::write(
            dir.join("pages.rw"),
            r#"
page fn article(ctx: PageContext, slug: Slug) -> Result<Html, PageError> {
    return Ok(html {<h1>{{ slug }}</h1>});
}
route article GET "/cikk/:slug<Slug>" => pages::article;
"#,
        )
        .unwrap();
        let program = compile_file(dir.join("main.rw")).unwrap();
        assert_eq!(program.routes.len(), 1);
        assert_eq!(program.routes[0].segments.len(), 2);
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn compile_file_reports_transitive_source_dependencies() {
        let dir = temp_app();
        fs::create_dir_all(dir.join("pages")).unwrap();
        fs::write(dir.join("main.rw"), "mod pages;\n").unwrap();
        fs::write(dir.join("pages.rw"), "mod pages::article;\n").unwrap();
        fs::write(
            dir.join("pages/article.rw"),
            r#"
page fn article(ctx: PageContext) -> Result<Html, PageError> { return Ok(html {ok}); }
route article GET "/article" => pages::article::article;
"#,
        )
        .unwrap();
        let compiled = compile_file_with_dependencies(dir.join("main.rw")).unwrap();
        let names: HashSet<String> = compiled
            .source_files
            .iter()
            .filter_map(|path| path.file_name().and_then(|v| v.to_str()).map(str::to_owned))
            .collect();
        assert!(names.contains("main.rw"));
        assert!(names.contains("pages.rw"));
        assert!(names.contains("article.rw"));
        assert!(compiled.program.page("pages::article::article").is_some());
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn nested_modules_use_application_root_namespace_layout() {
        let dir = temp_app();
        fs::create_dir_all(dir.join("pages")).unwrap();
        fs::write(dir.join("main.rw"), "mod pages;\n").unwrap();
        fs::write(dir.join("pages.rw"), "mod pages::article;\n").unwrap();
        fs::write(
            dir.join("pages/article.rw"),
            r#"
page fn article(ctx: PageContext, slug: Slug) -> Result<Html, PageError> { return Ok(html {ok}); }
route article GET "/cikk/:slug<Slug>" => pages::article::article;
"#,
        )
        .unwrap();
        let program = compile_file(dir.join("main.rw")).unwrap();
        assert!(program.page("pages::article::article").is_some());
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn module_resolution_is_fail_closed() {
        let dir = temp_app();
        fs::create_dir_all(dir.join("pages")).unwrap();
        fs::write(dir.join("main.rw"), "mod pages::article;\n").unwrap();
        fs::write(dir.join("pages/mod.rw"), "").unwrap();
        let err = compile_file(dir.join("main.rw")).unwrap_err().to_string();
        assert!(err.contains("pages/article.rw"));
        assert!(err.contains("application-root relative"));
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn module_symbols_stay_in_their_namespace() {
        let dir = temp_app();
        fs::write(dir.join("main.rw"), "mod a;\nmod b;\n").unwrap();
        fs::write(
            dir.join("a.rw"),
            r#"
page fn show(ctx: PageContext) -> Result<Html, PageError> { return Ok(html {a}); }
route showA GET "/a" => a::show;
"#,
        )
        .unwrap();
        fs::write(
            dir.join("b.rw"),
            r#"
page fn show(ctx: PageContext) -> Result<Html, PageError> { return Ok(html {b}); }
"#,
        )
        .unwrap();
        let program = compile_file(dir.join("main.rw")).unwrap();
        assert!(program.page("a::show").is_some());
        assert!(program.page("b::show").is_some());
        assert!(program.page("show").is_none());
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn local_handler_reference_resolves_inside_its_module() {
        let dir = temp_app();
        fs::write(dir.join("main.rw"), "mod pages;\n").unwrap();
        fs::write(
            dir.join("pages.rw"),
            r#"
page fn show(ctx: PageContext) -> Result<Html, PageError> { return Ok(html {ok}); }
route show GET "/" => show;
"#,
        )
        .unwrap();
        let program = compile_file(dir.join("main.rw")).unwrap();
        assert_eq!(program.routes[0].handler, "pages::show");
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn cross_module_handler_reference_must_be_qualified() {
        let dir = temp_app();
        fs::write(dir.join("main.rw"), "mod pages;\nmod routes;\n").unwrap();
        fs::write(
            dir.join("pages.rw"),
            r#"
page fn show(ctx: PageContext) -> Result<Html, PageError> { return Ok(html {ok}); }
"#,
        )
        .unwrap();
        fs::write(
            dir.join("routes.rw"),
            r#"
route show GET "/" => show;
"#,
        )
        .unwrap();
        let err = compile_file(dir.join("main.rw")).unwrap_err().to_string();
        assert!(err.contains("unknown handler") || err.contains("routes::show"));
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn nested_module_declaration_is_application_root_relative() {
        let dir = temp_app();
        fs::create_dir_all(dir.join("pages")).unwrap();
        fs::write(dir.join("main.rw"), "mod pages;\n").unwrap();
        fs::write(dir.join("pages.rw"), "mod article;\n").unwrap();
        fs::write(dir.join("pages/article.rw"), "").unwrap();
        let err = compile_file(dir.join("main.rw")).unwrap_err().to_string();
        assert!(err.contains("article.rw"));
        assert!(err.contains("application-root relative"));
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn module_declarations_after_code_are_rejected() {
        let dir = temp_app();
        fs::write(
            dir.join("main.rw"),
            r#"
page fn home(ctx: PageContext) -> Result<Html, PageError> { return Ok(html {ok}); }
mod pages;
"#,
        )
        .unwrap();
        fs::write(dir.join("pages.rw"), "").unwrap();
        let err = compile_file(dir.join("main.rw")).unwrap_err().to_string();
        assert!(err.contains("must appear before"));
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn relative_namespace_prefixes_are_rejected() {
        let dir = temp_app();
        fs::write(dir.join("main.rw"), "mod super::pages;\n").unwrap();
        let err = compile_file(dir.join("main.rw")).unwrap_err().to_string();
        assert!(err.contains("application-root relative"));
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn slug_builtin_is_typed() {
        let src = r#"
page fn article(ctx: PageContext, title: String) -> Result<Html, PageError> {
    let canonical = slug(title);
    return Ok(html {<p>{{ canonical }}</p>});
}
route article GET "/" query title<String> => article;
"#;
        let p = compile_source(src).unwrap();
        assert!(p.page("article").is_some());
    }
}
