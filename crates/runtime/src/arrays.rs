use language_core::{AppError, F32Value, Value};
use std::collections::HashMap;

pub(super) fn set_f32(
    env: &mut HashMap<String, Value>,
    array: &str,
    index: i64,
    value: F32Value,
) -> Result<(), AppError> {
    let index = usize::try_from(index).map_err(|_| AppError::BadRequest)?;
    let Some(Value::F32Array(items)) = env.get_mut(array) else {
        return Err(AppError::Internal);
    };
    let slot = items.get_mut(index).ok_or(AppError::BadRequest)?;
    *slot = value;
    Ok(())
}

pub(super) fn new_f32(len: i64, fill: F32Value) -> Result<Value, AppError> {
    let len = usize::try_from(len).map_err(|_| AppError::BadRequest)?;
    if len > 1_048_576 {
        return Err(AppError::MemoryLimit);
    }
    Ok(Value::F32Array(vec![fill; len]))
}

pub(super) fn get_f32(
    env: &HashMap<String, Value>,
    array: &str,
    index: i64,
) -> Result<F32Value, AppError> {
    let index = usize::try_from(index).map_err(|_| AppError::BadRequest)?;
    let Some(Value::F32Array(items)) = env.get(array) else {
        return Err(AppError::Internal);
    };
    items.get(index).copied().ok_or(AppError::BadRequest)
}

pub(super) fn len_f32(env: &HashMap<String, Value>, array: &str) -> Result<i64, AppError> {
    let Some(Value::F32Array(items)) = env.get(array) else {
        return Err(AppError::Internal);
    };
    i64::try_from(items.len()).map_err(|_| AppError::Internal)
}
