use crate::execution_context::Budget;
use crate::json_values::serialize_json_value;
use language_core::{AppError, OutboundCall, OutboundMethod, Value};
use std::collections::HashMap;

pub(crate) async fn execute(
    call: &OutboundCall,
    env: &HashMap<String, Value>,
    budget: &mut Budget,
    outbound: Option<&dyn crate::OutboundRuntime>,
) -> Result<Value, AppError> {
    let client = outbound.ok_or(AppError::Internal)?;
    let outcome = match call.method {
        OutboundMethod::Get => {
            budget.charge_external_io(call.path.len() as u64)?;
            client
                .get_status(&call.egress_target, &call.path)
                .await
                .map_err(|_| AppError::Internal)?
        }
        OutboundMethod::PostJson => {
            let body = call.body.as_ref().ok_or(AppError::Internal)?;
            let value = crate::vm::eval_expr(body, env, budget)?;
            let json = serialize_json_value(&value)?;
            budget.charge_alloc(json.len() as u64)?;
            budget.charge_external_io((call.path.len() + json.len()) as u64)?;
            client
                .post_json_status(&call.egress_target, &call.path, json.as_bytes())
                .await
                .map_err(|_| AppError::Internal)?
        }
    };
    budget.charge_external_io(outcome.transferred_bytes)?;
    Ok(Value::Int(i64::from(outcome.status)))
}
