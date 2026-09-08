use crate::arrays;
use crate::collections;
use crate::execution_context::Budget;
use crate::vm::eval_expr;
use language_core::{AppError, ComputeStatement, Expr, Value};
use std::collections::HashMap;

pub(crate) fn assign(
    name: &str,
    expr: &Expr,
    env: &mut HashMap<String, Value>,
    budget: &mut Budget,
) -> Result<(), AppError> {
    if !env.contains_key(name) {
        return Err(AppError::Internal);
    }
    let value = eval_expr(expr, env, budget)?;
    budget.charge_value(&value)?;
    env.insert(name.to_string(), value);
    Ok(())
}

pub(crate) fn set_f32_array(
    array: &str,
    index: &Expr,
    value: &Expr,
    env: &mut HashMap<String, Value>,
    budget: &mut Budget,
) -> Result<(), AppError> {
    let Value::Int(index) = eval_expr(index, env, budget)? else {
        return Err(AppError::Internal);
    };
    let Value::F32(value) = eval_expr(value, env, budget)? else {
        return Err(AppError::Internal);
    };
    arrays::set_f32(env, array, index, value)
}

pub(crate) fn set_string_dict(
    dict: &str,
    key: &Expr,
    value: &Expr,
    env: &mut HashMap<String, Value>,
    budget: &mut Budget,
) -> Result<(), AppError> {
    let Value::String(key) = eval_expr(key, env, budget)? else {
        return Err(AppError::Internal);
    };
    let Value::String(value) = eval_expr(value, env, budget)? else {
        return Err(AppError::Internal);
    };
    budget.charge_alloc((key.len() + value.len() + 32) as u64)?;
    collections::set_string_dict(env, dict, key, value)
}

pub(crate) fn execute_while(
    condition: &Expr,
    statements: &[ComputeStatement],
    env: &mut HashMap<String, Value>,
    budget: &mut Budget,
) -> Result<(), AppError> {
    loop {
        budget.charge(1)?;
        match eval_expr(condition, env, budget)? {
            Value::Bool(true) => execute_compute(statements, env, budget)?,
            Value::Bool(false) => return Ok(()),
            _ => return Err(AppError::Internal),
        }
    }
}

pub(crate) fn execute_if(
    condition: &Expr,
    statements: &[ComputeStatement],
    env: &mut HashMap<String, Value>,
    budget: &mut Budget,
) -> Result<(), AppError> {
    budget.charge(1)?;
    match eval_expr(condition, env, budget)? {
        Value::Bool(true) => execute_compute(statements, env, budget),
        Value::Bool(false) => Ok(()),
        _ => Err(AppError::Internal),
    }
}

fn execute_compute(
    statements: &[ComputeStatement],
    env: &mut HashMap<String, Value>,
    budget: &mut Budget,
) -> Result<(), AppError> {
    for statement in statements {
        budget.charge(1)?;
        match statement {
            ComputeStatement::Let { name, expr } => {
                let value = eval_expr(expr, env, budget)?;
                budget.charge_value(&value)?;
                env.insert(name.clone(), value);
            }
            ComputeStatement::Set { name, expr } => assign(name, expr, env, budget)?,
            ComputeStatement::F32ArraySet {
                array,
                index,
                value,
            } => set_f32_array(array, index, value, env, budget)?,
            ComputeStatement::StringDictSet { dict, key, value } => {
                set_string_dict(dict, key, value, env, budget)?
            }
            ComputeStatement::While {
                condition,
                statements,
            } => execute_while(condition, statements, env, budget)?,
            ComputeStatement::If {
                condition,
                statements,
            } => execute_if(condition, statements, env, budget)?,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExecutionLimits, ResourceProfileConfig};
    use language_core::{BinaryOp, F32Value};

    fn budget() -> Budget {
        Budget::new(
            &ExecutionLimits {
                max_instructions: 10_000,
                max_allocated_bytes: 1024 * 1024,
            },
            ResourceProfileConfig {
                max_instructions: 10_000,
                max_allocated_bytes: 1024 * 1024,
                max_concurrent: 1,
            },
        )
    }

    #[test]
    fn while_updates_scalar_until_condition_is_false() {
        let condition = Expr::Binary {
            left: Box::new(Expr::Variable("i".into())),
            op: BinaryOp::Lt,
            right: Box::new(Expr::Int(4)),
        };
        let statements = vec![ComputeStatement::Set {
            name: "i".into(),
            expr: Expr::Binary {
                left: Box::new(Expr::Variable("i".into())),
                op: BinaryOp::Add,
                right: Box::new(Expr::Int(1)),
            },
        }];
        let mut env = HashMap::from([("i".into(), Value::Int(0))]);
        execute_while(&condition, &statements, &mut env, &mut budget()).unwrap();
        assert_eq!(env.get("i"), Some(&Value::Int(4)));
    }

    #[test]
    fn while_is_instruction_budgeted() {
        let condition = Expr::Bool(true);
        let statements = vec![ComputeStatement::Set {
            name: "x".into(),
            expr: Expr::F32(F32Value::new(1.0).unwrap()),
        }];
        let mut env = HashMap::from([("x".into(), Value::F32(F32Value::new(0.0).unwrap()))]);
        let mut b = Budget::new(
            &ExecutionLimits {
                max_instructions: 5,
                max_allocated_bytes: 1024,
            },
            ResourceProfileConfig {
                max_instructions: 5,
                max_allocated_bytes: 1024,
                max_concurrent: 1,
            },
        );
        assert!(matches!(
            execute_while(&condition, &statements, &mut env, &mut b),
            Err(AppError::InstructionLimit)
        ));
    }
}

#[cfg(test)]
mod if_tests {
    use super::*;
    use crate::{ExecutionLimits, ResourceProfileConfig};

    #[test]
    fn if_executes_only_when_condition_is_true() {
        let statements = vec![ComputeStatement::Set {
            name: "i".into(),
            expr: Expr::Int(7),
        }];
        let mut env = HashMap::from([("i".into(), Value::Int(1))]);
        let make_budget = || {
            Budget::new(
                &ExecutionLimits {
                    max_instructions: 100,
                    max_allocated_bytes: 1024,
                },
                ResourceProfileConfig {
                    max_instructions: 100,
                    max_allocated_bytes: 1024,
                    max_concurrent: 1,
                },
            )
        };
        execute_if(
            &Expr::Bool(false),
            &statements,
            &mut env,
            &mut make_budget(),
        )
        .unwrap();
        assert_eq!(env.get("i"), Some(&Value::Int(1)));
        execute_if(&Expr::Bool(true), &statements, &mut env, &mut make_budget()).unwrap();
        assert_eq!(env.get("i"), Some(&Value::Int(7)));
    }
}
