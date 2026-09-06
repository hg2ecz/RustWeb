use crate::execution_context::{Budget, ExecutionLimits, ResourceProfiles};
use crate::request_binding::{
    decode_fields_into, decode_named_form_into, match_route, validate_route_inputs,
};
use crate::response::AppResponse;
use crate::statement_execution::{
    execute_action_plain, execute_action_statement, execute_page_plain, execute_page_statement,
};
use data::Database;
use futures_util::FutureExt;
use language_core::{
    ActionBody, ActionStatement, AppError, HttpMethod, PageBody, Program, Statement, Value,
};
use std::panic::AssertUnwindSafe;

pub async fn execute_request(
    program: &Program,
    method: HttpMethod,
    path: &str,
    form_pairs: &[(String, String)],
    db: Option<&Database>,
) -> Result<AppResponse, AppError> {
    execute_request_with_query_context(
        program,
        method,
        path,
        &[],
        form_pairs,
        &ExecutionLimits::default(),
        &[],
        db,
    )
    .await
}

pub async fn execute_request_with_context(
    program: &Program,
    method: HttpMethod,
    path: &str,
    form_pairs: &[(String, String)],
    limits: &ExecutionLimits,
    system_values: &[(String, Value)],
    db: Option<&Database>,
) -> Result<AppResponse, AppError> {
    execute_request_with_query_context(
        program,
        method,
        path,
        &[],
        form_pairs,
        limits,
        system_values,
        db,
    )
    .await
}

pub async fn execute_request_with_query_context(
    program: &Program,
    method: HttpMethod,
    path: &str,
    query_pairs: &[(String, String)],
    form_pairs: &[(String, String)],
    limits: &ExecutionLimits,
    system_values: &[(String, Value)],
    db: Option<&Database>,
) -> Result<AppResponse, AppError> {
    let profiles = ResourceProfiles::default_for_limits(limits);
    execute_request_with_profiles(
        program,
        method,
        path,
        query_pairs,
        form_pairs,
        limits,
        &profiles,
        system_values,
        db,
    )
    .await
}

pub async fn execute_request_with_profiles(
    program: &Program,
    method: HttpMethod,
    path: &str,
    query_pairs: &[(String, String)],
    form_pairs: &[(String, String)],
    limits: &ExecutionLimits,
    profiles: &ResourceProfiles,
    system_values: &[(String, Value)],
    db: Option<&Database>,
) -> Result<AppResponse, AppError> {
    let future = async {
        let mut budget = Budget::new(limits, profiles.default_config());
        execute_inner(
            program,
            method,
            path,
            query_pairs,
            form_pairs,
            &mut budget,
            profiles,
            system_values,
            db,
        )
        .await
    };
    match AssertUnwindSafe(future).catch_unwind().await {
        Ok(v) => v,
        Err(_) => Err(AppError::Internal),
    }
}

async fn execute_inner(
    program: &Program,
    method: HttpMethod,
    path: &str,
    query_pairs: &[(String, String)],
    form_pairs: &[(String, String)],
    budget: &mut Budget,
    profiles: &ResourceProfiles,
    system_values: &[(String, Value)],
    db: Option<&Database>,
) -> Result<AppResponse, AppError> {
    budget.charge(1)?;
    let (route, mut env) = match_route(program, method, path)?;
    decode_fields_into(program, &route.query_fields, query_pairs, &mut env)?;
    for (n, v) in system_values {
        budget.charge_value(v)?;
        env.insert(n.clone(), v.clone());
    }
    match method {
        HttpMethod::Get => {
            validate_route_inputs(route, &env)?;
            let page = program.page(&route.handler).ok_or(AppError::Internal)?;
            if page.needs_db && db.is_none() {
                return Err(AppError::Database);
            }
            let PageBody::Statements(statements) = &page.body;
            for s in statements {
                if let Statement::Resource {
                    profile,
                    statements: inner,
                    ..
                } = s
                {
                    let (profile_config, _permit) = profiles.acquire(profile).await?;
                    budget.push_profile(profile_config);
                    let result =
                        execute_page_plain(program, route, inner, &mut env, budget, db).await;
                    budget.pop_profile();
                    return result;
                }
                if let Some(response) =
                    execute_page_statement(program, route, s, &mut env, budget, db).await?
                {
                    return Ok(response);
                }
            }
        }
        HttpMethod::Post => {
            let body_schema = if route.json_fields.is_empty() {
                &route.form_fields
            } else {
                &route.json_fields
            };
            if route.form_schema.is_some() && route.json_fields.is_empty() {
                decode_named_form_into(program, route, form_pairs, &mut env)?;
            } else {
                decode_fields_into(program, body_schema, form_pairs, &mut env)?;
                validate_route_inputs(route, &env)?;
            }
            let action = program.action(&route.handler).ok_or(AppError::Internal)?;
            if action.needs_db && db.is_none() {
                return Err(AppError::Database);
            }
            let ActionBody::Statements(statements) = &action.body;
            for s in statements {
                if let ActionStatement::Resource {
                    profile,
                    statements: inner,
                    ..
                } = s
                {
                    let (profile_config, _permit) = profiles.acquire(profile).await?;
                    budget.push_profile(profile_config);
                    let result = execute_action_plain(program, inner, &mut env, budget, db).await;
                    budget.pop_profile();
                    return result;
                }
                if let Some(response) =
                    execute_action_statement(program, s, &mut env, budget, db).await?
                {
                    return Ok(response);
                }
            }
        }
    }
    Err(AppError::Internal)
}
