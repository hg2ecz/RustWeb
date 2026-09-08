use crate::diagnostics::CompileError;
use crate::handler_types::HandlerReturnKind;
use crate::module_namespace::qualify;
use crate::source_syntax::{function_bounds, line_number, split_top_level};
use crate::type_resolution::resolve_annotated_value_type;
use crate::{action_statements, control_flow, declarations, page_statements};
use language_core::{
    ActionBody, ActionFunction, FunctionParam, HandlerSecurityContract, PageBody, PageFunction,
    Program,
};

pub(super) fn parse_pages(
    source: &str,
    source_name: &str,
    namespace: &str,
    p: &mut Program,
) -> Result<(), CompileError> {
    let mut off = 0;
    while let Some(rel) = source[off..].find("page fn ") {
        let keyword = off + rel;
        if !declarations::is_top_level_declaration_at(source, keyword) {
            off = keyword + 8;
            continue;
        }
        let start = keyword + 8;
        let (name, sig_open, sig_close, body_open, body_close) =
            function_bounds(source, start, "page")?;
        let symbol_name = qualify(namespace, &name);
        if p.page(&symbol_name).is_some()
            || p.action(&symbol_name).is_some()
            || p.component(&symbol_name).is_some()
            || p.layout(&symbol_name).is_some()
        {
            return Err(CompileError::DuplicateHandler(name));
        }
        let (declared_return, security, declared_effects) = parse_handler_contract(
            "page",
            &name,
            &source[sig_close + 1..body_open],
            namespace,
            p,
        )?;
        let (params, needs_db) = parse_handler_params(
            &name,
            "PageContext",
            &source[sig_open + 1..sig_close],
            namespace,
            p,
        )?;
        let base_line = line_number(source, body_open + 1);
        let statements = page_statements::parse_page_statements(
            &symbol_name,
            namespace,
            &source[body_open + 1..body_close],
            &params,
            p,
            source_name,
            base_line,
            true,
        )?;
        if !control_flow::page_return_matches(&statements, declared_return) {
            return Err(CompileError::Syntax(format!(
                "page `{name}` return statement does not match declared return type"
            )));
        }
        let inferred_effects = crate::effect_security::collect_page(&statements);
        crate::effect_security::validate_network_capabilities(
            "page",
            &name,
            &declared_effects,
            &inferred_effects,
        )?;
        let effects = crate::effect_security::merge(declared_effects, inferred_effects);
        crate::effect_security::validate_db_capability("page", &name, needs_db, &effects)?;
        p.pages.push(PageFunction {
            name: symbol_name,
            params,
            needs_db,
            effects,
            security,
            body: PageBody::Statements(statements),
        });
        off = body_close + 1;
    }
    Ok(())
}

pub(super) fn parse_actions(
    source: &str,
    source_name: &str,
    namespace: &str,
    p: &mut Program,
) -> Result<(), CompileError> {
    let mut off = 0;
    while let Some(rel) = source[off..].find("action fn ") {
        let keyword = off + rel;
        if !declarations::is_top_level_declaration_at(source, keyword) {
            off = keyword + 10;
            continue;
        }
        let start = keyword + 10;
        let (name, sig_open, sig_close, body_open, body_close) =
            function_bounds(source, start, "action")?;
        let symbol_name = qualify(namespace, &name);
        if p.page(&symbol_name).is_some()
            || p.action(&symbol_name).is_some()
            || p.component(&symbol_name).is_some()
            || p.layout(&symbol_name).is_some()
        {
            return Err(CompileError::DuplicateHandler(name));
        }
        let (declared_return, security, declared_effects) = parse_handler_contract(
            "action",
            &name,
            &source[sig_close + 1..body_open],
            namespace,
            p,
        )?;
        let (params, needs_db) = parse_handler_params(
            &name,
            "ActionContext",
            &source[sig_open + 1..sig_close],
            namespace,
            p,
        )?;
        let base_line = line_number(source, body_open + 1);
        let statements = action_statements::parse_action_statements(
            &symbol_name,
            namespace,
            &source[body_open + 1..body_close],
            &params,
            p,
            source_name,
            base_line,
            true,
        )?;
        if !control_flow::action_return_matches(&statements, declared_return) {
            return Err(CompileError::Syntax(format!(
                "action `{name}` return statement does not match declared return type"
            )));
        }
        let inferred_effects = crate::effect_security::collect_action(&statements);
        crate::effect_security::validate_network_capabilities(
            "action",
            &name,
            &declared_effects,
            &inferred_effects,
        )?;
        let effects = crate::effect_security::merge(declared_effects, inferred_effects);
        crate::effect_security::validate_db_capability("action", &name, needs_db, &effects)?;
        p.actions.push(ActionFunction {
            name: symbol_name,
            params,
            needs_db,
            effects,
            security,
            body: ActionBody::Statements(statements),
        });
        off = body_close + 1;
    }
    Ok(())
}

fn parse_handler_params(
    function: &str,
    context: &str,
    input: &str,
    namespace: &str,
    p: &Program,
) -> Result<(Vec<FunctionParam>, bool), CompileError> {
    let mut params = Vec::new();
    let mut needs_db = false;
    for raw in split_top_level(input, ',') {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let (name, ty) = raw.split_once(':').ok_or_else(|| {
            CompileError::Syntax(format!(
                "function `{function}` parameter `{raw}` expected `name: Type`"
            ))
        })?;
        let name = name.trim();
        let ty = ty.trim();
        if matches!(name, "__flashKind" | "__flashMessage") {
            return Err(CompileError::Syntax(format!(
                "function `{function}` parameter name `{name}` is compiler-reserved"
            )));
        }
        if name == "ctx" && ty == context {
            continue;
        }
        if name == "db" && ty == "Db" {
            if needs_db {
                return Err(CompileError::Syntax(format!(
                    "function `{function}` duplicate Db capability"
                )));
            }
            needs_db = true;
            continue;
        }
        let annotated = resolve_annotated_value_type(ty, namespace, p).ok_or_else(|| {
            CompileError::Syntax(format!(
                "function `{function}` unsupported parameter type `{ty}`"
            ))
        })?;
        if matches!(annotated.value_type, language_core::ValueType::Credential(purpose) if purpose.is_crypto_key())
        {
            return Err(CompileError::security(
                "SEC-A04-023",
                format!("web handler parameter `{function}.{name}` cannot accept cryptographic key material from the request boundary"),
                Some("load cryptographic keys from trusted secret storage or a secret-classified model/query boundary".into()),
            ));
        }
        if annotated.sensitivity != language_core::DataSensitivity::Public {
            return Err(CompileError::security(
                "SEC-DATA-009",
                format!(
                    "web handler parameter `{function}.{name}` cannot declare classified request-boundary types"
                ),
                Some("request parameters are trust-boundary inputs, not secret capabilities; load classified data from a model or an explicit trusted capability instead".into()),
            ));
        }
        params.push(FunctionParam::public(name, annotated.value_type));
    }
    Ok((params, needs_db))
}

fn parse_handler_contract(
    kind: &str,
    name: &str,
    tail: &str,
    namespace: &str,
    program: &Program,
) -> Result<
    (
        HandlerReturnKind,
        HandlerSecurityContract,
        Vec<language_core::Effect>,
    ),
    CompileError,
> {
    let (tail, declared_effects) = match tail.rsplit_once(" uses ") {
        Some((before, raw)) => (
            before.trim_end(),
            crate::outbound_security::parse_handler_effects(name, raw, namespace, program)?,
        ),
        None => (tail, Vec::new()),
    };
    if tail.contains("requires") && tail.contains(" critical ") {
        return Err(CompileError::Syntax(format!(
            "handler `{name}` must use either `requires ...` or `critical <Operation>`, not both"
        )));
    }
    if let Some((return_tail, critical)) = tail.split_once(" critical ") {
        let return_kind = parse_handler_return_kind(kind, name, return_tail)?;
        let security =
            crate::handler_security::parse_critical(name, critical.trim(), namespace, program)?;
        return Ok((return_kind, security, declared_effects));
    }
    let (return_tail, requirements) = match tail.split_once("requires") {
        Some((before, after)) => (before, Some(after.trim())),
        None => (tail, None),
    };
    let return_kind = parse_handler_return_kind(kind, name, return_tail)?;
    let security = requirements
        .map(|raw| crate::handler_security::parse_requirements(name, raw, namespace, program))
        .transpose()?
        .unwrap_or_default();
    Ok((return_kind, security, declared_effects))
}

fn parse_handler_return_kind(
    kind: &str,
    name: &str,
    tail: &str,
) -> Result<HandlerReturnKind, CompileError> {
    let compact: String = tail.chars().filter(|c| !c.is_whitespace()).collect();
    match (kind, compact.as_str()) {
        ("page", "->Result<Html,PageError>") => Ok(HandlerReturnKind::Html),
        ("page", "->Result<Json,PageError>") => Ok(HandlerReturnKind::Json),
        ("action", "->Result<Redirect,PageError>") => Ok(HandlerReturnKind::Redirect),
        ("action", "->Result<Json,PageError>") => Ok(HandlerReturnKind::Json),
        _ => Err(CompileError::Syntax(format!(
            "{kind} `{name}` has unsupported return type `{}`",
            tail.trim()
        ))),
    }
}
