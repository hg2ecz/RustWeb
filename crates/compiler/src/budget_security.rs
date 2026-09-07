use crate::diagnostics::CompileError;
use language_core::{ActionBody, ActionStatement, HttpMethod, PageBody, Program, Route, Statement};

pub(super) fn validate_route_budget(route: &Route, program: &Program) -> Result<(), CompileError> {
    if route.budget_profile.is_none() {
        return Ok(());
    }
    if handler_has_resource_scope(route, program) {
        return Err(CompileError::security(
            "SEC-A10-001",
            format!(
                "route `{}` declares a route budget and its handler also opens `with resource ...`",
                route.name
            ),
            Some(
                "use one route-level `budget <profile>` for the whole handler, or remove it and keep the scoped `with resource ...` block"
                    .to_string(),
            ),
        ));
    }
    Ok(())
}

fn handler_has_resource_scope(route: &Route, program: &Program) -> bool {
    match route.method {
        HttpMethod::Get => program.page(&route.handler).is_some_and(|page| {
            let PageBody::Statements(statements) = &page.body;
            statements
                .iter()
                .any(|statement| matches!(statement, Statement::Resource { .. }))
        }),
        HttpMethod::Post => program.action(&route.handler).is_some_and(|action| {
            let ActionBody::Statements(statements) = &action.body;
            statements
                .iter()
                .any(|statement| matches!(statement, ActionStatement::Resource { .. }))
        }),
    }
}
