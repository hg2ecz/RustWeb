use crate::cache_safety::action_has_business_audit;
use crate::diagnostics::CompileError;
use language_core::{ActionBody, ActionStatement, CriticalOperation, Program};

pub(super) fn validate_program(program: &Program) -> Result<(), CompileError> {
    for action in &program.actions {
        let Some(name) = action.security.critical_operation.as_deref() else {
            continue;
        };
        let operation = program.critical_operation(name).ok_or_else(|| {
            CompileError::security(
                "SEC-A06-002",
                format!(
                    "handler `{}` references unknown critical operation `{name}`",
                    action.name
                ),
                None,
            )
        })?;
        let ActionBody::Statements(statements) = &action.body;
        validate_action_requirements(&action.name, operation, statements)?;
    }
    for page in &program.pages {
        let Some(name) = page.security.critical_operation.as_deref() else {
            continue;
        };
        let operation = program.critical_operation(name).ok_or_else(|| {
            CompileError::security(
                "SEC-A06-002",
                format!(
                    "handler `{}` references unknown critical operation `{name}`",
                    page.name
                ),
                None,
            )
        })?;
        if operation.transaction_required || operation.audit_required {
            return Err(CompileError::security(
                "SEC-A06-003",
                format!(
                    "page handler `{}` cannot satisfy critical operation `{}` transaction/audit requirements",
                    page.name, operation.name
                ),
                Some("use an action handler for state-changing critical operations".into()),
            ));
        }
    }
    Ok(())
}

fn validate_action_requirements(
    handler: &str,
    operation: &CriticalOperation,
    statements: &[ActionStatement],
) -> Result<(), CompileError> {
    if operation.transaction_required && !has_transaction(statements) {
        return Err(CompileError::security(
            "SEC-A06-004",
            format!(
                "critical handler `{handler}` must use a transaction for `{}`",
                operation.name
            ),
            Some("wrap the state-changing query calls in `transaction db { ... }`".into()),
        ));
    }
    if operation.audit_required && !action_has_business_audit(statements) {
        return Err(CompileError::security(
            "SEC-A09-001",
            format!(
                "critical handler `{handler}` must write an audit record for `{}`",
                operation.name
            ),
            Some("add a typed `audit ...` statement inside the transaction; secrets and sensitive values remain forbidden".into()),
        ));
    }
    Ok(())
}

fn has_transaction(statements: &[ActionStatement]) -> bool {
    statements.iter().any(|statement| match statement {
        ActionStatement::Transaction { .. } => true,
        ActionStatement::Resource { statements, .. } => has_transaction(statements),
        _ => false,
    })
}
