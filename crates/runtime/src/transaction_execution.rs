use crate::db_execution::execute_tx_query;
use crate::execution_context::Budget;
use crate::statement_execution::write_business_audit;
use data::Database;
use language_core::{AppError, Program, TRANSACTION_OUTCOME_ENUM_ID, TxStatement, Value};
use std::collections::HashMap;

pub(crate) async fn execute(
    program: &Program,
    outcome: &Option<String>,
    statements: &[TxStatement],
    env: &mut HashMap<String, Value>,
    budget: &mut Budget,
    database: &Database,
) -> Result<(), AppError> {
    let mut tx = match database.begin().await {
        Ok(tx) => tx,
        Err(_) if outcome.is_some() => {
            set_outcome(env, outcome.as_ref().expect("checked"), "RolledBack");
            return Ok(());
        }
        Err(_) => return Err(AppError::Database),
    };
    let mut failed = None;
    for statement in statements {
        let result = match statement {
            TxStatement::LetQuery { name, call } => {
                execute_tx_query(program, call, env, budget, &mut tx)
                    .await
                    .and_then(|value| {
                        budget.charge_value(&value)?;
                        env.insert(name.clone(), value);
                        Ok(())
                    })
            }
            TxStatement::Query(call) => execute_tx_query(program, call, env, budget, &mut tx)
                .await
                .map(|_| ()),
            TxStatement::BusinessAudit(audit) => {
                write_business_audit(audit, env, budget, &mut tx).await
            }
        };
        if let Err(error) = result {
            failed = Some(error);
            break;
        }
    }
    if let Some(error) = failed {
        if outcome.is_none() {
            let _ = tx.rollback().await;
            return Err(error);
        }
        if error != AppError::Database {
            let _ = tx.rollback().await;
            return Err(error);
        }
        let variant = if tx.rollback().await.is_ok() {
            "RolledBack"
        } else {
            "CommitUnknown"
        };
        set_outcome(env, outcome.as_ref().expect("checked"), variant);
        return Ok(());
    }
    if let Some(name) = outcome {
        let variant = if tx.commit().await.is_ok() {
            "Committed"
        } else {
            "CommitUnknown"
        };
        set_outcome(env, name, variant);
        return Ok(());
    }
    tx.commit().await.map_err(|_| AppError::Database)
}

fn set_outcome(env: &mut HashMap<String, Value>, name: &str, variant: &str) {
    env.insert(
        name.to_string(),
        Value::Enum {
            enum_id: TRANSACTION_OUTCOME_ENUM_ID,
            variant: variant.to_string(),
        },
    );
}
