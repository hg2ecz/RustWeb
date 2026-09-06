use language_core::{AppError, BuiltinFunction, Value};

const MAX_SPLIT_ITEMS: usize = 4096;

pub(crate) fn handles(function: BuiltinFunction) -> bool {
    matches!(
        function,
        BuiltinFunction::StringLen
            | BuiltinFunction::Trim
            | BuiltinFunction::TrimStart
            | BuiltinFunction::TrimEnd
            | BuiltinFunction::Lower
            | BuiltinFunction::Upper
            | BuiltinFunction::Contains
            | BuiltinFunction::StartsWith
            | BuiltinFunction::EndsWith
            | BuiltinFunction::Replace
            | BuiltinFunction::Split
            | BuiltinFunction::Substring
            | BuiltinFunction::IndexOf
            | BuiltinFunction::LastIndexOf
            | BuiltinFunction::CharAt
            | BuiltinFunction::Repeat
    )
}

pub(crate) fn eval(function: BuiltinFunction, stack: &mut Vec<Value>) -> Result<Value, AppError> {
    match function {
        BuiltinFunction::StringLen => {
            let text = pop_string(stack)?;
            Ok(Value::Int(
                text.chars().count().min(i64::MAX as usize) as i64
            ))
        }
        BuiltinFunction::Trim => unary_string(stack, |text| text.trim().to_owned()),
        BuiltinFunction::TrimStart => unary_string(stack, |text| text.trim_start().to_owned()),
        BuiltinFunction::TrimEnd => unary_string(stack, |text| text.trim_end().to_owned()),
        BuiltinFunction::Lower => unary_string(stack, |text| text.to_lowercase()),
        BuiltinFunction::Upper => unary_string(stack, |text| text.to_uppercase()),
        BuiltinFunction::Contains => {
            binary_string_bool(stack, |text, needle| text.contains(needle))
        }
        BuiltinFunction::StartsWith => {
            binary_string_bool(stack, |text, prefix| text.starts_with(prefix))
        }
        BuiltinFunction::EndsWith => {
            binary_string_bool(stack, |text, suffix| text.ends_with(suffix))
        }
        BuiltinFunction::Replace => replace(stack),
        BuiltinFunction::Split => split(stack),
        BuiltinFunction::Substring => substring(stack),
        BuiltinFunction::IndexOf => index_of(stack, false),
        BuiltinFunction::LastIndexOf => index_of(stack, true),
        BuiltinFunction::CharAt => char_at(stack),
        BuiltinFunction::Repeat => repeat(stack),
        _ => Err(AppError::Internal),
    }
}

pub(crate) fn estimated_result_alloc(
    function: BuiltinFunction,
    stack: &[Value],
) -> Result<u64, AppError> {
    match function {
        BuiltinFunction::StringLen
        | BuiltinFunction::Contains
        | BuiltinFunction::StartsWith
        | BuiltinFunction::EndsWith
        | BuiltinFunction::IndexOf
        | BuiltinFunction::LastIndexOf => Ok(0),
        BuiltinFunction::Trim | BuiltinFunction::TrimStart | BuiltinFunction::TrimEnd => {
            string_arg_len(stack, 0)
        }
        BuiltinFunction::Lower | BuiltinFunction::Upper => {
            string_arg_len(stack, 0).map(|bytes| bytes.saturating_mul(4))
        }
        BuiltinFunction::Replace => estimate_replace(stack),
        BuiltinFunction::Split => estimate_split(stack),
        BuiltinFunction::Substring => estimate_substring(stack),
        BuiltinFunction::CharAt => string_arg_len(stack, 1),
        BuiltinFunction::Repeat => estimate_repeat(stack),
        _ => Err(AppError::Internal),
    }
}

fn replace(stack: &mut Vec<Value>) -> Result<Value, AppError> {
    let to = pop_string(stack)?;
    let from = pop_string(stack)?;
    let text = pop_string(stack)?;
    if from.is_empty() {
        return Err(AppError::BadRequest);
    }
    Ok(Value::String(text.replace(from.as_str(), to.as_str())))
}

fn split(stack: &mut Vec<Value>) -> Result<Value, AppError> {
    let delimiter = pop_string(stack)?;
    let text = pop_string(stack)?;
    if delimiter.is_empty() {
        return Err(AppError::BadRequest);
    }
    let mut items = Vec::new();
    for piece in text.split(delimiter.as_str()) {
        if items.len() >= MAX_SPLIT_ITEMS {
            return Err(AppError::BadRequest);
        }
        items.push(piece.to_owned());
    }
    Ok(Value::StringList(items))
}

fn substring(stack: &mut Vec<Value>) -> Result<Value, AppError> {
    let has_length = matches!(stack.last(), Some(Value::Int(_)))
        && matches!(
            stack.get(stack.len().saturating_sub(2)),
            Some(Value::Int(_))
        )
        && matches!(
            stack.get(stack.len().saturating_sub(3)),
            Some(Value::String(_))
        );
    let length = if has_length {
        Some(pop_non_negative_usize(stack)?)
    } else {
        None
    };
    let start = pop_non_negative_usize(stack)?;
    let text = pop_string(stack)?;
    let chars: Vec<char> = text.chars().collect();
    if start > chars.len() {
        return Err(AppError::BadRequest);
    }
    let end = match length {
        Some(length) => start
            .checked_add(length)
            .ok_or(AppError::BadRequest)?
            .min(chars.len()),
        None => chars.len(),
    };
    Ok(Value::String(chars[start..end].iter().copied().collect()))
}

fn index_of(stack: &mut Vec<Value>, reverse: bool) -> Result<Value, AppError> {
    let needle = pop_string(stack)?;
    let text = pop_string(stack)?;
    let byte_index = if reverse {
        text.rfind(needle.as_str())
    } else {
        text.find(needle.as_str())
    };
    let Some(byte_index) = byte_index else {
        return Ok(Value::Int(-1));
    };
    Ok(Value::Int(
        text[..byte_index].chars().count().min(i64::MAX as usize) as i64,
    ))
}

fn char_at(stack: &mut Vec<Value>) -> Result<Value, AppError> {
    let index = pop_non_negative_usize(stack)?;
    let text = pop_string(stack)?;
    let Some(ch) = text.chars().nth(index) else {
        return Err(AppError::BadRequest);
    };
    Ok(Value::String(ch.to_string()))
}

fn repeat(stack: &mut Vec<Value>) -> Result<Value, AppError> {
    let count = pop_non_negative_usize(stack)?;
    let text = pop_string(stack)?;
    text.len().checked_mul(count).ok_or(AppError::BadRequest)?;
    Ok(Value::String(text.repeat(count)))
}

fn unary_string(
    stack: &mut Vec<Value>,
    operation: impl FnOnce(&str) -> String,
) -> Result<Value, AppError> {
    let text = pop_string(stack)?;
    Ok(Value::String(operation(&text)))
}

fn binary_string_bool(
    stack: &mut Vec<Value>,
    operation: impl FnOnce(&str, &str) -> bool,
) -> Result<Value, AppError> {
    let right = pop_string(stack)?;
    let left = pop_string(stack)?;
    Ok(Value::Bool(operation(&left, &right)))
}

fn pop_string(stack: &mut Vec<Value>) -> Result<String, AppError> {
    let Value::String(value) = stack.pop().ok_or(AppError::Internal)? else {
        return Err(AppError::Internal);
    };
    Ok(value)
}

fn pop_non_negative_usize(stack: &mut Vec<Value>) -> Result<usize, AppError> {
    let Value::Int(value) = stack.pop().ok_or(AppError::Internal)? else {
        return Err(AppError::Internal);
    };
    usize::try_from(value).map_err(|_| AppError::BadRequest)
}

fn string_arg_len(stack: &[Value], index_from_end: usize) -> Result<u64, AppError> {
    match stack.get(
        stack
            .len()
            .checked_sub(index_from_end + 1)
            .ok_or(AppError::Internal)?,
    ) {
        Some(Value::String(value)) => Ok(value.len() as u64),
        _ => Err(AppError::Internal),
    }
}

fn estimate_replace(stack: &[Value]) -> Result<u64, AppError> {
    let to = string_arg(stack, 0)?;
    let from = string_arg(stack, 1)?;
    let text = string_arg(stack, 2)?;
    if from.is_empty() {
        return Err(AppError::BadRequest);
    }
    let count = text.matches(from).count() as u64;
    Ok((text.len() as u64)
        .saturating_sub(count.saturating_mul(from.len() as u64))
        .saturating_add(count.saturating_mul(to.len() as u64)))
}

fn estimate_split(stack: &[Value]) -> Result<u64, AppError> {
    let delimiter = string_arg(stack, 0)?;
    let text = string_arg(stack, 1)?;
    if delimiter.is_empty() {
        return Err(AppError::BadRequest);
    }
    let count = text
        .matches(delimiter)
        .count()
        .saturating_add(1)
        .min(MAX_SPLIT_ITEMS + 1) as u64;
    Ok((text.len() as u64).saturating_add(count.saturating_mul(24)))
}

fn estimate_repeat(stack: &[Value]) -> Result<u64, AppError> {
    let count = match stack.last() {
        Some(Value::Int(value)) => u64::try_from(*value).map_err(|_| AppError::BadRequest)?,
        _ => return Err(AppError::Internal),
    };
    let text = string_arg(stack, 1)?;
    (text.len() as u64)
        .checked_mul(count)
        .ok_or(AppError::BadRequest)
}

fn string_arg(stack: &[Value], index_from_end: usize) -> Result<&str, AppError> {
    match stack.get(
        stack
            .len()
            .checked_sub(index_from_end + 1)
            .ok_or(AppError::Internal)?,
    ) {
        Some(Value::String(value)) => Ok(value),
        _ => Err(AppError::Internal),
    }
}

fn estimate_substring(stack: &[Value]) -> Result<u64, AppError> {
    let string_index = if matches!(stack.last(), Some(Value::Int(_)))
        && matches!(
            stack.get(stack.len().saturating_sub(2)),
            Some(Value::Int(_))
        )
        && matches!(
            stack.get(stack.len().saturating_sub(3)),
            Some(Value::String(_))
        ) {
        2
    } else {
        1
    };
    string_arg_len(stack, string_index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_indices_are_character_based() {
        let mut stack = vec![
            Value::String("árvíztűrő".into()),
            Value::String("tű".into()),
        ];
        assert_eq!(
            eval(BuiltinFunction::IndexOf, &mut stack).unwrap(),
            Value::Int(5)
        );

        let mut stack = vec![Value::String("árvíz".into()), Value::Int(1), Value::Int(3)];
        assert_eq!(
            eval(BuiltinFunction::Substring, &mut stack).unwrap(),
            Value::String("rví".into())
        );
    }

    #[test]
    fn replace_and_repeat_are_bounded_by_valid_inputs() {
        let mut stack = vec![
            Value::String("a-b-a".into()),
            Value::String("a".into()),
            Value::String("x".into()),
        ];
        assert_eq!(
            eval(BuiltinFunction::Replace, &mut stack).unwrap(),
            Value::String("x-b-x".into())
        );

        let mut stack = vec![Value::String("ab".into()), Value::Int(3)];
        assert_eq!(
            eval(BuiltinFunction::Repeat, &mut stack).unwrap(),
            Value::String("ababab".into())
        );
    }
}
