use language_core::{BinaryOp, BuiltinFunction, Expr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Op {
    PushString(String),
    PushInt(i64),
    PushF32(language_core::F32Value),
    NewF32Array,
    LoadCollectionIndex(String),
    LoadCollectionLen(String),
    PushBool(bool),
    PushEnum { enum_id: u16, variant: String },
    LoadVariable(String),
    LoadField { base: String, field: String },
    Slugify,
    Builtin(BuiltinFunction),
    Not,
    Pop,
    JumpIfFalse(usize),
    JumpIfTrue(usize),
    Binary(BinaryOp),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Program {
    pub(crate) ops: Vec<Op>,
}

pub(crate) fn compile(expr: &Expr) -> Program {
    let mut ops = Vec::new();
    emit(expr, &mut ops);
    Program { ops }
}

fn emit(expr: &Expr, out: &mut Vec<Op>) {
    match expr {
        Expr::String(value) => out.push(Op::PushString(value.clone())),
        Expr::Int(value) => out.push(Op::PushInt(*value)),
        Expr::F32(value) => out.push(Op::PushF32(*value)),
        Expr::F32ArrayNew { len, fill } => {
            emit(len, out);
            emit(fill, out);
            out.push(Op::NewF32Array);
        }
        Expr::CollectionIndex { collection, index } => {
            emit(index, out);
            out.push(Op::LoadCollectionIndex(collection.clone()));
        }
        Expr::CollectionLen { collection } => out.push(Op::LoadCollectionLen(collection.clone())),
        Expr::Bool(value) => out.push(Op::PushBool(*value)),
        Expr::EnumLiteral { enum_id, variant } => out.push(Op::PushEnum {
            enum_id: *enum_id,
            variant: variant.clone(),
        }),
        Expr::Variable(name) => out.push(Op::LoadVariable(name.clone())),
        Expr::Field { base, field } => out.push(Op::LoadField {
            base: base.clone(),
            field: field.clone(),
        }),
        Expr::Slugify(inner) => {
            emit(inner, out);
            out.push(Op::Slugify);
        }
        Expr::Builtin { function, args } => {
            for arg in args {
                emit(arg, out);
            }
            out.push(Op::Builtin(*function));
        }
        Expr::Not(inner) => {
            emit(inner, out);
            out.push(Op::Not);
        }
        Expr::Binary {
            left,
            op: BinaryOp::LogicalAnd,
            right,
        } => {
            emit_short_circuit(left, right, out, false);
        }
        Expr::Binary {
            left,
            op: BinaryOp::LogicalOr,
            right,
        } => {
            emit_short_circuit(left, right, out, true);
        }
        Expr::Binary { left, op, right } => {
            emit(left, out);
            emit(right, out);
            out.push(Op::Binary(*op));
        }
    }
}

fn emit_short_circuit(left: &Expr, right: &Expr, out: &mut Vec<Op>, jump_on: bool) {
    emit(left, out);
    let jump_index = out.len();
    out.push(if jump_on {
        Op::JumpIfTrue(usize::MAX)
    } else {
        Op::JumpIfFalse(usize::MAX)
    });
    out.push(Op::Pop);
    emit(right, out);
    let target = out.len();
    out[jump_index] = if jump_on {
        Op::JumpIfTrue(target)
    } else {
        Op::JumpIfFalse(target)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_f32_literal() {
        let value = language_core::F32Value::new(1.25).unwrap();
        assert_eq!(compile(&Expr::F32(value)).ops, vec![Op::PushF32(value)]);
    }

    #[test]
    fn compiles_expression_to_postfix_stack_program() {
        let expr = Expr::Binary {
            left: Box::new(Expr::Int(12)),
            op: BinaryOp::Add,
            right: Box::new(Expr::Binary {
                left: Box::new(Expr::Int(5)),
                op: BinaryOp::Mul,
                right: Box::new(Expr::Int(2)),
            }),
        };
        assert_eq!(
            compile(&expr).ops,
            vec![
                Op::PushInt(12),
                Op::PushInt(5),
                Op::PushInt(2),
                Op::Binary(BinaryOp::Mul),
                Op::Binary(BinaryOp::Add),
            ]
        );
    }
}
