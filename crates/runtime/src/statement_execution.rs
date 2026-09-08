use crate::public_projection::evaluate_public_projection;
use crate::control_flow;
use crate::db_execution::execute_read_query;
use crate::domain_values;
use crate::execution_context::Budget;
use crate::rendering::build_current_route_url;
use crate::request_binding::validate_redirect_location;
use crate::scalars::is_canonical_slug;
use crate::templates;
use crate::vm::eval_expr;
use chrono::Utc;
use data::{BindSet, Database, DbTransaction, DbValue, PreparedSql};
use language_core::{
    ActionStatement, AppError, AuthorizationMode, FlashKind, FlashMessage, LocalUrl, Program, Redirect,
    Route, Statement, Value,
};
use std::collections::HashMap;
use uuid::Uuid;
use crate::response::AppResponse;

pub(crate) fn authorize_object(
    rule: &language_core::ObjectAuthorization,
    env: &HashMap<String, Value>,
    budget: &mut Budget,
) -> Result<(), AppError> {
    budget.charge(1)?;
    let principal = match env.get("authPrincipal") {
        Some(Value::String(v)) if !v.is_empty() => v,
        _ => return Err(AppError::Forbidden),
    };
    let role_allowed = match env.get("__authRoles") {
        Some(Value::List(v)) => v
            .iter()
            .any(|x| matches!(x,Value::String(role) if rule.allow_roles.iter().any(|r|r==role))),
        _ => false,
    };
    if role_allowed {
        return Ok(());
    }
    match &rule.mode {
        AuthorizationMode::Authenticated => Ok(()),
        AuthorizationMode::Owner { field } => {
            let record = match env.get(&rule.object) {
                Some(Value::Record(v)) => v,
                _ => return Err(AppError::Forbidden),
            };
            match record.get(field) {
                Some(Value::String(owner)) if owner == principal => Ok(()),
                _ => Err(AppError::Forbidden),
            }
        }
    }
}

pub(crate) async fn execute_page_plain(
    program: &Program,
    route: &Route,
    statements: &[Statement],
    env: &mut HashMap<String, Value>,
    budget: &mut Budget,
    db: Option<&Database>,
    outbound: Option<&dyn crate::OutboundRuntime>,
) -> Result<AppResponse, AppError> {
    for s in statements {
        if matches!(s, Statement::Resource { .. }) {
            return Err(AppError::Internal);
        }
        if let Some(r) = execute_page_statement(program, route, s, env, budget, db, outbound).await? {
            return Ok(r);
        }
    }
    Err(AppError::Internal)
}
pub(crate) async fn execute_page_statement(
    program: &Program,
    route: &Route,
    s: &Statement,
    env: &mut HashMap<String, Value>,
    budget: &mut Budget,
    db: Option<&Database>,
    outbound: Option<&dyn crate::OutboundRuntime>,
) -> Result<Option<AppResponse>, AppError> {
    budget.charge(1)?;
    match s {
        Statement::Let { name, expr } => {
            let v = eval_expr(expr, env, budget)?;
            budget.charge_value(&v)?;
            env.insert(name.clone(), v);
            Ok(None)
        }
        Statement::LetValidated { name, domain, expr } => {
            let value = eval_expr(expr, env, budget)?;
            let value = domain_values::validate(program, *domain, value)?;
            budget.charge_value(&value)?;
            env.insert(name.clone(), value);
            Ok(None)
        }
        Statement::Set { name, expr } => control_flow::assign(name, expr, env, budget).map(|_| None),
        Statement::While { condition, statements } => control_flow::execute_while(condition, statements, env, budget).map(|_| None),
        Statement::If { condition, statements } => control_flow::execute_if(condition, statements, env, budget).map(|_| None),
        Statement::Match { expr, enum_id, arms } => {
            let value = eval_expr(expr, env, budget)?;
            let variant = match value {
                Value::Enum { enum_id: actual, variant } if actual == *enum_id => variant,
                _ => return Err(AppError::Internal),
            };
            let arm = arms.iter().find(|arm| arm.variant == variant).ok_or(AppError::Internal)?;
            for statement in &arm.statements {
                if let Some(response) = Box::pin(execute_page_statement(program, route, statement, env, budget, db, outbound)).await? {
                    return Ok(Some(response));
                }
            }
            Ok(None)
        }
        Statement::F32ArraySet { array, index, value } => control_flow::set_f32_array(array, index, value, env, budget).map(|_| None),
        Statement::StringDictSet { dict, key, value } => control_flow::set_string_dict(dict, key, value, env, budget).map(|_| None),
        Statement::LetQuery { name, call } => {
            let database = db.ok_or(AppError::Database)?;
            let v = execute_read_query(program, call, env, budget, database).await?;
            budget.charge_value(&v)?;
            env.insert(name.clone(), v);
            Ok(None)
        }
        Statement::LetOutboundStatus { name, call } => {
            let value = crate::outbound_execution::execute(call, env, budget, outbound).await?;
            env.insert(name.clone(), value);
            Ok(None)
        }
        Statement::Authorize(rule) => {
            authorize_object(rule, env, budget)?;
            Ok(None)
        }
        Statement::CanonicalSlug { param, canonical } => {
            let requested = env.get(param).ok_or(AppError::Internal)?;
            let requested = match requested {
                Value::String(v) if is_canonical_slug(v) => v,
                _ => return Err(AppError::Internal),
            };
            let canonical_value = eval_expr(canonical, env, budget)?;
            let canonical_slug = match &canonical_value {
                Value::String(v) if is_canonical_slug(v) => v,
                _ => return Err(AppError::Internal),
            };
            if requested == canonical_slug {
                Ok(None)
            } else {
                let location =
                    build_current_route_url(program, route, env, Some((param, &canonical_value)))?;
                validate_redirect_location(&location)?;
                Ok(Some(AppResponse::Redirect(Redirect::permanent(
                    LocalUrl::parse(location).ok_or(AppError::Internal)?,
                ))))
            }
        }
        Statement::ReturnHtml(t) => {
            templates::render_html(program, t, env, budget).map(|h| Some(AppResponse::Html(h)))
        }
        Statement::ReturnJson(expr) => {
            let value = eval_expr(expr, env, budget)?;
            let json = crate::json_values::serialize_json_value(&value)?;
            budget.charge_alloc(json.len() as u64)?;
            Ok(Some(AppResponse::Json(json)))
        }
        Statement::ReturnJsonProjection(projection) => {
            let value = evaluate_public_projection(projection, env, budget)?;
            let json = crate::json_values::serialize_json_value(&value)?;
            budget.charge_alloc(json.len() as u64)?;
            Ok(Some(AppResponse::Json(json)))
        }
        Statement::Fail(error) => Err((*error).into()),
        Statement::Resource { .. } => Err(AppError::Internal),
    }
}
pub(crate) async fn execute_action_plain(
    program: &Program,
    statements: &[ActionStatement],
    env: &mut HashMap<String, Value>,
    budget: &mut Budget,
    db: Option<&Database>,
    outbound: Option<&dyn crate::OutboundRuntime>,
) -> Result<AppResponse, AppError> {
    for s in statements {
        if matches!(s, ActionStatement::Resource { .. }) {
            return Err(AppError::Internal);
        }
        if let Some(r) = execute_action_statement(program, s, env, budget, db, outbound).await? {
            return Ok(r);
        }
    }
    Err(AppError::Internal)
}
fn audit_value_text(value: &Value) -> Result<String, AppError> {
    let text = value.display_text().ok_or(AppError::Internal)?;
    if text.len() > 255 {
        return Err(AppError::BadRequest);
    }
    Ok(text)
}

pub(crate) async fn write_business_audit(
    audit: &language_core::BusinessAudit,
    env: &HashMap<String, Value>,
    budget: &mut Budget,
    tx: &mut DbTransaction<'_>,
) -> Result<(), AppError> {
    budget.charge(4)?;
    let actor = match env.get("authPrincipal") {
        Some(Value::String(v)) if !v.is_empty() => v.clone(),
        _ => return Err(AppError::Forbidden),
    };
    let request_id = match env.get("__requestId") {
        Some(Value::String(v)) => v.clone(),
        _ => String::new(),
    };
    let object_id = audit_value_text(&eval_expr(&audit.object_id, env, budget)?)?;
    let previous = match &audit.previous {
        Some(e) => Some(audit_value_text(&eval_expr(e, env, budget)?)?),
        None => None,
    };
    let new_value = match &audit.new_value {
        Some(e) => Some(audit_value_text(&eval_expr(e, env, budget)?)?),
        None => None,
    };
    if audit.object_type.len() > 64
        || audit.action.len() > 64
        || audit.source_action.len() > 128
        || actor.len() > 255
        || request_id.len() > 128
    {
        return Err(AppError::Internal);
    }
    let mut binds = BindSet::new();
    binds
        .insert(
            "event_id",
            DbValue::String(Uuid::new_v4().hyphenated().to_string()),
        )
        .map_err(|_| AppError::Internal)?;
    binds
        .insert(
            "occurred_at",
            DbValue::String(Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)),
        )
        .map_err(|_| AppError::Internal)?;
    binds
        .insert("request_id", DbValue::String(request_id))
        .map_err(|_| AppError::Internal)?;
    binds
        .insert("actor", DbValue::String(actor))
        .map_err(|_| AppError::Internal)?;
    binds
        .insert(
            "source_action",
            DbValue::String(audit.source_action.clone()),
        )
        .map_err(|_| AppError::Internal)?;
    binds
        .insert("object_type", DbValue::String(audit.object_type.clone()))
        .map_err(|_| AppError::Internal)?;
    binds
        .insert("object_id", DbValue::String(object_id))
        .map_err(|_| AppError::Internal)?;
    binds
        .insert("action", DbValue::String(audit.action.clone()))
        .map_err(|_| AppError::Internal)?;
    binds
        .insert(
            "previous_value",
            DbValue::String(previous.unwrap_or_default()),
        )
        .map_err(|_| AppError::Internal)?;
    binds
        .insert("new_value", DbValue::String(new_value.unwrap_or_default()))
        .map_err(|_| AppError::Internal)?;
    let sql=PreparedSql::compile("INSERT INTO _rw_business_audit(event_id,occurred_at,request_id,actor,source_action,object_type,object_id,action,previous_value,new_value) VALUES(:event_id,:occurred_at,:request_id,:actor,:source_action,:object_type,:object_id,:action,:previous_value,:new_value)").map_err(|_|AppError::Internal)?;
    let result = tx
        .execute(&sql, &binds)
        .await
        .map_err(|_| AppError::Database)?;
    if result.rows_affected != 1 {
        return Err(AppError::Database);
    }
    Ok(())
}

pub(crate) async fn execute_action_statement(
    program: &Program,
    s: &ActionStatement,
    env: &mut HashMap<String, Value>,
    budget: &mut Budget,
    db: Option<&Database>,
    outbound: Option<&dyn crate::OutboundRuntime>,
) -> Result<Option<AppResponse>, AppError> {
    budget.charge(1)?;
    match s {
        ActionStatement::Let { name, expr } => {
            let v = eval_expr(expr, env, budget)?;
            budget.charge_value(&v)?;
            env.insert(name.clone(), v);
            Ok(None)
        }
        ActionStatement::LetValidated { name, domain, expr } => {
            let value = eval_expr(expr, env, budget)?;
            let value = domain_values::validate(program, *domain, value)?;
            budget.charge_value(&value)?;
            env.insert(name.clone(), value);
            Ok(None)
        }
        ActionStatement::Set { name, expr } => control_flow::assign(name, expr, env, budget).map(|_| None),
        ActionStatement::While { condition, statements } => control_flow::execute_while(condition, statements, env, budget).map(|_| None),
        ActionStatement::If { condition, statements } => control_flow::execute_if(condition, statements, env, budget).map(|_| None),
        ActionStatement::Match { expr, enum_id, arms } => {
            let value = eval_expr(expr, env, budget)?;
            let variant = match value {
                Value::Enum { enum_id: actual, variant } if actual == *enum_id => variant,
                _ => return Err(AppError::Internal),
            };
            let arm = arms.iter().find(|arm| arm.variant == variant).ok_or(AppError::Internal)?;
            for statement in &arm.statements {
                if let Some(response) = Box::pin(execute_action_statement(program, statement, env, budget, db, outbound)).await? {
                    return Ok(Some(response));
                }
            }
            Ok(None)
        }
        ActionStatement::F32ArraySet { array, index, value } => control_flow::set_f32_array(array, index, value, env, budget).map(|_| None),
        ActionStatement::StringDictSet { dict, key, value } => control_flow::set_string_dict(dict, key, value, env, budget).map(|_| None),
        ActionStatement::LetQuery { name, call } => {
            let database = db.ok_or(AppError::Database)?;
            let v = execute_read_query(program, call, env, budget, database).await?;
            budget.charge_value(&v)?;
            env.insert(name.clone(), v);
            Ok(None)
        }
        ActionStatement::LetOutboundStatus { name, call } => {
            let value = crate::outbound_execution::execute(call, env, budget, outbound).await?;
            env.insert(name.clone(), value);
            Ok(None)
        }
        ActionStatement::Authorize(rule) => {
            authorize_object(rule, env, budget)?;
            Ok(None)
        }
        ActionStatement::Flash(flash) => {
            env.insert(
                "__flashKind".into(),
                Value::String(flash.kind.as_str().into()),
            );
            env.insert(
                "__flashMessage".into(),
                Value::String(flash.message.clone()),
            );
            Ok(None)
        }
        ActionStatement::Transaction { outcome, statements } => {
            let database = db.ok_or(AppError::Database)?;
            crate::transaction_execution::execute(program, outcome, statements, env, budget, database).await?;
            Ok(None)
        }
        ActionStatement::ReturnRedirect(call) => {
            let route = program
                .routes
                .iter()
                .find(|route| route.name == call.route)
                .ok_or(AppError::Internal)?;
            let location = crate::rendering::build_route_url(
                program,
                route,
                &call.args,
                env,
                budget,
                true,
            )?;
            validate_redirect_location(&location)?;
            let redirect = match (env.get("__flashKind"), env.get("__flashMessage")) {
                (Some(Value::String(kind)), Some(Value::String(message))) => {
                    let kind = match kind.as_str() {
                        "success" => FlashKind::Success,
                        "info" => FlashKind::Info,
                        "warning" => FlashKind::Warning,
                        "error" => FlashKind::Error,
                        _ => return Err(AppError::Internal),
                    };
                    Redirect::new(LocalUrl::parse(location).ok_or(AppError::Internal)?).with_flash(FlashMessage {
                        kind,
                        message: message.clone(),
                    })
                }
                (None, None) => Redirect::new(LocalUrl::parse(location).ok_or(AppError::Internal)?),
                _ => return Err(AppError::Internal),
            };
            Ok(Some(AppResponse::Redirect(redirect)))
        }
        ActionStatement::ReturnJson(expr) => {
            let value = eval_expr(expr, env, budget)?;
            let json = crate::json_values::serialize_json_value(&value)?;
            budget.charge_alloc(json.len() as u64)?;
            Ok(Some(AppResponse::Json(json)))
        }
        ActionStatement::ReturnJsonProjection(projection) => {
            let value = evaluate_public_projection(projection, env, budget)?;
            let json = crate::json_values::serialize_json_value(&value)?;
            budget.charge_alloc(json.len() as u64)?;
            Ok(Some(AppResponse::Json(json)))
        }
        ActionStatement::Fail(error) => Err((*error).into()),
        ActionStatement::Resource { .. } => Err(AppError::Internal),
    }
}
