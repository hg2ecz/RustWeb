use crate::diagnostics::CompileError;
use crate::expression::infer_expr_type;
use crate::handler_types::StaticType;
use language_core::{BuiltinFunction, Expr, Program, ValueType};
use std::collections::HashMap;

pub(super) fn handles(function: BuiltinFunction) -> bool {
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

pub(super) fn infer(
    function: BuiltinFunction,
    args: &[Expr],
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<ValueType, CompileError> {
    match function {
        BuiltinFunction::StringLen => {
            require_types(function, args, &[ValueType::String], known, program)?;
            Ok(ValueType::Int)
        }
        BuiltinFunction::Trim
        | BuiltinFunction::TrimStart
        | BuiltinFunction::TrimEnd
        | BuiltinFunction::Lower
        | BuiltinFunction::Upper => {
            require_types(function, args, &[ValueType::String], known, program)?;
            Ok(ValueType::String)
        }
        BuiltinFunction::Contains | BuiltinFunction::StartsWith | BuiltinFunction::EndsWith => {
            require_types(
                function,
                args,
                &[ValueType::String, ValueType::String],
                known,
                program,
            )?;
            Ok(ValueType::Bool)
        }
        BuiltinFunction::Replace => {
            require_types(
                function,
                args,
                &[ValueType::String, ValueType::String, ValueType::String],
                known,
                program,
            )?;
            Ok(ValueType::String)
        }
        BuiltinFunction::Split => {
            require_types(
                function,
                args,
                &[ValueType::String, ValueType::String],
                known,
                program,
            )?;
            Ok(ValueType::StringList)
        }
        BuiltinFunction::Substring => {
            if !matches!(args.len(), 2 | 3) {
                return Err(signature_error(function, "String, Int[, Int]"));
            }
            let expected = if args.len() == 2 {
                vec![ValueType::String, ValueType::Int]
            } else {
                vec![ValueType::String, ValueType::Int, ValueType::Int]
            };
            require_types(function, args, &expected, known, program)?;
            Ok(ValueType::String)
        }
        BuiltinFunction::IndexOf | BuiltinFunction::LastIndexOf => {
            require_types(
                function,
                args,
                &[ValueType::String, ValueType::String],
                known,
                program,
            )?;
            Ok(ValueType::Int)
        }
        BuiltinFunction::CharAt => {
            require_types(
                function,
                args,
                &[ValueType::String, ValueType::Int],
                known,
                program,
            )?;
            Ok(ValueType::String)
        }
        BuiltinFunction::Repeat => {
            require_types(
                function,
                args,
                &[ValueType::String, ValueType::Int],
                known,
                program,
            )?;
            Ok(ValueType::String)
        }
        _ => Err(CompileError::Syntax(
            "internal: non-string builtin routed to string type checker".into(),
        )),
    }
}

fn require_types(
    function: BuiltinFunction,
    args: &[Expr],
    expected: &[ValueType],
    known: &HashMap<String, StaticType>,
    program: &Program,
) -> Result<(), CompileError> {
    if args.len() != expected.len() {
        return Err(signature_error(function, &format_types(expected)));
    }
    for (arg, expected_ty) in args.iter().zip(expected) {
        if infer_expr_type(arg, known, program)? != *expected_ty {
            return Err(signature_error(function, &format_types(expected)));
        }
    }
    Ok(())
}

fn format_types(types: &[ValueType]) -> String {
    types
        .iter()
        .map(|ty| format!("{ty:?}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn signature_error(function: BuiltinFunction, expected: &str) -> CompileError {
    CompileError::Syntax(format!(
        "{}(...) requires ({expected})",
        function.source_name()
    ))
}
