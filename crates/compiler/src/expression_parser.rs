use crate::module_namespace::resolve;
use crate::{CompileError, builtin_registry, lexer};
use language_core::{BinaryOp, Expr, Program};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExprToken {
    String(String),
    Int(i64),
    F32(language_core::F32Value),
    Ident(String),
    Dot,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    ShiftLeft,
    ShiftRight,
    Amp,
    Caret,
    Pipe,
    AndAnd,
    OrOr,
    Bang,
    Lt,
    Le,
    Gt,
    Ge,
    EqEq,
    Ne,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
}

pub(super) fn parse_expr_in_namespace(
    input: &str,
    namespace: &str,
    program: &Program,
) -> Result<Expr, CompileError> {
    let tokens = lexer::lex_expr(input)?;
    let mut p = ExprParser {
        tokens: &tokens,
        pos: 0,
        namespace,
        program,
    };
    let e = p.parse_logical_or()?;
    if p.pos != tokens.len() {
        return Err(CompileError::Syntax(format!(
            "unexpected token in expression `{input}`"
        )));
    }
    Ok(e)
}

fn binary(left: Expr, op: BinaryOp, right: Expr) -> Expr {
    Expr::Binary {
        left: Box::new(left),
        op,
        right: Box::new(right),
    }
}

struct ExprParser<'a> {
    tokens: &'a [ExprToken],
    pos: usize,
    namespace: &'a str,
    program: &'a Program,
}
impl ExprParser<'_> {
    fn parse_logical_or(&mut self) -> Result<Expr, CompileError> {
        self.parse_left_associative(
            Self::parse_logical_and,
            &[(ExprToken::OrOr, BinaryOp::LogicalOr)],
        )
    }

    fn parse_logical_and(&mut self) -> Result<Expr, CompileError> {
        self.parse_left_associative(
            Self::parse_bit_or,
            &[(ExprToken::AndAnd, BinaryOp::LogicalAnd)],
        )
    }

    fn parse_bit_or(&mut self) -> Result<Expr, CompileError> {
        self.parse_left_associative(Self::parse_bit_xor, &[(ExprToken::Pipe, BinaryOp::BitOr)])
    }

    fn parse_bit_xor(&mut self) -> Result<Expr, CompileError> {
        self.parse_left_associative(Self::parse_bit_and, &[(ExprToken::Caret, BinaryOp::BitXor)])
    }

    fn parse_bit_and(&mut self) -> Result<Expr, CompileError> {
        self.parse_left_associative(Self::parse_compare, &[(ExprToken::Amp, BinaryOp::BitAnd)])
    }

    fn parse_compare(&mut self) -> Result<Expr, CompileError> {
        let mut left = self.parse_shift()?;
        let op = match self.tokens.get(self.pos) {
            Some(ExprToken::Lt) => Some(BinaryOp::Lt),
            Some(ExprToken::Le) => Some(BinaryOp::Le),
            Some(ExprToken::Gt) => Some(BinaryOp::Gt),
            Some(ExprToken::Ge) => Some(BinaryOp::Ge),
            Some(ExprToken::EqEq) => Some(BinaryOp::Eq),
            Some(ExprToken::Ne) => Some(BinaryOp::Ne),
            _ => None,
        };
        if let Some(op) = op {
            self.pos += 1;
            let right = self.parse_shift()?;
            left = binary(left, op, right);
            if matches!(
                self.tokens.get(self.pos),
                Some(
                    ExprToken::Lt
                        | ExprToken::Le
                        | ExprToken::Gt
                        | ExprToken::Ge
                        | ExprToken::EqEq
                        | ExprToken::Ne
                )
            ) {
                return Err(CompileError::Syntax(
                    "chained comparisons are not supported".into(),
                ));
            }
        }
        Ok(left)
    }

    fn parse_shift(&mut self) -> Result<Expr, CompileError> {
        self.parse_left_associative(
            Self::parse_add_sub,
            &[
                (ExprToken::ShiftLeft, BinaryOp::ShiftLeft),
                (ExprToken::ShiftRight, BinaryOp::ShiftRight),
            ],
        )
    }

    fn parse_add_sub(&mut self) -> Result<Expr, CompileError> {
        self.parse_left_associative(
            Self::parse_mul_div_rem,
            &[
                (ExprToken::Plus, BinaryOp::Add),
                (ExprToken::Minus, BinaryOp::Sub),
            ],
        )
    }

    fn parse_mul_div_rem(&mut self) -> Result<Expr, CompileError> {
        self.parse_left_associative(
            Self::parse_unary,
            &[
                (ExprToken::Star, BinaryOp::Mul),
                (ExprToken::Slash, BinaryOp::Div),
                (ExprToken::Percent, BinaryOp::Rem),
            ],
        )
    }

    fn parse_unary(&mut self) -> Result<Expr, CompileError> {
        if self.tokens.get(self.pos) == Some(&ExprToken::Bang) {
            self.pos += 1;
            return Ok(Expr::Not(Box::new(self.parse_unary()?)));
        }
        self.parse_primary()
    }

    fn parse_left_associative(
        &mut self,
        next: fn(&mut Self) -> Result<Expr, CompileError>,
        operators: &[(ExprToken, BinaryOp)],
    ) -> Result<Expr, CompileError> {
        let mut left = next(self)?;
        loop {
            let Some((_, op)) = operators
                .iter()
                .find(|(token, _)| self.tokens.get(self.pos) == Some(token))
            else {
                break;
            };
            self.pos += 1;
            let right = next(self)?;
            left = binary(left, *op, right);
        }
        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<Expr, CompileError> {
        let tok = self
            .tokens
            .get(self.pos)
            .ok_or_else(|| CompileError::Syntax("expected expression".into()))?
            .clone();
        self.pos += 1;
        match tok {
            ExprToken::String(v) => Ok(Expr::String(v)),
            ExprToken::Int(v) => Ok(Expr::Int(v)),
            ExprToken::F32(v) => Ok(Expr::F32(v)),
            ExprToken::Minus => {
                let inner = self.parse_primary()?;
                match inner {
                    Expr::Int(v) => v
                        .checked_neg()
                        .map(Expr::Int)
                        .ok_or_else(|| CompileError::Syntax("integer out of range".into())),
                    Expr::F32(v) => language_core::F32Value::new(-v.get())
                        .map(Expr::F32)
                        .ok_or_else(|| {
                            CompileError::Syntax("F32 literal must be finite and in range".into())
                        }),
                    _ => Err(CompileError::Syntax(
                        "unary - requires Int or F32 literal".into(),
                    )),
                }
            }
            ExprToken::Ident(v) if v == "true" => Ok(Expr::Bool(true)),
            ExprToken::Ident(v) if v == "false" => Ok(Expr::Bool(false)),
            ExprToken::Ident(v) => {
                if let Some(function) = builtin_registry::resolve(&v)
                    && self.tokens.get(self.pos) == Some(&ExprToken::LParen)
                {
                    self.pos += 1;
                    let mut args = Vec::new();
                    if self.tokens.get(self.pos) != Some(&ExprToken::RParen) {
                        loop {
                            args.push(self.parse_logical_or()?);
                            if self.tokens.get(self.pos) == Some(&ExprToken::Comma) {
                                self.pos += 1;
                                continue;
                            }
                            break;
                        }
                    }
                    if self.tokens.get(self.pos) != Some(&ExprToken::RParen) {
                        return Err(CompileError::Syntax(format!("{v}(...) missing )")));
                    }
                    self.pos += 1;
                    Ok(Expr::Builtin { function, args })
                } else if v == "slug" && self.tokens.get(self.pos) == Some(&ExprToken::LParen) {
                    self.pos += 1;
                    let inner = self.parse_logical_or()?;
                    if self.tokens.get(self.pos) != Some(&ExprToken::RParen) {
                        return Err(CompileError::Syntax(
                            "slug(...) expects exactly one expression".into(),
                        ));
                    }
                    self.pos += 1;
                    Ok(Expr::Slugify(Box::new(inner)))
                } else if v == "arrayF32" && self.tokens.get(self.pos) == Some(&ExprToken::LParen) {
                    self.pos += 1;
                    let len = self.parse_logical_or()?;
                    if self.tokens.get(self.pos) != Some(&ExprToken::Comma) {
                        return Err(CompileError::Syntax(
                            "arrayF32(len, fill) expects two expressions".into(),
                        ));
                    }
                    self.pos += 1;
                    let fill = self.parse_logical_or()?;
                    if self.tokens.get(self.pos) != Some(&ExprToken::RParen) {
                        return Err(CompileError::Syntax(
                            "arrayF32(len, fill) expects two expressions".into(),
                        ));
                    }
                    self.pos += 1;
                    Ok(Expr::F32ArrayNew {
                        len: Box::new(len),
                        fill: Box::new(fill),
                    })
                } else if v == "len" && self.tokens.get(self.pos) == Some(&ExprToken::LParen) {
                    self.pos += 1;
                    let array = match self.tokens.get(self.pos).cloned() {
                        Some(ExprToken::Ident(name)) => name,
                        _ => {
                            return Err(CompileError::Syntax(
                                "len(...) expects a collection variable".into(),
                            ));
                        }
                    };
                    self.pos += 1;
                    if self.tokens.get(self.pos) != Some(&ExprToken::RParen) {
                        return Err(CompileError::Syntax(
                            "len(...) expects exactly one variable".into(),
                        ));
                    }
                    self.pos += 1;
                    Ok(Expr::CollectionLen { collection: array })
                } else if self.tokens.get(self.pos) == Some(&ExprToken::LBracket) {
                    self.pos += 1;
                    let index = self.parse_logical_or()?;
                    if self.tokens.get(self.pos) != Some(&ExprToken::RBracket) {
                        return Err(CompileError::Syntax("array index missing ]".into()));
                    }
                    self.pos += 1;
                    Ok(Expr::CollectionIndex {
                        collection: v,
                        index: Box::new(index),
                    })
                } else if self.tokens.get(self.pos) == Some(&ExprToken::Dot) {
                    self.pos += 1;
                    let field = match self.tokens.get(self.pos).cloned() {
                        Some(ExprToken::Ident(x)) => x,
                        _ => {
                            return Err(CompileError::Syntax(
                                "field/enum variant name expected after .".into(),
                            ));
                        }
                    };
                    self.pos += 1;
                    let enum_name = resolve(self.namespace, &v);
                    if let Some((enum_id, def)) = self.program.enum_by_name(&enum_name) {
                        if !def.variants.iter().any(|x| x == &field) {
                            return Err(CompileError::Syntax(format!(
                                "enum `{enum_name}` has no variant `{field}`"
                            )));
                        }
                        Ok(Expr::EnumLiteral {
                            enum_id,
                            variant: field,
                        })
                    } else {
                        Ok(Expr::Field { base: v, field })
                    }
                } else {
                    Ok(Expr::Variable(v))
                }
            }
            ExprToken::LParen => {
                let e = self.parse_logical_or()?;
                if self.tokens.get(self.pos) != Some(&ExprToken::RParen) {
                    return Err(CompileError::Syntax("expected )".into()));
                }
                self.pos += 1;
                Ok(e)
            }
            _ => Err(CompileError::Syntax("expected expression value".into())),
        }
    }
}
