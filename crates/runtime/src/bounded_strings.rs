use language_core::{AppError, Value};

const MAX_SPLIT_ITEMS: usize = 4096;

pub(crate) fn split(stack: &mut Vec<Value>) -> Result<Value, AppError> {
    let delimiter = pop_string(stack)?;
    let text = pop_string(stack)?;
    split_with_limit(text, delimiter, MAX_SPLIT_ITEMS)
}

pub(crate) fn split_bounded(stack: &mut Vec<Value>) -> Result<Value, AppError> {
    let max_items = pop_non_negative_usize(stack)?;
    if !(1..=MAX_SPLIT_ITEMS).contains(&max_items) {
        return Err(AppError::BadRequest);
    }
    let delimiter = pop_string(stack)?;
    let text = pop_string(stack)?;
    split_with_limit(text, delimiter, max_items)
}

pub(crate) fn estimate_split(stack: &[Value]) -> Result<u64, AppError> {
    estimate(stack, 0, 1, MAX_SPLIT_ITEMS)
}

pub(crate) fn estimate_split_bounded(stack: &[Value]) -> Result<u64, AppError> {
    let max_items = match stack.last() {
        Some(Value::Int(value)) => usize::try_from(*value).map_err(|_| AppError::BadRequest)?,
        _ => return Err(AppError::Internal),
    };
    if !(1..=MAX_SPLIT_ITEMS).contains(&max_items) {
        return Err(AppError::BadRequest);
    }
    estimate(stack, 1, 2, max_items)
}

fn split_with_limit(text: String, delimiter: String, max_items: usize) -> Result<Value, AppError> {
    if delimiter.is_empty() {
        return Err(AppError::BadRequest);
    }
    let mut items = Vec::new();
    for piece in text.split(delimiter.as_str()) {
        if items.len() >= max_items {
            return Err(AppError::BadRequest);
        }
        items.push(piece.to_owned());
    }
    Ok(Value::StringList(items))
}

fn estimate(
    stack: &[Value],
    delimiter_index: usize,
    text_index: usize,
    max_items: usize,
) -> Result<u64, AppError> {
    let delimiter = string_arg(stack, delimiter_index)?;
    let text = string_arg(stack, text_index)?;
    if delimiter.is_empty() {
        return Err(AppError::BadRequest);
    }
    let count = text
        .matches(delimiter)
        .count()
        .saturating_add(1)
        .min(max_items.saturating_add(1)) as u64;
    Ok((text.len() as u64).saturating_add(count.saturating_mul(24)))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_split_fails_closed_when_limit_is_exceeded() {
        let mut stack = vec![
            Value::String("a,b,c".into()),
            Value::String(",".into()),
            Value::Int(2),
        ];
        assert_eq!(split_bounded(&mut stack), Err(AppError::BadRequest));
    }
}
