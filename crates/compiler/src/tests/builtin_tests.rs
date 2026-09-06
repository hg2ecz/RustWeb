use super::*;

#[test]
fn parses_f32_math_builtins() {
    let p = Program::default();
    for source in ["sin(0.5f32)", "cos(0.5f32)", "sqrt(4.0f32)", "abs(-1.5f32)"] {
        let expr = parse_expr(source, &p).unwrap();
        assert_eq!(
            infer_expr_type(&expr, &HashMap::new(), &p).unwrap(),
            ValueType::F32
        );
    }
}

#[test]
fn abs_accepts_int_and_timer_returns_int() {
    let p = Program::default();
    let abs = parse_expr("abs(-42)", &p).unwrap();
    assert_eq!(
        infer_expr_type(&abs, &HashMap::new(), &p).unwrap(),
        ValueType::Int
    );
    let timer = parse_expr("monotonicNanos()", &p).unwrap();
    assert_eq!(
        infer_expr_type(&timer, &HashMap::new(), &p).unwrap(),
        ValueType::Int
    );
}

#[test]
fn rejects_wrong_math_builtin_types_and_arity() {
    let p = Program::default();
    let sin_int = parse_expr("sin(1)", &p).unwrap();
    assert!(infer_expr_type(&sin_int, &HashMap::new(), &p).is_err());
    let timer_arg = parse_expr("monotonicNanos(1)", &p).unwrap();
    assert!(infer_expr_type(&timer_arg, &HashMap::new(), &p).is_err());
    let abs_string = parse_expr("abs(\"x\")", &p).unwrap();
    assert!(infer_expr_type(&abs_string, &HashMap::new(), &p).is_err());
}

#[test]
fn public_cache_rejects_monotonic_timer_output() {
    let src = r#"
page fn timed(ctx: PageContext) -> Result<Html, PageError> {
    let started = monotonicNanos();
    return Ok(html {<p>{{ started }}</p>});
}
route timed GET "/timed" cache public ttl 60 => timed;
"#;
    assert!(compile_source(src).is_err());
}

#[test]
fn string_builtins_are_typed() {
    let p = Program::default();
    let env = HashMap::new();
    for source in [
        "trim(\" hello \")",
        "lower(\"Hello\")",
        "upper(\"Hello\")",
        "replace(\"a-b\", \"-\", \"_\")",
    ] {
        let expr = parse_expr(source, &p).unwrap();
        assert_eq!(infer_expr_type(&expr, &env, &p).unwrap(), ValueType::String);
    }
    for source in [
        "contains(\"hello\", \"ell\")",
        "startsWith(\"hello\", \"he\")",
        "endsWith(\"hello\", \"lo\")",
    ] {
        let expr = parse_expr(source, &p).unwrap();
        assert_eq!(infer_expr_type(&expr, &env, &p).unwrap(), ValueType::Bool);
    }
    let expr = parse_expr("stringLen(\"árvíz\")", &p).unwrap();
    assert_eq!(infer_expr_type(&expr, &env, &p).unwrap(), ValueType::Int);
}

#[test]
fn string_builtins_reject_wrong_types_and_arity() {
    let p = Program::default();
    let env = HashMap::new();
    for source in [
        "trim(1)",
        "contains(\"x\", 1)",
        "replace(\"x\", \"x\")",
        "stringLen(1)",
    ] {
        let expr = parse_expr(source, &p).unwrap();
        assert!(infer_expr_type(&expr, &env, &p).is_err(), "{source}");
    }
}

#[test]
fn regex_builtins_parse_and_infer_types() {
    let p = Program::default();
    let env = HashMap::new();

    let matched = parse_expr("regexMatch(\"abc-12\", \"^[a-z]+-[0-9]+$\")", &p).unwrap();
    assert_eq!(
        infer_expr_type(&matched, &env, &p).unwrap(),
        ValueType::Bool
    );

    let replaced = parse_expr("regexReplace(\"a1\", \"[0-9]\", \"#\")", &p).unwrap();
    assert_eq!(
        infer_expr_type(&replaced, &env, &p).unwrap(),
        ValueType::String
    );

    let captures = parse_expr("regexCaptures(\"a1\", \"([a-z])([0-9])\")", &p).unwrap();
    assert_eq!(
        infer_expr_type(&captures, &env, &p).unwrap(),
        ValueType::StringDict
    );
}

#[test]
fn regex_builtins_reject_wrong_types_and_arity() {
    let p = Program::default();
    let env = HashMap::new();
    for source in [
        "regexMatch(1, \"x\")",
        "regexReplace(\"x\", \"x\")",
        "regexCaptures(\"x\", 1)",
    ] {
        let expr = parse_expr(source, &p).unwrap();
        assert!(infer_expr_type(&expr, &env, &p).is_err(), "{source}");
    }
}
