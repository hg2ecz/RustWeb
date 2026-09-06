use super::*;

#[test]
fn parses_f32_array_creation_index_and_len() {
    let p = Program::default();
    let mut known = HashMap::new();
    let create = parse_expr("arrayF32(4096, 0.0f32)", &p).unwrap();
    assert_eq!(
        infer_expr_type(&create, &known, &p).unwrap(),
        ValueType::F32Array
    );
    known.insert("a".into(), StaticType::Scalar(ValueType::F32Array));
    let index = parse_expr("a[17]", &p).unwrap();
    assert_eq!(infer_expr_type(&index, &known, &p).unwrap(), ValueType::F32);
    let len = parse_expr("len(a)", &p).unwrap();
    assert_eq!(infer_expr_type(&len, &known, &p).unwrap(), ValueType::Int);
}

#[test]
fn rejects_non_int_index_and_non_f32_fill() {
    let p = Program::default();
    let bad_fill = parse_expr("arrayF32(8, 0)", &p).unwrap();
    assert!(infer_expr_type(&bad_fill, &HashMap::new(), &p).is_err());
    let mut known = HashMap::new();
    known.insert("a".into(), StaticType::Scalar(ValueType::F32Array));
    let bad_index = parse_expr("a[1.0f32]", &p).unwrap();
    assert!(infer_expr_type(&bad_index, &known, &p).is_err());
}
