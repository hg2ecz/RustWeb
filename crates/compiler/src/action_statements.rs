use crate::diagnostics::CompileError;
use crate::domain_symbols::display_domain_symbol;
use crate::expression::{
    infer_expr_type, infer_static_expr_type, parse_expr_in_namespace, validate_expr,
};
use crate::handler_types::{HandlerReturnKind, StaticType, scalar_known};
use crate::source_syntax::{
    consume_return_tail, find_statement_end, is_identifier, matching_brace, matching_paren,
    preview, read_ident, skip_ws_and_comments,
};
use crate::statement_helpers::{parse_business_audit, parse_object_authorize, parse_query_call};
use crate::{arrays, control_flow, dicts};
use language_core::{
    ActionStatement, Expr, FlashKind, FlashMessage, FunctionParam, Program, QueryCapability,
    QueryReturn, ResourceUse, SourceLocation, TxStatement, ValueType,
};

pub(super) fn parse_action_statements(
    name: &str,
    namespace: &str,
    body: &str,
    params: &[FunctionParam],
    p: &mut Program,
    source_name: &str,
    base_line: usize,
    allow_resource: bool,
) -> Result<Vec<ActionStatement>, CompileError> {
    let mut out = Vec::new();
    let mut known = scalar_known(params);
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
                    "action `{name}` nested resource profiles are not supported in v0.1"
                )));
            }
            let profile_start = cursor + "with resource ".len();
            let profile = read_ident(body, profile_start).ok_or_else(|| {
                CompileError::Syntax(format!("action `{name}` resource profile name expected"))
            })?;
            let after = profile_start + profile.len();
            let open = body[after..].find('{').map(|v| after + v).ok_or_else(|| {
                CompileError::Syntax(format!("action `{name}` resource block missing {{"))
            })?;
            if !body[after..open].trim().is_empty() {
                return Err(CompileError::Syntax(format!(
                    "action `{name}` invalid resource profile syntax"
                )));
            }
            let close = matching_brace(body, open).ok_or_else(|| {
                CompileError::Syntax(format!("action `{name}` resource block unclosed"))
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
            let inner = parse_action_statements(
                name,
                namespace,
                &body[open + 1..close],
                params,
                p,
                source_name,
                inner_base,
                false,
            )?;
            out.push(ActionStatement::Resource {
                profile,
                source,
                statements: inner,
            });
            cursor = skip_ws_and_comments(body, close + 1);
            if cursor < body.len() {
                return Err(CompileError::Syntax(format!(
                    "action `{name}` resource block must be the final statement in v0.1"
                )));
            }
            continue;
        }
        if body[cursor..].starts_with("transaction db") {
            let open = body[cursor + "transaction db".len()..]
                .find('{')
                .map(|v| cursor + "transaction db".len() + v)
                .ok_or_else(|| {
                    CompileError::Syntax(format!("action `{name}` transaction missing {{"))
                })?;
            let close = matching_brace(body, open).ok_or_else(|| {
                CompileError::Syntax(format!("action `{name}` transaction unclosed"))
            })?;
            let mut tx_known = known.clone();
            let mut statements = Vec::new();
            let mut tx_cursor = 0;
            let tx_body = &body[open + 1..close];
            while tx_cursor < tx_body.len() {
                tx_cursor = skip_ws_and_comments(tx_body, tx_cursor);
                if tx_cursor >= tx_body.len() {
                    break;
                }
                let end = find_statement_end(tx_body, tx_cursor)?;
                let line = tx_body[tx_cursor..end].trim().trim_end_matches(';').trim();
                if line.starts_with("audit ") {
                    statements.push(TxStatement::BusinessAudit(parse_business_audit(
                        name, namespace, line, &tx_known, p,
                    )?));
                } else if line.starts_with("let ") {
                    let rest = &line[4..];
                    let (local, rhs) = rest.split_once('=').ok_or_else(|| {
                        CompileError::Syntax(format!("transaction let `{line}` missing ="))
                    })?;
                    let local = local.trim();
                    if !is_identifier(local) {
                        return Err(CompileError::Syntax(format!(
                            "invalid transaction local `{local}`"
                        )));
                    }
                    let (call, ty) = parse_query_call(
                        rhs.trim(),
                        namespace,
                        p,
                        &tx_known,
                        QueryCapability::Transaction,
                    )?
                    .ok_or_else(|| {
                        CompileError::Syntax(format!(
                            "transaction let requires a mutating query call; got `{line}`"
                        ))
                    })?;
                    if matches!(
                        p.query(&call.query).map(|q| &q.return_type),
                        Some(&QueryReturn::Void)
                    ) {
                        return Err(CompileError::Syntax(format!(
                            "void query `{}` cannot be assigned to `{local}`",
                            call.query
                        )));
                    }
                    tx_known.insert(local.into(), ty.clone());
                    known.insert(local.into(), ty);
                    statements.push(TxStatement::LetQuery {
                        name: local.into(),
                        call,
                    });
                } else {
                    let (call, _) = parse_query_call(
                        line,
                        namespace,
                        p,
                        &tx_known,
                        QueryCapability::Transaction,
                    )?
                    .ok_or_else(|| {
                        CompileError::Syntax(format!(
                            "transaction only supports mutating query calls; got `{line}`"
                        ))
                    })?;
                    statements.push(TxStatement::Query(call));
                }
                tx_cursor = end + 1;
            }
            if statements.is_empty() {
                return Err(CompileError::Syntax(format!(
                    "action `{name}` empty transaction"
                )));
            }
            out.push(ActionStatement::Transaction { statements });
            cursor = close + 1;
            continue;
        }
        if body[cursor..].starts_with("let ") {
            let after = cursor + 4;
            let eq = body[after..]
                .find('=')
                .map(|v| after + v)
                .ok_or_else(|| CompileError::Syntax(format!("action `{name}` let has no =")))?;
            let local = body[after..eq].trim();
            if !is_identifier(local)
                || matches!(
                    local,
                    "authPrincipal" | "authMfaVerified" | "__flashKind" | "__flashMessage"
                )
            {
                return Err(CompileError::Syntax(format!(
                    "action `{name}` invalid local `{local}`"
                )));
            }
            let end = find_statement_end(body, eq + 1)?;
            let rhs = body[eq + 1..end].trim();
            if let Some((call, ty)) =
                parse_query_call(rhs, namespace, p, &known, QueryCapability::Db)?
            {
                known.insert(local.into(), ty);
                out.push(ActionStatement::LetQuery {
                    name: local.into(),
                    call,
                });
            } else {
                let expr = parse_expr_in_namespace(rhs, namespace, p)?;
                validate_expr(&expr, &known, p)?;
                let ty = infer_expr_type(&expr, &known, p)?;
                known.insert(local.into(), StaticType::Scalar(ty));
                out.push(ActionStatement::Let {
                    name: local.into(),
                    expr,
                });
            }
            cursor = end + 1;
            continue;
        }
        if body[cursor..].starts_with("while ") {
            let (condition, statements, close) = control_flow::parse_while_block(
                "action", name, namespace, body, cursor, &known, p,
            )?;
            out.push(ActionStatement::While {
                condition,
                statements,
            });
            cursor = close + 1;
            continue;
        }
        if body[cursor..].starts_with("if ") {
            let (condition, statements, close) =
                control_flow::parse_if_block("action", name, namespace, body, cursor, &known, p)?;
            out.push(ActionStatement::If {
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
                        let (array, index, value) = arrays::parse_f32_array_set(
                            "action", name, text, namespace, &known, p,
                        )?;
                        out.push(ActionStatement::F32ArraySet {
                            array,
                            index,
                            value,
                        });
                    }
                    Some(StaticType::Scalar(ValueType::StringDict)) => {
                        let (dict, key, value) = dicts::parse_string_dict_set(
                            "action", name, text, namespace, &known, p,
                        )?;
                        out.push(ActionStatement::StringDictSet { dict, key, value });
                    }
                    _ => {
                        return Err(CompileError::Syntax(format!(
                            "action `{name}` set target `{collection}` is not a mutable collection"
                        )));
                    }
                }
            } else {
                let (target, rhs) = text.split_once('=').ok_or_else(|| {
                    CompileError::Syntax(format!("action `{name}` set requires ="))
                })?;
                let target = target.trim();
                let expected = known
                    .get(target)
                    .cloned()
                    .ok_or_else(|| CompileError::UnknownVariable(target.into()))?;
                let expr = parse_expr_in_namespace(rhs.trim(), namespace, p)?;
                validate_expr(&expr, &known, p)?;
                if infer_static_expr_type(&expr, &known, p)? != expected {
                    return Err(CompileError::Syntax(format!(
                        "action `{name}` set `{target}` type mismatch"
                    )));
                }
                out.push(ActionStatement::Set {
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
            let rule = parse_object_authorize(text, &known, p, &format!("action `{name}`"))?;
            out.push(ActionStatement::Authorize(rule));
            cursor = end + 1;
            continue;
        }
        if body[cursor..].starts_with("flash ") {
            let end = find_statement_end(body, cursor)?;
            let text = body[cursor..end].trim().trim_end_matches(';').trim();
            let rest = text.strip_prefix("flash ").unwrap().trim();
            let (kind_raw, message_raw) =
                rest.split_once(char::is_whitespace).ok_or_else(|| {
                    CompileError::Syntax(format!(
                        "action `{name}` flash requires kind and string literal"
                    ))
                })?;
            let kind = match kind_raw {
                "success" => FlashKind::Success,
                "info" => FlashKind::Info,
                "warning" => FlashKind::Warning,
                "error" => FlashKind::Error,
                _ => {
                    return Err(CompileError::Syntax(format!(
                        "action `{name}` flash kind must be success, info, warning, or error"
                    )));
                }
            };
            let expr = parse_expr_in_namespace(message_raw.trim(), namespace, p)?;
            let Expr::String(message) = expr else {
                return Err(CompileError::Syntax(format!(
                    "action `{name}` flash message must be a compiler-owned string literal"
                )));
            };
            if message.is_empty()
                || message.len() > 200
                || message.bytes().any(|b| matches!(b, b'\r' | b'\n' | 0))
            {
                return Err(CompileError::Syntax(format!(
                    "action `{name}` flash message must be 1..200 bytes and single-line"
                )));
            }
            out.push(ActionStatement::Flash(FlashMessage { kind, message }));
            cursor = end + 1;
            continue;
        }
        if body[cursor..].starts_with("return Ok(json(") {
            let start = cursor + "return Ok(json(".len();
            let close = matching_paren(body, start - 1).ok_or_else(|| {
                CompileError::Syntax(format!("action `{name}` json return unclosed"))
            })?;
            let expr = parse_expr_in_namespace(body[start..close].trim(), namespace, p)?;
            validate_expr(&expr, &known, p)?;
            out.push(ActionStatement::ReturnJson(expr));
            cursor = consume_return_tail(body, close + 1)?;
            continue;
        }
        if body[cursor..].starts_with("return Ok(redirect(") {
            let start = cursor + "return Ok(redirect(".len();
            let close = matching_paren(body, start - 1).ok_or_else(|| {
                CompileError::Syntax(format!("action `{name}` redirect unclosed"))
            })?;
            let expr = parse_expr_in_namespace(body[start..close].trim(), namespace, p)?;
            validate_expr(&expr, &known, p)?;
            if infer_expr_type(&expr, &known, p)? != ValueType::String {
                return Err(CompileError::Syntax(
                    "redirect target must be String".into(),
                ));
            }
            out.push(ActionStatement::ReturnRedirect(expr));
            cursor = consume_return_tail(body, close + 1)?;
            continue;
        }
        return Err(CompileError::Syntax(format!(
            "action `{name}` unsupported statement near `{}`",
            preview(&body[cursor..])
        )));
    }
    if !matches!(
        out.last(),
        Some(ActionStatement::ReturnRedirect(_))
            | Some(ActionStatement::ReturnJson(_))
            | Some(ActionStatement::Resource { .. })
    ) {
        return Err(CompileError::Syntax(format!(
            "action `{name}` must return Redirect or Json"
        )));
    }
    fn flash_count(items: &[ActionStatement]) -> usize {
        items
            .iter()
            .map(|s| match s {
                ActionStatement::Flash(_) => 1,
                ActionStatement::Resource { statements, .. } => flash_count(statements),
                _ => 0,
            })
            .sum()
    }
    let flashes = flash_count(&out);
    if flashes > 1 {
        return Err(CompileError::Syntax(format!(
            "action `{name}` may set at most one flash message"
        )));
    }
    if flashes == 1 && control_flow::action_return_kind(&out) != Some(HandlerReturnKind::Redirect) {
        return Err(CompileError::Syntax(format!(
            "action `{name}` flash requires a Redirect return"
        )));
    }
    Ok(out)
}
