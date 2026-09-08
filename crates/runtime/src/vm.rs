use crate::execution_context::Budget;
use crate::{
    arrays, builtins, bytecode, collections, numeric,
    scalars::{is_canonical_slug, slugify_ascii},
};
use language_core::{AppError, Expr, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

const MAX_EXPR_BYTECODE_CACHE_ENTRIES: usize = 8192;

fn expression_bytecode(expr: &Expr) -> Result<Arc<bytecode::Program>, AppError> {
    static CACHE: OnceLock<Mutex<HashMap<Expr, Arc<bytecode::Program>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(program) = cache
        .lock()
        .map_err(|_| AppError::Internal)?
        .get(expr)
        .cloned()
    {
        return Ok(program);
    }

    let compiled = Arc::new(bytecode::compile(expr));
    let mut guard = cache.lock().map_err(|_| AppError::Internal)?;
    if guard.len() >= MAX_EXPR_BYTECODE_CACHE_ENTRIES {
        guard.clear();
    }
    Ok(guard
        .entry(expr.clone())
        .or_insert_with(|| Arc::clone(&compiled))
        .clone())
}

pub(crate) fn eval_expr(
    e: &Expr,
    env: &HashMap<String, Value>,
    budget: &mut Budget,
) -> Result<Value, AppError> {
    let program = expression_bytecode(e)?;
    eval_bytecode(&program, env, budget)
}

fn eval_bytecode(
    program: &bytecode::Program,
    env: &HashMap<String, Value>,
    budget: &mut Budget,
) -> Result<Value, AppError> {
    let mut stack = Vec::with_capacity(8);
    let mut ip = 0usize;
    while ip < program.ops.len() {
        let op = &program.ops[ip];
        budget.charge(1)?;
        match op {
            bytecode::Op::PushString(v) => stack.push(Value::String(v.clone())),
            bytecode::Op::PushInt(v) => stack.push(Value::Int(*v)),
            bytecode::Op::PushF32(v) => stack.push(Value::F32(*v)),
            bytecode::Op::NewF32Array => {
                let Value::F32(fill) = stack.pop().ok_or(AppError::Internal)? else {
                    return Err(AppError::Internal);
                };
                let Value::Int(len) = stack.pop().ok_or(AppError::Internal)? else {
                    return Err(AppError::Internal);
                };
                budget.charge_alloc((len.max(0) as u64).saturating_mul(4))?;
                stack.push(arrays::new_f32(len, fill)?);
            }
            bytecode::Op::LoadCollectionIndex(name) => {
                let key = stack.pop().ok_or(AppError::Internal)?;
                stack.push(collections::get(env, name, key)?);
            }
            bytecode::Op::LoadCollectionLen(name) => {
                stack.push(Value::Int(collections::len(env, name)?))
            }
            bytecode::Op::PushBool(v) => stack.push(Value::Bool(*v)),
            bytecode::Op::PushEnum { enum_id, variant } => stack.push(Value::Enum {
                enum_id: *enum_id,
                variant: variant.clone(),
            }),
            bytecode::Op::LoadVariable(name) => {
                stack.push(env.get(name).cloned().ok_or(AppError::Internal)?)
            }
            bytecode::Op::LoadField { base, field } => match env.get(base) {
                Some(Value::Record(fields)) => {
                    stack.push(fields.get(field).cloned().ok_or(AppError::Internal)?)
                }
                _ => return Err(AppError::Internal),
            },
            bytecode::Op::Slugify => {
                let Value::String(text) = stack.pop().ok_or(AppError::Internal)? else {
                    return Err(AppError::Internal);
                };
                let slug = slugify_ascii(&text);
                if !is_canonical_slug(&slug) {
                    return Err(AppError::BadRequest);
                }
                budget.charge_alloc(slug.len() as u64)?;
                stack.push(Value::String(slug));
            }
            bytecode::Op::Builtin(function) => {
                budget.charge(function.metadata().instruction_cost)?;
                let prepared = builtins::prepare(*function, &stack)?;
                budget.charge_alloc(prepared.result_alloc())?;
                let value = builtins::eval_prepared(prepared, &mut stack)?;
                stack.push(value);
            }
            bytecode::Op::Not => {
                let Value::Bool(value) = stack.pop().ok_or(AppError::Internal)? else {
                    return Err(AppError::Internal);
                };
                stack.push(Value::Bool(!value));
            }
            bytecode::Op::Pop => {
                stack.pop().ok_or(AppError::Internal)?;
            }
            bytecode::Op::JumpIfFalse(target) => {
                let Some(Value::Bool(value)) = stack.last() else {
                    return Err(AppError::Internal);
                };
                if !value {
                    ip = *target;
                    continue;
                }
            }
            bytecode::Op::JumpIfTrue(target) => {
                let Some(Value::Bool(value)) = stack.last() else {
                    return Err(AppError::Internal);
                };
                if *value {
                    ip = *target;
                    continue;
                }
            }
            bytecode::Op::Binary(op) => {
                let right = stack.pop().ok_or(AppError::Internal)?;
                let left = stack.pop().ok_or(AppError::Internal)?;
                stack.push(numeric::eval_binary_budgeted(left, *op, right, budget)?);
            }
        }
        ip += 1;
    }
    if stack.len() != 1 {
        return Err(AppError::Internal);
    }
    stack.pop().ok_or(AppError::Internal)
}
