use crate::test_support::*;
use language_core::BinaryOp;

fn budget() -> Budget {
    let limits = ExecutionLimits {
        max_instructions: 1000,
        max_allocated_bytes: 1024 * 1024,
    };
    Budget::new(
        &limits,
        ResourceProfileConfig {
            max_instructions: 1000,
            max_allocated_bytes: 1024 * 1024,
            max_concurrent: 1,
        },
    )
}

#[test]
fn bytecode_executes_integer_precedence_tree() {
    let expr = Expr::Binary {
        left: Box::new(Expr::Int(12)),
        op: BinaryOp::Add,
        right: Box::new(Expr::Binary {
            left: Box::new(Expr::Int(5)),
            op: BinaryOp::Mul,
            right: Box::new(Expr::Int(2)),
        }),
    };
    let mut b = budget();
    let value = eval_expr(&expr, &HashMap::new(), &mut b).unwrap();
    assert_eq!(value, Value::Int(22));
    assert_eq!(b.remaining_request_instructions(), 995);
}

#[test]
fn bytecode_reads_variables_and_model_fields() {
    let mut env = HashMap::new();
    env.insert("n".into(), Value::Int(7));
    env.insert(
        "product".into(),
        Value::Record(HashMap::from([("price".into(), Value::Int(35))])),
    );
    let expr = Expr::Binary {
        left: Box::new(Expr::Variable("n".into())),
        op: BinaryOp::Mul,
        right: Box::new(Expr::Field {
            base: "product".into(),
            field: "price".into(),
        }),
    };
    let mut b = budget();
    assert_eq!(eval_expr(&expr, &env, &mut b).unwrap(), Value::Int(245));
}

#[test]
fn bytecode_preserves_checked_arithmetic_errors() {
    let expr = Expr::Binary {
        left: Box::new(Expr::Int(i64::MAX)),
        op: BinaryOp::Add,
        right: Box::new(Expr::Int(1)),
    };
    let mut b = budget();
    assert_eq!(
        eval_expr(&expr, &HashMap::new(), &mut b),
        Err(AppError::Internal)
    );
}

#[test]
fn logical_and_short_circuits_rhs() {
    let rhs = Expr::Binary {
        left: Box::new(Expr::Binary {
            left: Box::new(Expr::Int(1)),
            op: BinaryOp::Div,
            right: Box::new(Expr::Int(0)),
        }),
        op: BinaryOp::Eq,
        right: Box::new(Expr::Int(0)),
    };
    let expr = Expr::Binary {
        left: Box::new(Expr::Bool(false)),
        op: BinaryOp::LogicalAnd,
        right: Box::new(rhs),
    };
    let mut b = budget();
    assert_eq!(
        eval_expr(&expr, &HashMap::new(), &mut b).unwrap(),
        Value::Bool(false)
    );
}

#[test]
fn logical_or_short_circuits_rhs() {
    let rhs = Expr::Binary {
        left: Box::new(Expr::Binary {
            left: Box::new(Expr::Int(1)),
            op: BinaryOp::Div,
            right: Box::new(Expr::Int(0)),
        }),
        op: BinaryOp::Eq,
        right: Box::new(Expr::Int(0)),
    };
    let expr = Expr::Binary {
        left: Box::new(Expr::Bool(true)),
        op: BinaryOp::LogicalOr,
        right: Box::new(rhs),
    };
    let mut b = budget();
    assert_eq!(
        eval_expr(&expr, &HashMap::new(), &mut b).unwrap(),
        Value::Bool(true)
    );
}

#[test]
fn logical_not_executes_in_bytecode() {
    let mut b = budget();
    assert_eq!(
        eval_expr(
            &Expr::Not(Box::new(Expr::Bool(false))),
            &HashMap::new(),
            &mut b
        )
        .unwrap(),
        Value::Bool(true)
    );
}
