use crate::cache_safety::expr_uses_request_state;
use crate::diagnostics::CompileError;
#[cfg(test)]
use crate::expression::parse_expr;
use crate::expression::{
    infer_expr_type, infer_static_expr_type, parse_expr_in_namespace, validate_expr,
};
use crate::handler_types::{HandlerReturnKind, StaticType};
use crate::source_syntax::{
    find_statement_end, is_identifier, matching_brace, preview, skip_ws_and_comments,
};
use crate::{arrays, dicts};
use language_core::{ActionStatement, ComputeStatement, Expr, Program, Statement, ValueType};
use std::collections::HashMap;

pub(super) fn parse_while_block(
    handler_kind: &str,
    handler_name: &str,
    namespace: &str,
    body: &str,
    cursor: usize,
    known: &HashMap<String, StaticType>,
    p: &Program,
) -> Result<(Expr, Vec<ComputeStatement>, usize), CompileError> {
    let cond_start = cursor + "while ".len();
    let open = body[cond_start..]
        .find('{')
        .map(|v| cond_start + v)
        .ok_or_else(|| {
            CompileError::Syntax(format!("{handler_kind} `{handler_name}` while missing {{"))
        })?;
    let condition = parse_expr_in_namespace(body[cond_start..open].trim(), namespace, p)?;
    validate_expr(&condition, known, p)?;
    if infer_expr_type(&condition, known, p)? != ValueType::Bool {
        return Err(CompileError::Syntax(format!(
            "{handler_kind} `{handler_name}` while condition must be Bool"
        )));
    }
    let close = matching_brace(body, open).ok_or_else(|| {
        CompileError::Syntax(format!(
            "{handler_kind} `{handler_name}` while block unclosed"
        ))
    })?;
    let mut inner_known = known.clone();
    let statements = parse_compute_statements(
        handler_kind,
        handler_name,
        namespace,
        &body[open + 1..close],
        &mut inner_known,
        p,
    )?;
    Ok((condition, statements, close))
}

pub(super) fn parse_if_block(
    handler_kind: &str,
    handler_name: &str,
    namespace: &str,
    body: &str,
    cursor: usize,
    known: &HashMap<String, StaticType>,
    p: &Program,
) -> Result<(Expr, Vec<ComputeStatement>, usize), CompileError> {
    let cond_start = cursor + "if ".len();
    let open = body[cond_start..]
        .find('{')
        .map(|v| cond_start + v)
        .ok_or_else(|| {
            CompileError::Syntax(format!("{handler_kind} `{handler_name}` if missing {{"))
        })?;
    let condition = parse_expr_in_namespace(body[cond_start..open].trim(), namespace, p)?;
    validate_expr(&condition, known, p)?;
    if infer_expr_type(&condition, known, p)? != ValueType::Bool {
        return Err(CompileError::Syntax(format!(
            "{handler_kind} `{handler_name}` if condition must be Bool"
        )));
    }
    let close = matching_brace(body, open).ok_or_else(|| {
        CompileError::Syntax(format!("{handler_kind} `{handler_name}` if block unclosed"))
    })?;
    let mut inner_known = known.clone();
    let statements = parse_compute_statements(
        handler_kind,
        handler_name,
        namespace,
        &body[open + 1..close],
        &mut inner_known,
        p,
    )?;
    Ok((condition, statements, close))
}

fn parse_compute_statements(
    handler_kind: &str,
    handler_name: &str,
    namespace: &str,
    body: &str,
    known: &mut HashMap<String, StaticType>,
    p: &Program,
) -> Result<Vec<ComputeStatement>, CompileError> {
    let mut out = Vec::new();
    let mut cursor = 0usize;
    while cursor < body.len() {
        cursor = skip_ws_and_comments(body, cursor);
        if cursor >= body.len() {
            break;
        }
        if body[cursor..].starts_with("let ") {
            let after = cursor + 4;
            let eq = body[after..].find('=').map(|v| after + v).ok_or_else(|| {
                CompileError::Syntax(format!("{handler_kind} `{handler_name}` let has no ="))
            })?;
            let local = body[after..eq].trim();
            if !is_identifier(local) {
                return Err(CompileError::Syntax(format!(
                    "{handler_kind} `{handler_name}` invalid local `{local}`"
                )));
            }
            let end = find_statement_end(body, eq + 1)?;
            let expr = parse_expr_in_namespace(body[eq + 1..end].trim(), namespace, p)?;
            validate_expr(&expr, known, p)?;
            let ty = infer_expr_type(&expr, known, p)?;
            known.insert(local.into(), StaticType::Scalar(ty));
            out.push(ComputeStatement::Let {
                name: local.into(),
                expr,
            });
            cursor = end + 1;
            continue;
        }
        if body[cursor..].starts_with("set ") {
            let end = find_statement_end(body, cursor)?;
            let text = body[cursor + 4..end].trim().trim_end_matches(';').trim();
            if text
                .split_once('=')
                .map(|(lhs, _)| lhs.contains('['))
                .unwrap_or(false)
            {
                let target = text
                    .split_once('=')
                    .map(|(lhs, _)| lhs.trim())
                    .unwrap_or("");
                let collection = target.split('[').next().unwrap_or("").trim();
                match known.get(collection) {
                    Some(StaticType::Scalar(ValueType::F32Array)) => {
                        let (array, index, value) = arrays::parse_f32_array_set(
                            handler_kind,
                            handler_name,
                            text,
                            namespace,
                            known,
                            p,
                        )?;
                        out.push(ComputeStatement::F32ArraySet {
                            array,
                            index,
                            value,
                        });
                    }
                    Some(StaticType::Scalar(ValueType::StringDict)) => {
                        let (dict, key, value) = dicts::parse_string_dict_set(
                            handler_kind,
                            handler_name,
                            text,
                            namespace,
                            known,
                            p,
                        )?;
                        out.push(ComputeStatement::StringDictSet { dict, key, value });
                    }
                    _ => {
                        return Err(CompileError::Syntax(format!(
                            "{handler_kind} `{handler_name}` set target `{collection}` is not a mutable collection"
                        )));
                    }
                }
            } else {
                let (name, rhs) = text.split_once('=').ok_or_else(|| {
                    CompileError::Syntax(format!("{handler_kind} `{handler_name}` set requires ="))
                })?;
                let name = name.trim();
                if !is_identifier(name) {
                    return Err(CompileError::Syntax(format!(
                        "{handler_kind} `{handler_name}` invalid set target `{name}`"
                    )));
                }
                let expected = known
                    .get(name)
                    .cloned()
                    .ok_or_else(|| CompileError::UnknownVariable(name.into()))?;
                let expr = parse_expr_in_namespace(rhs.trim(), namespace, p)?;
                validate_expr(&expr, known, p)?;
                let actual = infer_static_expr_type(&expr, known, p)?;
                if expected != actual {
                    return Err(CompileError::Syntax(format!(
                        "{handler_kind} `{handler_name}` set `{name}` type mismatch"
                    )));
                }
                out.push(ComputeStatement::Set {
                    name: name.into(),
                    expr,
                });
            }
            cursor = end + 1;
            continue;
        }
        if body[cursor..].starts_with("while ") {
            let (condition, statements, close) = parse_while_block(
                handler_kind,
                handler_name,
                namespace,
                body,
                cursor,
                known,
                p,
            )?;
            out.push(ComputeStatement::While {
                condition,
                statements,
            });
            cursor = close + 1;
            continue;
        }
        if body[cursor..].starts_with("if ") {
            let (condition, statements, close) = parse_if_block(
                handler_kind,
                handler_name,
                namespace,
                body,
                cursor,
                known,
                p,
            )?;
            out.push(ComputeStatement::If {
                condition,
                statements,
            });
            cursor = close + 1;
            continue;
        }
        return Err(CompileError::Syntax(format!(
            "{handler_kind} `{handler_name}` unsupported compute statement near `{}`",
            preview(&body[cursor..])
        )));
    }
    Ok(out)
}

pub(super) fn page_return_kind(statements: &[Statement]) -> Option<HandlerReturnKind> {
    match statements.last()? {
        Statement::ReturnHtml(_) => Some(HandlerReturnKind::Html),
        Statement::ReturnJson(_) => Some(HandlerReturnKind::Json),
        Statement::Resource { statements, .. } => page_return_kind(statements),
        Statement::Authorize(_)
        | Statement::CanonicalSlug { .. }
        | Statement::Let { .. }
        | Statement::Set { .. }
        | Statement::While { .. }
        | Statement::If { .. }
        | Statement::F32ArraySet { .. }
        | Statement::StringDictSet { .. }
        | Statement::LetQuery { .. } => None,
    }
}

pub(super) fn action_return_kind(statements: &[ActionStatement]) -> Option<HandlerReturnKind> {
    match statements.last()? {
        ActionStatement::ReturnRedirect(_) => Some(HandlerReturnKind::Redirect),
        ActionStatement::ReturnJson(_) => Some(HandlerReturnKind::Json),
        ActionStatement::Resource { statements, .. } => action_return_kind(statements),
        ActionStatement::Authorize(_)
        | ActionStatement::Let { .. }
        | ActionStatement::Set { .. }
        | ActionStatement::While { .. }
        | ActionStatement::If { .. }
        | ActionStatement::F32ArraySet { .. }
        | ActionStatement::StringDictSet { .. }
        | ActionStatement::LetQuery { .. }
        | ActionStatement::Transaction { .. }
        | ActionStatement::Flash(_) => None,
    }
}

pub(super) fn compute_uses_request_state(statements: &[ComputeStatement]) -> Option<&str> {
    for statement in statements {
        let hit = match statement {
            ComputeStatement::Let { expr, .. } | ComputeStatement::Set { expr, .. } => {
                expr_uses_request_state(expr)
            }
            ComputeStatement::F32ArraySet { index, value, .. } => {
                expr_uses_request_state(index).or_else(|| expr_uses_request_state(value))
            }
            ComputeStatement::StringDictSet { key, value, .. } => {
                expr_uses_request_state(key).or_else(|| expr_uses_request_state(value))
            }
            ComputeStatement::While {
                condition,
                statements,
            } => expr_uses_request_state(condition)
                .or_else(|| compute_uses_request_state(statements)),
            ComputeStatement::If {
                condition,
                statements,
            } => expr_uses_request_state(condition)
                .or_else(|| compute_uses_request_state(statements)),
        };
        if hit.is_some() {
            return hit;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile_source;
    use language_core::PageBody;

    #[test]
    fn compiles_budgeted_while_with_scalar_and_array_set() {
        let src = r#"
page fn test(ctx: PageContext) -> Result<Json, PageError> {
    let i = 0;
    let a = arrayF32(4, 0.0f32);
    while i < len(a) {
        set a[i] = 1.5f32;
        set i = i + 1;
    }
    return Ok(json(i));
}
route test GET "/test" => test;
"#;
        let p = compile_source(src).unwrap();
        let page = p.page("test").unwrap();
        let PageBody::Statements(statements) = &page.body;
        assert!(
            statements
                .iter()
                .any(|s| matches!(s, Statement::While { .. }))
        );
    }

    #[test]
    fn comparisons_are_typed_bool_and_mixed_numeric_types_are_rejected() {
        let p = Program::default();
        let known = HashMap::new();
        let e = parse_expr("3 < 4", &p).unwrap();
        assert_eq!(infer_expr_type(&e, &known, &p).unwrap(), ValueType::Bool);
        let mixed = parse_expr("3 < 4.0f32", &p).unwrap();
        assert!(infer_expr_type(&mixed, &known, &p).is_err());
    }
}

#[cfg(test)]
mod if_tests {
    use super::*;
    use crate::compile_source;
    use language_core::{BuiltinFunction, PageBody};

    #[test]
    fn compiles_if_with_tof32_and_nested_compute() {
        let src = r#"
page fn test(ctx: PageContext) -> Result<Json, PageError> {
    let i = 3;
    let x = toF32(i);
    if i < 4 {
        set i = i + 1;
    }
    return Ok(json(i));
}
route test GET "/test" => test;
"#;
        let p = compile_source(src).unwrap();
        let page = p.page("test").unwrap();
        let PageBody::Statements(statements) = &page.body;
        assert!(statements.iter().any(|s| matches!(s, Statement::If { .. })));
        assert!(statements.iter().any(|s| matches!(s, Statement::Let { name, expr: Expr::Builtin { function: BuiltinFunction::ToF32, .. } } if name == "x")));
    }
}
