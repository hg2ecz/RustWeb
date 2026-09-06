use std::collections::HashMap;

use language_core::{AppError, Value};

use super::arrays;

pub(super) fn get(
    env: &HashMap<String, Value>,
    collection: &str,
    key: Value,
) -> Result<Value, AppError> {
    match (env.get(collection), key) {
        (Some(Value::F32Array(_)), Value::Int(index)) => {
            Ok(Value::F32(arrays::get_f32(env, collection, index)?))
        }
        (Some(Value::StringList(items)), Value::Int(index)) => {
            let idx = usize::try_from(index).map_err(|_| AppError::BadRequest)?;
            Ok(Value::String(
                items.get(idx).cloned().ok_or(AppError::BadRequest)?,
            ))
        }
        (Some(Value::StringDict(items)), Value::String(key)) => Ok(Value::String(
            items.get(&key).cloned().ok_or(AppError::BadRequest)?,
        )),
        _ => Err(AppError::Internal),
    }
}

pub(super) fn len(env: &HashMap<String, Value>, collection: &str) -> Result<i64, AppError> {
    let len = match env.get(collection) {
        Some(Value::F32Array(_)) => return arrays::len_f32(env, collection),
        Some(Value::StringList(items)) => items.len(),
        Some(Value::StringDict(items)) => items.len(),
        _ => return Err(AppError::Internal),
    };
    i64::try_from(len).map_err(|_| AppError::Internal)
}

pub(super) fn set_string_dict(
    env: &mut HashMap<String, Value>,
    dict: &str,
    key: String,
    value: String,
) -> Result<(), AppError> {
    const MAX_DICT_ITEMS: usize = 4096;
    const MAX_DICT_KEY_BYTES: usize = 1024;
    if key.is_empty() || key.len() > MAX_DICT_KEY_BYTES {
        return Err(AppError::BadRequest);
    }
    let Some(Value::StringDict(items)) = env.get_mut(dict) else {
        return Err(AppError::Internal);
    };
    if !items.contains_key(&key) && items.len() >= MAX_DICT_ITEMS {
        return Err(AppError::BadRequest);
    }
    items.insert(key, value);
    Ok(())
}
