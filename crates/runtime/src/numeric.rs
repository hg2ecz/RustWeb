use crate::execution_context::Budget;
use language_core::{AppError, BinaryOp, F32Value, Value};
use rust_decimal::Decimal;

pub(crate) fn eval_binary_budgeted(
    left: Value,
    op: BinaryOp,
    right: Value,
    budget: &mut Budget,
) -> Result<Value, AppError> {
    if let (Value::String(a), BinaryOp::Add, Value::String(b)) = (&left, op, &right) {
        budget.charge_alloc((a.len() as u64).saturating_add(b.len() as u64))?;
    }
    eval_binary_value(left, op, right)
}

pub(crate) fn eval_binary_value(
    left: Value,
    op: BinaryOp,
    right: Value,
) -> Result<Value, AppError> {
    match (left, op, right) {
        (Value::String(a), BinaryOp::Add, Value::String(b)) => Ok(Value::String(a + &b)),

        (Value::Int(a), BinaryOp::Add, Value::Int(b)) => checked_int(a.checked_add(b)),
        (Value::Int(a), BinaryOp::Sub, Value::Int(b)) => checked_int(a.checked_sub(b)),
        (Value::Int(a), BinaryOp::Mul, Value::Int(b)) => checked_int(a.checked_mul(b)),
        (Value::Int(_), BinaryOp::Div | BinaryOp::Rem, Value::Int(0)) => Err(AppError::Internal),
        (Value::Int(a), BinaryOp::Div, Value::Int(b)) => checked_int(a.checked_div(b)),
        (Value::Int(a), BinaryOp::Rem, Value::Int(b)) => checked_int(a.checked_rem(b)),
        (Value::Int(a), BinaryOp::ShiftLeft, Value::Int(b)) => checked_shift(a, b, true),
        (Value::Int(a), BinaryOp::ShiftRight, Value::Int(b)) => checked_shift(a, b, false),
        (Value::Int(a), BinaryOp::BitAnd, Value::Int(b)) => Ok(Value::Int(a & b)),
        (Value::Int(a), BinaryOp::BitXor, Value::Int(b)) => Ok(Value::Int(a ^ b)),
        (Value::Int(a), BinaryOp::BitOr, Value::Int(b)) => Ok(Value::Int(a | b)),

        (Value::F32(a), BinaryOp::Add, Value::F32(b)) => finite_f32(a.get() + b.get()),
        (Value::F32(a), BinaryOp::Sub, Value::F32(b)) => finite_f32(a.get() - b.get()),
        (Value::F32(a), BinaryOp::Mul, Value::F32(b)) => finite_f32(a.get() * b.get()),
        (Value::F32(_), BinaryOp::Div | BinaryOp::Rem, Value::F32(b)) if b.get() == 0.0 => {
            Err(AppError::Internal)
        }
        (Value::F32(a), BinaryOp::Div, Value::F32(b)) => finite_f32(a.get() / b.get()),
        (Value::F32(a), BinaryOp::Rem, Value::F32(b)) => finite_f32(a.get() % b.get()),

        (Value::Decimal(a), BinaryOp::Add, Value::Decimal(b)) => checked_decimal(a.checked_add(b)),
        (Value::Decimal(a), BinaryOp::Sub, Value::Decimal(b)) => checked_decimal(a.checked_sub(b)),
        (Value::Decimal(a), BinaryOp::Mul, Value::Decimal(b)) => checked_decimal(a.checked_mul(b)),
        (Value::Decimal(_), BinaryOp::Div | BinaryOp::Rem, Value::Decimal(b))
            if b == Decimal::ZERO =>
        {
            Err(AppError::Internal)
        }
        (Value::Decimal(a), BinaryOp::Div, Value::Decimal(b)) => checked_decimal(a.checked_div(b)),
        (Value::Decimal(a), BinaryOp::Rem, Value::Decimal(b)) => Ok(Value::Decimal(a % b)),

        (Value::Bool(a), BinaryOp::LogicalAnd, Value::Bool(b)) => Ok(Value::Bool(a && b)),
        (Value::Bool(a), BinaryOp::LogicalOr, Value::Bool(b)) => Ok(Value::Bool(a || b)),

        (Value::Int(a), BinaryOp::Lt, Value::Int(b)) => Ok(Value::Bool(a < b)),
        (Value::Int(a), BinaryOp::Le, Value::Int(b)) => Ok(Value::Bool(a <= b)),
        (Value::Int(a), BinaryOp::Gt, Value::Int(b)) => Ok(Value::Bool(a > b)),
        (Value::Int(a), BinaryOp::Ge, Value::Int(b)) => Ok(Value::Bool(a >= b)),
        (Value::Int(a), BinaryOp::Eq, Value::Int(b)) => Ok(Value::Bool(a == b)),
        (Value::Int(a), BinaryOp::Ne, Value::Int(b)) => Ok(Value::Bool(a != b)),
        (Value::F32(a), BinaryOp::Lt, Value::F32(b)) => Ok(Value::Bool(a.get() < b.get())),
        (Value::F32(a), BinaryOp::Le, Value::F32(b)) => Ok(Value::Bool(a.get() <= b.get())),
        (Value::F32(a), BinaryOp::Gt, Value::F32(b)) => Ok(Value::Bool(a.get() > b.get())),
        (Value::F32(a), BinaryOp::Ge, Value::F32(b)) => Ok(Value::Bool(a.get() >= b.get())),
        (Value::F32(a), BinaryOp::Eq, Value::F32(b)) => Ok(Value::Bool(a.get() == b.get())),
        (Value::F32(a), BinaryOp::Ne, Value::F32(b)) => Ok(Value::Bool(a.get() != b.get())),
        (Value::Decimal(a), BinaryOp::Lt, Value::Decimal(b)) => Ok(Value::Bool(a < b)),
        (Value::Decimal(a), BinaryOp::Le, Value::Decimal(b)) => Ok(Value::Bool(a <= b)),
        (Value::Decimal(a), BinaryOp::Gt, Value::Decimal(b)) => Ok(Value::Bool(a > b)),
        (Value::Decimal(a), BinaryOp::Ge, Value::Decimal(b)) => Ok(Value::Bool(a >= b)),
        (Value::Decimal(a), BinaryOp::Eq, Value::Decimal(b)) => Ok(Value::Bool(a == b)),
        (Value::Decimal(a), BinaryOp::Ne, Value::Decimal(b)) => Ok(Value::Bool(a != b)),
        (Value::String(a), BinaryOp::Eq, Value::String(b)) => Ok(Value::Bool(a == b)),
        (Value::String(a), BinaryOp::Ne, Value::String(b)) => Ok(Value::Bool(a != b)),
        (Value::Bool(a), BinaryOp::Eq, Value::Bool(b)) => Ok(Value::Bool(a == b)),
        (Value::Bool(a), BinaryOp::Ne, Value::Bool(b)) => Ok(Value::Bool(a != b)),
        _ => Err(AppError::Internal),
    }
}

fn checked_int(value: Option<i64>) -> Result<Value, AppError> {
    value.map(Value::Int).ok_or(AppError::Internal)
}

fn checked_decimal(value: Option<Decimal>) -> Result<Value, AppError> {
    value.map(Value::Decimal).ok_or(AppError::Internal)
}

fn checked_shift(value: i64, amount: i64, left: bool) -> Result<Value, AppError> {
    let amount = u32::try_from(amount).map_err(|_| AppError::BadRequest)?;
    let shifted = if left {
        value.checked_shl(amount)
    } else {
        value.checked_shr(amount)
    };
    shifted.map(Value::Int).ok_or(AppError::BadRequest)
}

fn finite_f32(value: f32) -> Result<Value, AppError> {
    F32Value::new(value)
        .map(Value::F32)
        .ok_or(AppError::Internal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remainder_shift_and_bitwise_int_operations_are_checked() {
        assert_eq!(
            eval_binary_value(Value::Int(17), BinaryOp::Rem, Value::Int(5)).unwrap(),
            Value::Int(2)
        );
        assert_eq!(
            eval_binary_value(Value::Int(3), BinaryOp::ShiftLeft, Value::Int(2)).unwrap(),
            Value::Int(12)
        );
        assert_eq!(
            eval_binary_value(Value::Int(0b1100), BinaryOp::BitAnd, Value::Int(0b1010)).unwrap(),
            Value::Int(0b1000)
        );
        assert_eq!(
            eval_binary_value(Value::Int(1), BinaryOp::ShiftLeft, Value::Int(64)),
            Err(AppError::BadRequest)
        );
    }
}
