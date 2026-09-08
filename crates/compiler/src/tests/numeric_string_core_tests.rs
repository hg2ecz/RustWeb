use super::*;

#[test]
fn arithmetic_bitwise_and_logical_operators_are_typed() {
    let program = Program::default();
    let known = HashMap::new();

    for source in [
        "17 % 5", "3 << 2", "12 >> 1", "12 & 10", "12 ^ 10", "12 | 10",
    ] {
        let expr = parse_expr(source, &program).unwrap();
        assert_eq!(
            infer_expr_type(&expr, &known, &program).unwrap(),
            ValueType::Int,
            "{source}"
        );
    }

    for source in ["true && false", "true || false", "!false", "1 < 2 && 3 < 4"] {
        let expr = parse_expr(source, &program).unwrap();
        assert_eq!(
            infer_expr_type(&expr, &known, &program).unwrap(),
            ValueType::Bool,
            "{source}"
        );
    }
}

#[test]
fn operator_precedence_keeps_arithmetic_shift_comparison_and_logic_layers() {
    let program = Program::default();
    let expr = parse_expr("1 + 2 * 3 << 1 == 14 && true", &program).unwrap();
    let Expr::Binary {
        op: BinaryOp::LogicalAnd,
        left,
        ..
    } = expr
    else {
        panic!("logical and expected at root");
    };
    assert!(matches!(
        *left,
        Expr::Binary {
            op: BinaryOp::Eq,
            ..
        }
    ));
}

#[test]
fn extended_math_builtins_are_strictly_f32() {
    let program = Program::default();
    let known = HashMap::new();

    for source in [
        "ln(2.0f32)",
        "log10(100.0f32)",
        "log(8.0f32, 2.0f32)",
        "exp(1.0f32)",
        "pow(2.0f32, 8.0f32)",
        "round(2.5f32)",
        "floor(2.5f32)",
        "ceil(2.5f32)",
    ] {
        let expr = parse_expr(source, &program).unwrap();
        assert_eq!(
            infer_expr_type(&expr, &known, &program).unwrap(),
            ValueType::F32,
            "{source}"
        );
    }

    let bad = parse_expr("pow(2, 8)", &program).unwrap();
    assert!(infer_expr_type(&bad, &known, &program).is_err());
}

#[test]
fn extended_string_builtins_have_explicit_types() {
    let program = Program::default();
    let known = HashMap::new();

    for source in [
        "trimStart(\" x \" )",
        "trimEnd(\" x \" )",
        "substring(\"árvíz\", 1)",
        "substring(\"árvíz\", 1, 3)",
        "charAt(\"árvíz\", 2)",
        "repeat(\"ab\", 3)",
    ] {
        let expr = parse_expr(source, &program).unwrap();
        assert_eq!(
            infer_expr_type(&expr, &known, &program).unwrap(),
            ValueType::String,
            "{source}"
        );
    }

    for source in ["indexOf(\"árvíz\", \"ví\")", "lastIndexOf(\"aba\", \"a\")"] {
        let expr = parse_expr(source, &program).unwrap();
        assert_eq!(
            infer_expr_type(&expr, &known, &program).unwrap(),
            ValueType::Int,
            "{source}"
        );
    }
}

#[test]
fn invalid_operator_types_are_rejected_at_compile_time() {
    let program = Program::default();
    let known = HashMap::new();

    for source in [
        "1 && 2",
        "true << 1",
        "1.0f32 & 2.0f32",
        "\"x\" % \"y\"",
        "!1",
    ] {
        let expr = parse_expr(source, &program).unwrap();
        assert!(
            infer_expr_type(&expr, &known, &program).is_err(),
            "{source}"
        );
    }
}
