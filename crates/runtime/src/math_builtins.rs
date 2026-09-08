use language_core::{AppError, BuiltinFunction, F32Value, Value};
use std::sync::OnceLock;
use std::time::Instant;

pub(crate) fn handles(function: BuiltinFunction) -> bool {
    matches!(
        function,
        BuiltinFunction::Sin
            | BuiltinFunction::Cos
            | BuiltinFunction::Sqrt
            | BuiltinFunction::Abs
            | BuiltinFunction::Ln
            | BuiltinFunction::Log10
            | BuiltinFunction::Log
            | BuiltinFunction::Exp
            | BuiltinFunction::Pow
            | BuiltinFunction::Round
            | BuiltinFunction::Floor
            | BuiltinFunction::Ceil
            | BuiltinFunction::MonotonicNanos
            | BuiltinFunction::ToF32
    )
}

pub(crate) fn eval(function: BuiltinFunction, stack: &mut Vec<Value>) -> Result<Value, AppError> {
    match function {
        BuiltinFunction::Sin => unary_f32(stack, f32::sin),
        BuiltinFunction::Cos => unary_f32(stack, f32::cos),
        BuiltinFunction::Sqrt => {
            unary_f32(
                stack,
                |value| if value < 0.0 { f32::NAN } else { value.sqrt() },
            )
        }
        BuiltinFunction::Ln => unary_f32(stack, f32::ln),
        BuiltinFunction::Log10 => unary_f32(stack, f32::log10),
        BuiltinFunction::Exp => unary_f32(stack, f32::exp),
        BuiltinFunction::Round => unary_f32(stack, f32::round),
        BuiltinFunction::Floor => unary_f32(stack, f32::floor),
        BuiltinFunction::Ceil => unary_f32(stack, f32::ceil),
        BuiltinFunction::Log => binary_f32(stack, |value, base| value.log(base)),
        BuiltinFunction::Pow => binary_f32(stack, f32::powf),
        BuiltinFunction::Abs => abs(stack),
        BuiltinFunction::MonotonicNanos => Ok(Value::Int(monotonic_nanos())),
        BuiltinFunction::ToF32 => {
            let Value::Int(value) = stack.pop().ok_or(AppError::Internal)? else {
                return Err(AppError::Internal);
            };
            finite_f32(value as f32)
        }
        _ => Err(AppError::Internal),
    }
}

fn abs(stack: &mut Vec<Value>) -> Result<Value, AppError> {
    match stack.pop().ok_or(AppError::Internal)? {
        Value::Int(value) => value
            .checked_abs()
            .map(Value::Int)
            .ok_or(AppError::Internal),
        Value::F32(value) => finite_f32(value.get().abs()),
        _ => Err(AppError::Internal),
    }
}

fn unary_f32(
    stack: &mut Vec<Value>,
    operation: impl FnOnce(f32) -> f32,
) -> Result<Value, AppError> {
    let Value::F32(value) = stack.pop().ok_or(AppError::Internal)? else {
        return Err(AppError::Internal);
    };
    finite_f32(operation(value.get()))
}

fn binary_f32(
    stack: &mut Vec<Value>,
    operation: impl FnOnce(f32, f32) -> f32,
) -> Result<Value, AppError> {
    let Value::F32(right) = stack.pop().ok_or(AppError::Internal)? else {
        return Err(AppError::Internal);
    };
    let Value::F32(left) = stack.pop().ok_or(AppError::Internal)? else {
        return Err(AppError::Internal);
    };
    finite_f32(operation(left.get(), right.get()))
}

fn finite_f32(value: f32) -> Result<Value, AppError> {
    F32Value::new(value)
        .map(Value::F32)
        .ok_or(AppError::Internal)
}

fn monotonic_nanos() -> i64 {
    static ORIGIN: OnceLock<Instant> = OnceLock::new();
    let nanos = ORIGIN.get_or_init(Instant::now).elapsed().as_nanos();
    nanos.min(i64::MAX as u128) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f32_value(value: f32) -> Value {
        Value::F32(F32Value::new(value).unwrap())
    }

    #[test]
    fn logarithm_exponential_and_rounding_are_finite() {
        let mut stack = vec![f32_value(8.0), f32_value(2.0)];
        let Value::F32(result) = eval(BuiltinFunction::Log, &mut stack).unwrap() else {
            panic!("F32 expected")
        };
        assert_eq!(result.get(), 3.0);

        let mut stack = vec![f32_value(2.0), f32_value(3.0)];
        let Value::F32(result) = eval(BuiltinFunction::Pow, &mut stack).unwrap() else {
            panic!("F32 expected")
        };
        assert_eq!(result.get(), 8.0);

        let mut stack = vec![f32_value(2.6)];
        let Value::F32(result) = eval(BuiltinFunction::Round, &mut stack).unwrap() else {
            panic!("F32 expected")
        };
        assert_eq!(result.get(), 3.0);
    }

    #[test]
    fn invalid_logarithm_is_rejected() {
        let mut stack = vec![f32_value(-1.0)];
        assert_eq!(
            eval(BuiltinFunction::Ln, &mut stack),
            Err(AppError::Internal)
        );
    }
}
