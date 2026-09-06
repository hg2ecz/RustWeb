use crate::diagnostics::CompileError;
use crate::domain_symbols::display_domain_symbol;
use crate::expression::{
    infer_expr_type, infer_static_expr_type, parse_expr_in_namespace, validate_expr,
};
use crate::handler_types::{StaticType, scalar_known};
use crate::source_syntax::{
    consume_return_tail, find_statement_end, is_identifier, matching_brace, matching_paren,
    preview, read_ident, skip_ws_and_comments,
};
use crate::statement_helpers::{parse_object_authorize, parse_query_call};
use crate::{arrays, control_flow, dicts, html_template};
use language_core::{
    FunctionParam, Program, QueryCapability, ResourceUse, SourceLocation, Statement, ValueType,
};

pub(super) fn parse_page_statements(
    name: &str,
    namespace: &str,
    body: &str,
    params: &[FunctionParam],
    p: &mut Program,
    source_name: &str,
    base_line: usize,
    allow_resource: bool,
) -> Result<Vec<Statement>, CompileError> {
    let mut out = Vec::new();
    let mut known = scalar_known(params);
    known.insert("csrfToken".into(), StaticType::Scalar(ValueType::String));
    known.insert(
        "authPrincipal".into(),
        StaticType::Scalar(ValueType::String),
    );
    known.insert(
        "authMfaVerified".into(),
        StaticType::Scalar(ValueType::Bool),
    );
    let mut cursor = 0;
    while cursor < body.len() {
        cursor = skip_ws_and_comments(body, cursor);
        if cursor >= body.len() {
            break;
        }
        if body[cursor..].starts_with("with resource ") {
            if !allow_resource {
                return Err(CompileError::Syntax(format!(
                    "page `{name}` nested resource profiles are not supported in v0.1"
                )));
            }
            let profile_start = cursor + "with resource ".len();
            let profile = read_ident(body, profile_start).ok_or_else(|| {
                CompileError::Syntax(format!("page `{name}` resource profile name expected"))
            })?;
            let after = profile_start + profile.len();
            let open = body[after..].find('{').map(|v| after + v).ok_or_else(|| {
                CompileError::Syntax(format!("page `{name}` resource block missing {{"))
            })?;
            if !body[after..open].trim().is_empty() {
                return Err(CompileError::Syntax(format!(
                    "page `{name}` invalid resource profile syntax"
                )));
            }
            let close = matching_brace(body, open).ok_or_else(|| {
                CompileError::Syntax(format!("page `{name}` resource block unclosed"))
            })?;
            let line = base_line + body[..cursor].bytes().filter(|b| *b == b'\n').count();
            let source = SourceLocation {
                file: source_name.into(),
                line,
                function: display_domain_symbol(name),
            };
            p.resource_uses.push(ResourceUse {
                profile: profile.clone(),
                source: source.clone(),
            });
            let inner_base = base_line + body[..open + 1].bytes().filter(|b| *b == b'\n').count();
            let inner = parse_page_statements(
                name,
                namespace,
                &body[open + 1..close],
                params,
                p,
                source_name,
                inner_base,
                false,
            )?;
            out.push(Statement::Resource {
                profile,
                source,
                statements: inner,
            });
            cursor = skip_ws_and_comments(body, close + 1);
            if cursor < body.len() {
                return Err(CompileError::Syntax(format!(
                    "page `{name}` resource block must be the final statement in v0.1"
                )));
            }
            continue;
        }
        if body[cursor..].starts_with("let ") {
            let after = cursor + 4;
            let eq = body[after..]
                .find('=')
                .map(|v| after + v)
                .ok_or_else(|| CompileError::Syntax(format!("page `{name}` let has no =")))?;
            let local = body[after..eq].trim();
            if !is_identifier(local)
                || matches!(
                    local,
                    "csrfToken"
                        | "authPrincipal"
                        | "authMfaVerified"
                        | "__flashKind"
                        | "__flashMessage"
                )
            {
                return Err(CompileError::Syntax(format!(
                    "page `{name}` invalid local `{local}`"
                )));
            }
            let end = find_statement_end(body, eq + 1)?;
            let rhs = body[eq + 1..end].trim();
            if let Some((call, ty)) =
                parse_query_call(rhs, namespace, p, &known, QueryCapability::Db)?
            {
                known.insert(local.into(), ty);
                out.push(Statement::LetQuery {
                    name: local.into(),
                    call,
                });
            } else {
                let expr = parse_expr_in_namespace(rhs, namespace, p)?;
                validate_expr(&expr, &known, p)?;
                let ty = infer_expr_type(&expr, &known, p)?;
                known.insert(local.into(), StaticType::Scalar(ty));
                out.push(Statement::Let {
                    name: local.into(),
                    expr,
                });
            }
            cursor = end + 1;
            continue;
        }
        if body[cursor..].starts_with("while ") {
            let (condition, statements, close) =
                control_flow::parse_while_block("page", name, namespace, body, cursor, &known, p)?;
            out.push(Statement::While {
                condition,
                statements,
            });
            cursor = close + 1;
            continue;
        }
        if body[cursor..].starts_with("if ") {
            let (condition, statements, close) =
                control_flow::parse_if_block("page", name, namespace, body, cursor, &known, p)?;
            out.push(Statement::If {
                condition,
                statements,
            });
            cursor = close + 1;
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
                        let (array, index, value) =
                            arrays::parse_f32_array_set("page", name, text, namespace, &known, p)?;
                        out.push(Statement::F32ArraySet {
                            array,
                            index,
                            value,
                        });
                    }
                    Some(StaticType::Scalar(ValueType::StringDict)) => {
                        let (dict, key, value) =
                            dicts::parse_string_dict_set("page", name, text, namespace, &known, p)?;
                        out.push(Statement::StringDictSet { dict, key, value });
                    }
                    _ => {
                        return Err(CompileError::Syntax(format!(
                            "page `{name}` set target `{collection}` is not a mutable collection"
                        )));
                    }
                }
            } else {
                let (target, rhs) = text
                    .split_once('=')
                    .ok_or_else(|| CompileError::Syntax(format!("page `{name}` set requires =")))?;
                let target = target.trim();
                let expected = known
                    .get(target)
                    .cloned()
                    .ok_or_else(|| CompileError::UnknownVariable(target.into()))?;
                let expr = parse_expr_in_namespace(rhs.trim(), namespace, p)?;
                validate_expr(&expr, &known, p)?;
                if infer_static_expr_type(&expr, &known, p)? != expected {
                    return Err(CompileError::Syntax(format!(
                        "page `{name}` set `{target}` type mismatch"
                    )));
                }
                out.push(Statement::Set {
                    name: target.into(),
                    expr,
                });
            }
            cursor = end + 1;
            continue;
        }
        if body[cursor..].starts_with("authorize ") {
            let end = find_statement_end(body, cursor)?;
            let text = body[cursor..end].trim().trim_end_matches(';').trim();
            let rule = parse_object_authorize(text, &known, p, &format!("page `{name}`"))?;
            out.push(Statement::Authorize(rule));
            cursor = end + 1;
            continue;
        }
        if body[cursor..].starts_with("canonical slug ") {
            if !allow_resource {
                return Err(CompileError::Syntax(format!(
                    "page `{name}` canonical slug must be a top-level page statement"
                )));
            }
            if out
                .iter()
                .any(|statement| matches!(statement, Statement::CanonicalSlug { .. }))
            {
                return Err(CompileError::Syntax(format!(
                    "page `{name}` supports exactly one canonical slug invariant in v0.1"
                )));
            }
            let end = find_statement_end(body, cursor)?;
            let text = body[cursor..end].trim().trim_end_matches(';').trim();
            let rest = text.strip_prefix("canonical slug ").ok_or_else(|| {
                CompileError::Syntax(format!("page `{name}` invalid canonical slug syntax"))
            })?;
            let (param, expr_text) = rest.split_once(" from ").ok_or_else(|| {
                CompileError::Syntax(format!(
                    "page `{name}` canonical slug syntax is `canonical slug <path-param> from <Slug expression>`"
                ))
            })?;
            let param = param.trim();
            if !is_identifier(param) {
                return Err(CompileError::Syntax(format!(
                    "page `{name}` canonical slug path parameter is invalid"
                )));
            }
            match known.get(param) {
                Some(StaticType::Scalar(ValueType::Slug)) => {}
                _ => {
                    return Err(CompileError::Syntax(format!(
                        "page `{name}` canonical slug parameter `{param}` must have type Slug"
                    )));
                }
            }
            let canonical = parse_expr_in_namespace(expr_text.trim(), namespace, p)?;
            validate_expr(&canonical, &known, p)?;
            if infer_expr_type(&canonical, &known, p)? != ValueType::Slug {
                return Err(CompileError::Syntax(format!(
                    "page `{name}` canonical slug expression must have type Slug"
                )));
            }
            out.push(Statement::CanonicalSlug {
                param: param.into(),
                canonical,
            });
            cursor = end + 1;
            continue;
        }
        if body[cursor..].starts_with("return Ok(json(") {
            let start = cursor + "return Ok(json(".len();
            let close = matching_paren(body, start - 1).ok_or_else(|| {
                CompileError::Syntax(format!("page `{name}` json return unclosed"))
            })?;
            let expr = parse_expr_in_namespace(body[start..close].trim(), namespace, p)?;
            validate_expr(&expr, &known, p)?;
            out.push(Statement::ReturnJson(expr));
            cursor = consume_return_tail(body, close + 1)?;
            continue;
        }
        if body[cursor..].starts_with("return Ok(html") {
            let kw = cursor + "return Ok(html".len();
            let open = body[kw..]
                .find('{')
                .map(|v| kw + v)
                .ok_or_else(|| CompileError::Syntax(format!("page `{name}` html missing {{")))?;
            let close = matching_brace(body, open)
                .ok_or_else(|| CompileError::Syntax(format!("page `{name}` html unclosed")))?;
            out.push(Statement::ReturnHtml(html_template::parse_html_template(
                &body[open + 1..close],
                namespace,
                &known,
                p,
            )?));
            cursor = consume_return_tail(body, close + 1)?;
            continue;
        }
        return Err(CompileError::Syntax(format!(
            "page `{name}` unsupported statement near `{}`",
            preview(&body[cursor..])
        )));
    }
    if !matches!(
        out.last(),
        Some(Statement::ReturnHtml(_))
            | Some(Statement::ReturnJson(_))
            | Some(Statement::Resource { .. })
    ) {
        return Err(CompileError::Syntax(format!(
            "page `{name}` must return Html or Json"
        )));
    }
    Ok(out)
}
