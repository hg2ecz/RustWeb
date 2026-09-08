use language_core::{AppError, BuiltinExecutionKind, BuiltinFunction, Value};
use std::collections::BTreeMap;

pub(crate) enum PreparedBuiltin {
    Simple {
        function: BuiltinFunction,
        result_alloc: u64,
    },
    Regex(super::regex_builtins::PreparedRegexBuiltin),
}

impl PreparedBuiltin {
    pub(crate) fn result_alloc(&self) -> u64 {
        match self {
            Self::Simple { result_alloc, .. } => *result_alloc,
            Self::Regex(prepared) => prepared.result_alloc(),
        }
    }
}

pub(crate) fn prepare(
    function: BuiltinFunction,
    stack: &[Value],
) -> Result<PreparedBuiltin, AppError> {
    match function.metadata().execution_kind {
        BuiltinExecutionKind::Regex => {
            super::regex_builtins::prepare(function, stack).map(PreparedBuiltin::Regex)
        }
        BuiltinExecutionKind::Simple => Ok(PreparedBuiltin::Simple {
            function,
            result_alloc: estimated_simple_result_alloc(function, stack)?,
        }),
    }
}

pub(crate) fn eval_prepared(
    prepared: PreparedBuiltin,
    stack: &mut Vec<Value>,
) -> Result<Value, AppError> {
    match prepared {
        PreparedBuiltin::Simple { function, .. } => eval_simple(function, stack),
        PreparedBuiltin::Regex(prepared) => super::regex_builtins::eval_prepared(prepared, stack),
    }
}

fn eval_simple(function: BuiltinFunction, stack: &mut Vec<Value>) -> Result<Value, AppError> {
    if super::math_builtins::handles(function) {
        return super::math_builtins::eval(function, stack);
    }
    if super::string_builtins::handles(function) {
        return super::string_builtins::eval(function, stack);
    }
    if super::credential_builtins::handles(function) {
        return super::credential_builtins::eval(function, stack);
    }

    match function {
        BuiltinFunction::DictNew => Ok(Value::StringDict(BTreeMap::new())),
        BuiltinFunction::ContainsKey => {
            let key = pop_string(stack)?;
            let Value::StringDict(dict) = stack.pop().ok_or(AppError::Internal)? else {
                return Err(AppError::Internal);
            };
            Ok(Value::Bool(dict.contains_key(&key)))
        }
        BuiltinFunction::RemoveKey => {
            let key = pop_string(stack)?;
            let Value::StringDict(mut dict) = stack.pop().ok_or(AppError::Internal)? else {
                return Err(AppError::Internal);
            };
            dict.remove(&key);
            Ok(Value::StringDict(dict))
        }
        BuiltinFunction::Redact => {
            let _ = stack.pop().ok_or(AppError::Internal)?;
            Ok(Value::String("[redacted]".into()))
        }
        BuiltinFunction::RegexMatch
        | BuiltinFunction::RegexReplace
        | BuiltinFunction::RegexCaptures => Err(AppError::Internal),
        _ => Err(AppError::Internal),
    }
}

fn estimated_simple_result_alloc(
    function: BuiltinFunction,
    stack: &[Value],
) -> Result<u64, AppError> {
    if super::math_builtins::handles(function) {
        return Ok(0);
    }
    if super::string_builtins::handles(function) {
        return super::string_builtins::estimated_result_alloc(function, stack);
    }
    if super::credential_builtins::handles(function) {
        return super::credential_builtins::estimated_result_alloc(function, stack);
    }

    match function {
        BuiltinFunction::DictNew | BuiltinFunction::ContainsKey => Ok(0),
        BuiltinFunction::Redact => Ok("[redacted]".len() as u64),
        BuiltinFunction::RemoveKey => match stack
            .get(stack.len().checked_sub(2).ok_or(AppError::Internal)?)
        {
            Some(Value::StringDict(dict)) => Ok(super::memory::estimate_string_dict_bytes(dict)),
            _ => Err(AppError::Internal),
        },
        BuiltinFunction::RegexMatch
        | BuiltinFunction::RegexReplace
        | BuiltinFunction::RegexCaptures => Err(AppError::Internal),
        _ => Err(AppError::Internal),
    }
}

fn pop_string(stack: &mut Vec<Value>) -> Result<String, AppError> {
    let Value::String(value) = stack.pop().ok_or(AppError::Internal)? else {
        return Err(AppError::Internal);
    };
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dict_builtins_stay_in_generic_dispatcher() {
        let mut stack = Vec::new();
        assert_eq!(
            eval_simple(BuiltinFunction::DictNew, &mut stack).unwrap(),
            Value::StringDict(BTreeMap::new())
        );
    }

    #[test]
    fn redact_discards_the_original_value() {
        let mut stack = vec![Value::String("top-secret".into())];
        assert_eq!(
            eval_simple(BuiltinFunction::Redact, &mut stack).unwrap(),
            Value::String("[redacted]".into())
        );
    }
}
