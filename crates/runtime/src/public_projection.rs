use crate::execution_context::Budget;
use language_core::{AppError, ProjectionSourceKind, PublicProjection, Value};
use std::collections::HashMap;

pub(crate) fn evaluate_public_projection(
    projection: &PublicProjection,
    env: &HashMap<String, Value>,
    budget: &mut Budget,
) -> Result<Value, AppError> {
    let source = env.get(&projection.source).ok_or(AppError::Internal)?;
    match projection.source_kind {
        ProjectionSourceKind::Model => project_record(source, projection, budget),
        ProjectionSourceKind::OptionalModel => match source {
            Value::Null => Ok(Value::Null),
            _ => project_record(source, projection, budget),
        },
        ProjectionSourceKind::ModelList => {
            let Value::List(items) = source else {
                return Err(AppError::Internal);
            };
            budget.charge_alloc((items.len() as u64).saturating_mul(16))?;
            let mut projected = Vec::with_capacity(items.len());
            for item in items {
                projected.push(project_record(item, projection, budget)?);
            }
            Ok(Value::List(projected))
        }
    }
}

fn project_record(
    source: &Value,
    projection: &PublicProjection,
    budget: &mut Budget,
) -> Result<Value, AppError> {
    let Value::Record(source) = source else {
        return Err(AppError::Internal);
    };
    budget.charge_alloc((projection.fields.len() as u64).saturating_mul(64))?;
    let mut fields = HashMap::with_capacity(projection.fields.len());
    for name in &projection.fields {
        let value = source.get(name).cloned().ok_or(AppError::Internal)?;
        fields.insert(name.clone(), value);
    }
    Ok(Value::Record(fields))
}
