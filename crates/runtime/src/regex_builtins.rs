use language_core::{AppError, BuiltinFunction, Value};
use regex::Regex;
use std::collections::BTreeMap;

const MAX_PATTERN_BYTES: usize = 4 * 1024;
const MAX_INPUT_BYTES: usize = 1024 * 1024;
const MAX_CAPTURES: usize = 64;
const MAX_REPLACEMENT_BYTES: usize = 16 * 1024;
const MAX_RESULT_BYTES: u64 = 16 * 1024 * 1024;

pub(crate) struct PreparedRegexBuiltin {
    function: BuiltinFunction,
    regex: Regex,
    result_alloc: u64,
}

impl PreparedRegexBuiltin {
    pub(crate) fn result_alloc(&self) -> u64 {
        self.result_alloc
    }
}

pub(crate) fn prepare(
    function: BuiltinFunction,
    stack: &[Value],
) -> Result<PreparedRegexBuiltin, AppError> {
    let (text, pattern) = peek_text_pattern(function, stack)?;
    let regex = compile(pattern, text)?;
    let result_alloc = match function {
        BuiltinFunction::RegexMatch => 0,
        BuiltinFunction::RegexReplace => estimate_replace_bytes(stack, &regex)?,
        BuiltinFunction::RegexCaptures => estimate_capture_bytes(text, &regex)?,
        _ => return Err(AppError::Internal),
    };
    Ok(PreparedRegexBuiltin {
        function,
        regex,
        result_alloc,
    })
}

pub(crate) fn eval_prepared(
    prepared: PreparedRegexBuiltin,
    stack: &mut Vec<Value>,
) -> Result<Value, AppError> {
    match prepared.function {
        BuiltinFunction::RegexMatch => {
            let (text, _pattern) = pop_text_pattern(stack)?;
            Ok(Value::Bool(prepared.regex.is_match(&text)))
        }
        BuiltinFunction::RegexReplace => {
            let Value::String(replacement) = stack.pop().ok_or(AppError::Internal)? else {
                return Err(AppError::Internal);
            };
            if replacement.len() > MAX_REPLACEMENT_BYTES {
                return Err(AppError::BadRequest);
            }
            let (text, _pattern) = pop_text_pattern(stack)?;
            Ok(Value::String(
                prepared
                    .regex
                    .replace_all(&text, replacement.as_str())
                    .into_owned(),
            ))
        }
        BuiltinFunction::RegexCaptures => {
            let (text, _pattern) = pop_text_pattern(stack)?;
            captures_to_value(&prepared.regex, &text)
        }
        _ => Err(AppError::Internal),
    }
}

fn captures_to_value(regex: &Regex, text: &str) -> Result<Value, AppError> {
    if regex.captures_len() > MAX_CAPTURES + 1 {
        return Err(AppError::BadRequest);
    }
    let Some(captures) = regex.captures(text) else {
        return Ok(Value::StringDict(BTreeMap::new()));
    };
    let mut out = BTreeMap::new();
    for (index, item) in captures.iter().enumerate() {
        if let Some(matched) = item {
            out.insert(index.to_string(), matched.as_str().to_owned());
        }
    }
    for (index, name) in regex.capture_names().enumerate() {
        if let Some(name) = name {
            if let Some(matched) = captures.get(index) {
                out.insert(name.to_owned(), matched.as_str().to_owned());
            }
        }
    }
    Ok(Value::StringDict(out))
}

fn estimate_replace_bytes(stack: &[Value], regex: &Regex) -> Result<u64, AppError> {
    let Value::String(replacement) = stack.last().ok_or(AppError::Internal)? else {
        return Err(AppError::Internal);
    };
    if replacement.len() > MAX_REPLACEMENT_BYTES {
        return Err(AppError::BadRequest);
    }
    let Some(Value::String(text)) =
        stack.get(stack.len().checked_sub(3).ok_or(AppError::Internal)?)
    else {
        return Err(AppError::Internal);
    };
    let mut total = 0_u64;
    let mut previous_end = 0_usize;
    let mut expanded = String::new();
    for captures in regex.captures_iter(text) {
        let matched = captures.get(0).ok_or(AppError::Internal)?;
        total = checked_result_size(total, matched.start().saturating_sub(previous_end) as u64)?;
        expanded.clear();
        captures.expand(replacement, &mut expanded);
        total = checked_result_size(total, expanded.len() as u64)?;
        previous_end = matched.end();
    }
    checked_result_size(total, text.len().saturating_sub(previous_end) as u64)
}

fn estimate_capture_bytes(text: &str, regex: &Regex) -> Result<u64, AppError> {
    if regex.captures_len() > MAX_CAPTURES + 1 {
        return Err(AppError::BadRequest);
    }
    let Some(captures) = regex.captures(text) else {
        return Ok(0);
    };
    let mut total = 0_u64;
    for (index, item) in captures.iter().enumerate() {
        if let Some(matched) = item {
            total = checked_result_size(total, matched.as_str().len() as u64)?;
            total = checked_result_size(total, index.to_string().len() as u64 + 48)?;
        }
    }
    for (index, name) in regex.capture_names().enumerate() {
        if let Some(name) = name {
            if let Some(matched) = captures.get(index) {
                total = checked_result_size(
                    total,
                    name.len() as u64 + matched.as_str().len() as u64 + 48,
                )?;
            }
        }
    }
    Ok(total)
}

fn checked_result_size(current: u64, add: u64) -> Result<u64, AppError> {
    let next = current.checked_add(add).ok_or(AppError::BadRequest)?;
    if next > MAX_RESULT_BYTES {
        return Err(AppError::BadRequest);
    }
    Ok(next)
}

fn peek_text_pattern<'a>(
    function: BuiltinFunction,
    stack: &'a [Value],
) -> Result<(&'a str, &'a str), AppError> {
    let pattern_offset = match function {
        BuiltinFunction::RegexReplace => 2,
        BuiltinFunction::RegexMatch | BuiltinFunction::RegexCaptures => 1,
        _ => return Err(AppError::Internal),
    };
    let text_offset = pattern_offset + 1;
    let Value::String(pattern) = stack
        .get(
            stack
                .len()
                .checked_sub(pattern_offset)
                .ok_or(AppError::Internal)?,
        )
        .ok_or(AppError::Internal)?
    else {
        return Err(AppError::Internal);
    };
    let Value::String(text) = stack
        .get(
            stack
                .len()
                .checked_sub(text_offset)
                .ok_or(AppError::Internal)?,
        )
        .ok_or(AppError::Internal)?
    else {
        return Err(AppError::Internal);
    };
    validate_sizes(text, pattern)?;
    Ok((text, pattern))
}

fn pop_text_pattern(stack: &mut Vec<Value>) -> Result<(String, String), AppError> {
    let Value::String(pattern) = stack.pop().ok_or(AppError::Internal)? else {
        return Err(AppError::Internal);
    };
    let Value::String(text) = stack.pop().ok_or(AppError::Internal)? else {
        return Err(AppError::Internal);
    };
    validate_sizes(&text, &pattern)?;
    Ok((text, pattern))
}

fn validate_sizes(text: &str, pattern: &str) -> Result<(), AppError> {
    if text.len() > MAX_INPUT_BYTES || pattern.len() > MAX_PATTERN_BYTES {
        return Err(AppError::BadRequest);
    }
    Ok(())
}

fn compile(pattern: &str, text: &str) -> Result<Regex, AppError> {
    validate_sizes(text, pattern)?;
    Regex::new(pattern).map_err(|_| AppError::BadRequest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prepared_eval(function: BuiltinFunction, stack: &mut Vec<Value>) -> Result<Value, AppError> {
        let prepared = prepare(function, stack)?;
        eval_prepared(prepared, stack)
    }

    #[test]
    fn matches_and_replaces() {
        let mut stack = vec![
            Value::String("abc-123".into()),
            Value::String(r"^[a-z]+-\d+$".into()),
        ];
        assert_eq!(
            prepared_eval(BuiltinFunction::RegexMatch, &mut stack).unwrap(),
            Value::Bool(true)
        );

        let mut stack = vec![
            Value::String("a1 b2".into()),
            Value::String(r"\d".into()),
            Value::String("#".into()),
        ];
        assert_eq!(
            prepared_eval(BuiltinFunction::RegexReplace, &mut stack).unwrap(),
            Value::String("a# b#".into())
        );
    }

    #[test]
    fn captures_include_numeric_and_named_keys() {
        let mut stack = vec![
            Value::String("user-42".into()),
            Value::String(r"^(?P<name>[a-z]+)-(?P<id>\d+)$".into()),
        ];
        let Value::StringDict(captures) =
            prepared_eval(BuiltinFunction::RegexCaptures, &mut stack).unwrap()
        else {
            panic!("dict expected");
        };
        assert_eq!(captures.get("0").map(String::as_str), Some("user-42"));
        assert_eq!(captures.get("name").map(String::as_str), Some("user"));
        assert_eq!(captures.get("id").map(String::as_str), Some("42"));
    }

    #[test]
    fn invalid_pattern_is_bad_request() {
        let stack = vec![Value::String("abc".into()), Value::String("(".into())];
        assert!(matches!(
            prepare(BuiltinFunction::RegexMatch, &stack),
            Err(AppError::BadRequest)
        ));
    }
}
