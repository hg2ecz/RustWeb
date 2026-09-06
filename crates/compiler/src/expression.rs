use crate::expression_parser;
use crate::handler_types::StaticType;
use crate::{CompileError, builtin_types};
use language_core::{BinaryOp, Expr, Program, ValueType};
use std::collections::HashMap;

#[cfg(test)]
pub(super) fn parse_expr(input: &str, program: &Program) -> Result<Expr, CompileError> {
    expression_parser::parse_expr_in_namespace(input, "", program)
}

pub(super) fn parse_expr_in_namespace(
    input: &str,
    namespace: &str,
    program: &Program,
) -> Result<Expr, CompileError> {
    expression_parser::parse_expr_in_namespace(input, namespace, program)
}

pub(super) fn validate_expr(
    e: &Expr,
    k: &HashMap<String, StaticType>,
    p: &Program,
) -> Result<(), CompileError> {
    match e {
        Expr::Variable(n) => {
            if !k.contains_key(n) {
                return Err(CompileError::UnknownVariable(n.clone()));
            }
        }
        Expr::Field { base, field } => match k.get(base) {
            Some(StaticType::Model(m)) => {
                let model = p
                    .model(m)
                    .ok_or_else(|| CompileError::UnknownModel(m.clone()))?;
                if !model.fields.iter().any(|f| f.name == *field) {
                    return Err(CompileError::Syntax(format!(
                        "model `{m}` has no field `{field}`"
                    )));
                }
            }
            Some(StaticType::Upload) => {
                if !matches!(
                    field.as_str(),
                    "path" | "filename" | "contentType" | "bytes"
                ) {
                    return Err(CompileError::Syntax(format!(
                        "Upload has no field `{field}`"
                    )));
                }
            }
            _ => {
                return Err(CompileError::Syntax(format!(
                    "`{base}` is not a model/upload value"
                )));
            }
        },
        Expr::Slugify(inner) => {
            validate_expr(inner, k, p)?;
        }
        Expr::Builtin { args, .. } => {
            for arg in args {
                validate_expr(arg, k, p)?;
            }
        }
        Expr::Not(inner) => validate_expr(inner, k, p)?,
        Expr::Binary { left, right, .. } => {
            validate_expr(left, k, p)?;
            validate_expr(right, k, p)?
        }
        Expr::F32ArrayNew { len, fill } => {
            validate_expr(len, k, p)?;
            validate_expr(fill, k, p)?;
        }
        Expr::CollectionIndex { index, .. } => validate_expr(index, k, p)?,
        Expr::CollectionLen { .. } => {}
        _ => {}
    }
    Ok(())
}
pub(super) fn infer_static_expr_type(
    e: &Expr,
    k: &HashMap<String, StaticType>,
    p: &Program,
) -> Result<StaticType, CompileError> {
    match e {
        Expr::Variable(n) => k
            .get(n)
            .cloned()
            .ok_or_else(|| CompileError::UnknownVariable(n.clone())),
        _ => Ok(StaticType::Scalar(infer_expr_type(e, k, p)?)),
    }
}
pub(super) fn infer_expr_type(
    e: &Expr,
    k: &HashMap<String, StaticType>,
    p: &Program,
) -> Result<ValueType, CompileError> {
    match e {
        Expr::String(_) => Ok(ValueType::String),
        Expr::Int(_) => Ok(ValueType::Int),
        Expr::F32(_) => Ok(ValueType::F32),
        Expr::F32ArrayNew { len, fill } => {
            if infer_expr_type(len, k, p)? != ValueType::Int
                || infer_expr_type(fill, k, p)? != ValueType::F32
            {
                return Err(CompileError::Syntax(
                    "arrayF32(len, fill) requires Int and F32".into(),
                ));
            }
            Ok(ValueType::F32Array)
        }
        Expr::CollectionIndex { collection, index } => match k.get(collection) {
            Some(StaticType::Scalar(ValueType::F32Array)) => {
                if infer_expr_type(index, k, p)? != ValueType::Int {
                    return Err(CompileError::Syntax("Array<F32> index must be Int".into()));
                }
                Ok(ValueType::F32)
            }
            Some(StaticType::Scalar(ValueType::StringList)) => {
                if infer_expr_type(index, k, p)? != ValueType::Int {
                    return Err(CompileError::Syntax(
                        "List<String> index must be Int".into(),
                    ));
                }
                Ok(ValueType::String)
            }
            Some(StaticType::Scalar(ValueType::StringDict)) => {
                if infer_expr_type(index, k, p)? != ValueType::String {
                    return Err(CompileError::Syntax(
                        "Dict<String,String> key must be String".into(),
                    ));
                }
                Ok(ValueType::String)
            }
            _ => Err(CompileError::Syntax(format!(
                "`{collection}` is not an indexable collection"
            ))),
        },
        Expr::CollectionLen { collection } => {
            if matches!(
                k.get(collection),
                Some(StaticType::Scalar(
                    ValueType::F32Array | ValueType::StringList | ValueType::StringDict
                ))
            ) {
                Ok(ValueType::Int)
            } else {
                Err(CompileError::Syntax(format!(
                    "`{collection}` is not a collection"
                )))
            }
        }
        Expr::Builtin { function, args } => {
            builtin_types::infer_builtin_type(*function, args, k, p)
        }
        Expr::Bool(_) => Ok(ValueType::Bool),
        Expr::EnumLiteral { enum_id, .. } => Ok(ValueType::Enum(*enum_id)),
        Expr::Slugify(inner) => {
            if infer_expr_type(inner, k, p)? != ValueType::String {
                return Err(CompileError::Syntax(
                    "slug(...) requires a String expression".into(),
                ));
            }
            Ok(ValueType::Slug)
        }
        Expr::Variable(n) => match k.get(n) {
            Some(StaticType::Scalar(t)) => Ok(*t),
            Some(StaticType::Upload) => Err(CompileError::Syntax(format!(
                "Upload `{n}` cannot be interpolated directly; use a metadata field"
            ))),
            Some(StaticType::Model(_)) => Err(CompileError::Syntax(format!(
                "model `{n}` cannot be interpolated directly; use a field"
            ))),
            Some(StaticType::OptionalModel(_)) => Err(CompileError::Syntax(format!(
                "optional model `{n}` must be checked with `@if` before field access"
            ))),
            Some(StaticType::ListModel(_)) => Err(CompileError::Syntax(format!(
                "list `{n}` must be iterated with `@for`"
            ))),
            None => Err(CompileError::UnknownVariable(n.clone())),
        },
        Expr::Field { base, field } => match k.get(base) {
            Some(StaticType::Model(m)) => p
                .model(m)
                .and_then(|x| x.fields.iter().find(|f| f.name == *field))
                .map(|f| f.ty)
                .ok_or_else(|| CompileError::Syntax(format!("unknown field `{base}.{field}`"))),
            Some(StaticType::Upload) => match field.as_str() {
                "path" | "filename" | "contentType" => Ok(ValueType::String),
                "bytes" => Ok(ValueType::Int),
                _ => Err(CompileError::Syntax(format!(
                    "Upload has no field `{field}`"
                ))),
            },
            _ => Err(CompileError::Syntax(format!(
                "`{base}` is not a model/upload"
            ))),
        },
        Expr::Not(inner) => {
            if infer_expr_type(inner, k, p)? == ValueType::Bool {
                Ok(ValueType::Bool)
            } else {
                Err(CompileError::Syntax("logical ! requires Bool".into()))
            }
        }
        Expr::Binary { left, op, right } => {
            let l = infer_expr_type(left, k, p)?;
            let r = infer_expr_type(right, k, p)?;
            match op {
                BinaryOp::Add if l == ValueType::String && r == ValueType::String => {
                    Ok(ValueType::String)
                }
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem
                    if l == ValueType::Int && r == ValueType::Int =>
                {
                    Ok(ValueType::Int)
                }
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem
                    if l == ValueType::F32 && r == ValueType::F32 =>
                {
                    Ok(ValueType::F32)
                }
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem
                    if l == ValueType::Decimal && r == ValueType::Decimal =>
                {
                    Ok(ValueType::Decimal)
                }
                BinaryOp::ShiftLeft
                | BinaryOp::ShiftRight
                | BinaryOp::BitAnd
                | BinaryOp::BitXor
                | BinaryOp::BitOr
                    if l == ValueType::Int && r == ValueType::Int =>
                {
                    Ok(ValueType::Int)
                }
                BinaryOp::LogicalAnd | BinaryOp::LogicalOr
                    if l == ValueType::Bool && r == ValueType::Bool =>
                {
                    Ok(ValueType::Bool)
                }
                BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge
                    if l == r
                        && matches!(l, ValueType::Int | ValueType::F32 | ValueType::Decimal) =>
                {
                    Ok(ValueType::Bool)
                }
                BinaryOp::Eq | BinaryOp::Ne
                    if l == r
                        && matches!(
                            l,
                            ValueType::String
                                | ValueType::Int
                                | ValueType::F32
                                | ValueType::Decimal
                                | ValueType::Bool
                        ) =>
                {
                    Ok(ValueType::Bool)
                }
                _ => Err(CompileError::Syntax("invalid binary operands".into())),
            }
        }
    }
}
