use crate::scanner::{Token, TokenKind, TokenValue};
use derive_more::{Display, Error};
use std::{iter::Peekable, ops::Range};

fn fmt_opt_token(tok: Option<&Token>, src: &str) -> String {
    tok.map(|tok| format!("'{}'", &src[tok.byte_range.clone()]))
        .unwrap_or_else(|| "end-of-file".to_string())
}

fn make_parse_result<T>(val: T, errors: Vec<ParseError>) -> ParseResult<T> {
    if errors.is_empty() {
        Ok(val)
    } else {
        Err(errors)
    }
}

#[derive(Debug, Clone, Error, Display)]
pub enum ParseError {
    #[display("unexpected token")]
    UnexpectedToken {
        found: Option<Token>,
        expected: Option<TokenKind>,
    },

    #[display("expected expression")]
    ExpectedExpression { found: Option<Token> },

    #[display("duplicate parameter")]
    DuplicateParameter { param_token: Token },

    #[display("expression is not assignable")]
    ExprNotAssignable(#[error(ignore)] Expr),
}

impl ParseError {
    pub fn fmt_with_src(&self, src: &str) -> String {
        match self {
            Self::UnexpectedToken { found, expected } => {
                let found_str = fmt_opt_token(found.as_ref(), src);
                match expected {
                    Some(exp) => format!("expected {exp}, found {found_str}"),
                    None => format!("unexpected {found_str}"),
                }
            }
            Self::ExpectedExpression { found } => {
                let found_str = fmt_opt_token(found.as_ref(), src);
                format!("expected expression, found {found_str}")
            }
            Self::DuplicateParameter { param_token } => {
                let found_str = fmt_opt_token(Some(param_token), src);
                format!("duplicate parameter {found_str}")
            }
            Self::ExprNotAssignable(..) => "cannot assign to expression".to_string(),
        }
    }
}

impl ParseError {
    pub fn byte_range(&self) -> Option<Range<usize>> {
        match self {
            Self::UnexpectedToken { found, .. } => found.as_ref().map(|t| t.byte_range.clone()),
            Self::ExpectedExpression { found } => found.as_ref().map(|t| t.byte_range.clone()),
            Self::DuplicateParameter { param_token } => Some(param_token.byte_range.clone()),
            Self::ExprNotAssignable(..) => None,
        }
    }
}

pub type ParseResult<T> = Result<T, Vec<ParseError>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub funcs: Vec<Func>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Func {
    pub name: String,
    pub args: Vec<String>,
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LValue {
    Iden(String),
    Access(Box<LValue>, Expr),
}

impl TryFrom<Expr> for LValue {
    type Error = ParseError;

    fn try_from(expr: Expr) -> Result<Self, Self::Error> {
        match expr {
            Expr::Iden(name) => Ok(Self::Iden(name)),
            Expr::Access { value, idx } => {
                let value_lval = Self::try_from(*value)?;
                Ok(Self::Access(Box::new(value_lval), *idx))
            }
            expr => Err(ParseError::ExprNotAssignable(expr)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    Expr(Expr),
    Assign(LValue, Expr),
    AssignInPlace(InPlaceOp, LValue, Expr),
    Block(Vec<Stmt>),
    If {
        cond: Expr,
        stmt: Box<Stmt>,
        else_stmt: Option<Box<Stmt>>,
    },
    While {
        cond: Expr,
        stmt: Box<Stmt>,
    },
    Loop(Box<Stmt>),
    Return(Option<Expr>),
    Break,
    Continue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rel {
    Eq,
    Neq,
    Gt,
    Geq,
    Lt,
    Leq,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    BoolAnd,
    BoolOr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
pub enum InPlaceOp {
    #[display("+=")]
    Add,
    #[display("-=")]
    Sub,
    #[display("*=")]
    Mul,
    #[display("/=")]
    Div,
    #[display("%=")]
    Mod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Plus,
    Minus,
    BoolNot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Rel(Rel, Box<Expr>, Box<Expr>),
    Ternary {
        cond: Box<Expr>,
        if_yes: Box<Expr>,
        if_no: Box<Expr>,
    },
    BinOp(BinOp, Box<Expr>, Box<Expr>),
    UnaryOp(UnaryOp, Box<Expr>),
    Iden(String),
    Int(u32),
    Bool(bool),
    Null,
    Str(String),
    List(Vec<Expr>),
    Dict(Vec<(Expr, Expr)>),
    FuncCall(String, Vec<Expr>),
    Access {
        value: Box<Expr>,
        idx: Box<Expr>,
    },
}

#[derive(Debug)]
pub struct UmaParser<'a, Iter: Iterator<Item = Token>> {
    tokens: Peekable<&'a mut Iter>,
}

impl<'a, I: Iterator<Item = Token>> UmaParser<'a, I> {
    pub fn new(tokens: &'a mut I) -> Self {
        Self {
            tokens: tokens.peekable(),
        }
    }

    fn accept_token(&mut self, kind: TokenKind) -> Option<Token> {
        self.tokens
            .peek()
            .is_some_and(|tok| tok.val.kind() == kind)
            .then(|| self.tokens.next().unwrap())
    }

    fn accept(&mut self, kind: TokenKind) -> bool {
        self.accept_token(kind).is_some()
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Token, Vec<ParseError>> {
        let tok = self.tokens.next();

        if let Some(t) = &tok
            && t.val.kind() == kind
        {
            Ok(tok.unwrap())
        } else {
            Err(vec![ParseError::UnexpectedToken {
                found: tok,
                expected: Some(kind),
            }])
        }
    }

    fn peek_is_not(&mut self, kind: TokenKind) -> bool {
        self.tokens.peek().is_none_or(|tok| tok.val.kind() != kind)
    }

    pub fn expect_done(&mut self) -> Result<(), ParseError> {
        if let Some(tok) = self.tokens.next() {
            return Err(ParseError::UnexpectedToken {
                found: Some(tok),
                expected: None,
            });
        }

        Ok(())
    }

    pub fn program_to_end(&mut self) -> ParseResult<Program> {
        let program = self.program()?;
        self.expect_done().map_err(|e| vec![e])?;
        Ok(program)
    }

    pub fn program(&mut self) -> ParseResult<Program> {
        let mut funcs = vec![];
        let mut errors = vec![];

        while self.accept(TokenKind::Fn) {
            let name_token = self.expect(TokenKind::Iden)?;
            let name = name_token.val.into_iden();

            self.expect(TokenKind::LParen)?;

            let mut args = vec![];

            if let Some(param_token) = self.accept_token(TokenKind::Iden) {
                let TokenValue::Iden(arg_name) = param_token.val else {
                    unreachable!()
                };
                args.push(arg_name);

                while self.accept(TokenKind::Comma) {
                    let param_token = self.expect(TokenKind::Iden)?;
                    let TokenValue::Iden(arg_name) = &param_token.val else {
                        unreachable!()
                    };

                    if args.contains(arg_name) {
                        errors.push(ParseError::DuplicateParameter { param_token });
                    } else {
                        args.push(arg_name.clone());
                    }
                }
            }

            self.expect(TokenKind::RParen)?;
            self.expect(TokenKind::LBrace)?;
            let stmts = self.stmts()?;
            self.expect(TokenKind::RBrace)?;

            funcs.push(Func { name, stmts, args });
        }

        make_parse_result(Program { funcs }, errors)
    }

    fn stmts(&mut self) -> ParseResult<Vec<Stmt>> {
        let mut stmts = vec![];

        while self.peek_is_not(TokenKind::RBrace) {
            let stmt = self.stmt()?;
            stmts.push(stmt);
        }

        Ok(stmts)
    }

    fn stmt(&mut self) -> ParseResult<Stmt> {
        if self.accept(TokenKind::LBrace) {
            let blk_stmts = self.stmts()?;
            self.expect(TokenKind::RBrace)?;
            Ok(Stmt::Block(blk_stmts))
        } else if self.accept(TokenKind::If) {
            self.expect(TokenKind::LParen)?;
            let cond = self.expr()?;
            self.expect(TokenKind::RParen)?;
            let stmt = self.stmt()?;

            let else_stmt = self
                .accept(TokenKind::Else)
                .then(|| self.stmt())
                .transpose()?;

            Ok(Stmt::If {
                cond,
                stmt: Box::new(stmt),
                else_stmt: else_stmt.map(Box::new),
            })
        } else if self.accept(TokenKind::While) {
            self.expect(TokenKind::LParen)?;
            let cond = self.expr()?;
            self.expect(TokenKind::RParen)?;
            let stmt = self.stmt()?;

            Ok(Stmt::While {
                cond,
                stmt: Box::new(stmt),
            })
        } else if self.accept(TokenKind::Loop) {
            let stmt = self.stmt()?;
            Ok(Stmt::Loop(Box::new(stmt)))
        } else if self.accept(TokenKind::Return) {
            let expr = self
                .peek_is_not(TokenKind::Semi)
                .then(|| self.expr())
                .transpose()?;

            self.expect(TokenKind::Semi)?;
            Ok(Stmt::Return(expr))
        } else if self.accept(TokenKind::Break) {
            self.expect(TokenKind::Semi)?;
            Ok(Stmt::Break)
        } else if self.accept(TokenKind::Continue) {
            self.expect(TokenKind::Semi)?;
            Ok(Stmt::Continue)
        } else {
            let expr = self.expr()?;

            let stmt = if self.accept(TokenKind::Assign) {
                let lval = expr.try_into().map_err(|e| vec![e])?;
                let src_expr = self.expr()?;
                Stmt::Assign(lval, src_expr)
            } else if self.accept(TokenKind::AddAssign) {
                let lval = expr.try_into().map_err(|e| vec![e])?;
                let src_expr = self.expr()?;
                Stmt::AssignInPlace(InPlaceOp::Add, lval, src_expr)
            } else if self.accept(TokenKind::SubAssign) {
                let lval = expr.try_into().map_err(|e| vec![e])?;
                let src_expr = self.expr()?;
                Stmt::AssignInPlace(InPlaceOp::Sub, lval, src_expr)
            } else if self.accept(TokenKind::MulAssign) {
                let lval = expr.try_into().map_err(|e| vec![e])?;
                let src_expr = self.expr()?;
                Stmt::AssignInPlace(InPlaceOp::Mul, lval, src_expr)
            } else if self.accept(TokenKind::DivAssign) {
                let lval = expr.try_into().map_err(|e| vec![e])?;
                let src_expr = self.expr()?;
                Stmt::AssignInPlace(InPlaceOp::Div, lval, src_expr)
            } else if self.accept(TokenKind::ModAssign) {
                let lval = expr.try_into().map_err(|e| vec![e])?;
                let src_expr = self.expr()?;
                Stmt::AssignInPlace(InPlaceOp::Mod, lval, src_expr)
            } else {
                Stmt::Expr(expr)
            };

            self.expect(TokenKind::Semi)?;
            Ok(stmt)
        }
    }

    fn expr(&mut self) -> ParseResult<Expr> {
        self.ter_expr()
    }

    fn ter_expr(&mut self) -> ParseResult<Expr> {
        let expr = self.or_expr()?;

        if self.accept(TokenKind::Question) {
            let if_yes = self.expr()?;
            self.expect(TokenKind::Colon)?;
            let if_no = self.ter_expr()?;

            Ok(Expr::Ternary {
                cond: Box::new(expr),
                if_yes: Box::new(if_yes),
                if_no: Box::new(if_no),
            })
        } else {
            Ok(expr)
        }
    }

    fn or_expr(&mut self) -> ParseResult<Expr> {
        let mut expr = self.and_expr()?;

        while self.accept(TokenKind::BoolOr) {
            let right = self.and_expr()?;
            expr = Expr::BinOp(BinOp::BoolOr, Box::new(expr), Box::new(right));
        }

        Ok(expr)
    }

    fn and_expr(&mut self) -> ParseResult<Expr> {
        let mut expr = self.eq_expr()?;

        while self.accept(TokenKind::BoolAnd) {
            let right = self.eq_expr()?;
            expr = Expr::BinOp(BinOp::BoolAnd, Box::new(expr), Box::new(right));
        }

        Ok(expr)
    }

    fn eq_expr(&mut self) -> ParseResult<Expr> {
        let mut expr = self.ineq_expr()?;

        loop {
            let rel = if self.accept(TokenKind::Eq) {
                Rel::Eq
            } else if self.accept(TokenKind::Neq) {
                Rel::Neq
            } else {
                break;
            };

            let right = self.ineq_expr()?;
            expr = Expr::Rel(rel, Box::new(expr), Box::new(right));
        }

        Ok(expr)
    }

    fn ineq_expr(&mut self) -> ParseResult<Expr> {
        let mut expr = self.add_expr()?;

        loop {
            let rel = if self.accept(TokenKind::Lt) {
                Rel::Lt
            } else if self.accept(TokenKind::Leq) {
                Rel::Leq
            } else if self.accept(TokenKind::Gt) {
                Rel::Gt
            } else if self.accept(TokenKind::Geq) {
                Rel::Geq
            } else {
                break;
            };

            let right = self.add_expr()?;
            expr = Expr::Rel(rel, Box::new(expr), Box::new(right));
        }

        Ok(expr)
    }

    fn add_expr(&mut self) -> ParseResult<Expr> {
        let mut expr = self.mul_expr()?;

        loop {
            let op = if self.accept(TokenKind::Add) {
                BinOp::Add
            } else if self.accept(TokenKind::Sub) {
                BinOp::Sub
            } else {
                break;
            };

            let right = self.mul_expr()?;
            expr = Expr::BinOp(op, Box::new(expr), Box::new(right));
        }

        Ok(expr)
    }

    fn mul_expr(&mut self) -> ParseResult<Expr> {
        let mut expr = self.unary_expr()?;

        loop {
            let op = if self.accept(TokenKind::Mul) {
                BinOp::Mul
            } else if self.accept(TokenKind::Div) {
                BinOp::Div
            } else if self.accept(TokenKind::Mod) {
                BinOp::Mod
            } else {
                break;
            };

            let right = self.unary_expr()?;
            expr = Expr::BinOp(op, Box::new(expr), Box::new(right));
        }

        Ok(expr)
    }

    fn unary_expr(&mut self) -> ParseResult<Expr> {
        if self.accept(TokenKind::Add) {
            let expr = self.access_expr()?;
            Ok(Expr::UnaryOp(UnaryOp::Plus, Box::new(expr)))
        } else if self.accept(TokenKind::Sub) {
            let expr = self.access_expr()?;
            Ok(Expr::UnaryOp(UnaryOp::Minus, Box::new(expr)))
        } else if self.accept(TokenKind::BoolNot) {
            let expr = self.access_expr()?;
            Ok(Expr::UnaryOp(UnaryOp::BoolNot, Box::new(expr)))
        } else {
            self.access_expr()
        }
    }

    fn access_expr(&mut self) -> ParseResult<Expr> {
        let mut expr = self.base_expr()?;

        while self.accept(TokenKind::LBracket) {
            let idx_expr = self.expr()?;
            self.expect(TokenKind::RBracket)?;

            expr = Expr::Access {
                value: Box::new(expr),
                idx: Box::new(idx_expr),
            };
        }

        Ok(expr)
    }

    fn dict_entry(&mut self) -> ParseResult<(Expr, Expr)> {
        let key_expr = self.expr()?;
        self.expect(TokenKind::Colon)?;
        let val_expr = self.expr()?;
        Ok((key_expr, val_expr))
    }

    fn base_expr(&mut self) -> ParseResult<Expr> {
        let tok = self.tokens.peek().cloned();

        if self.accept(TokenKind::LParen) {
            let expr = self.expr()?;
            self.expect(TokenKind::RParen)?;
            Ok(expr)
        } else if let Some(tok) = self.accept_token(TokenKind::NumLit) {
            Ok(Expr::Int(*tok.val.as_num_lit()))
        } else if self.accept(TokenKind::True) {
            Ok(Expr::Bool(true))
        } else if self.accept(TokenKind::False) {
            Ok(Expr::Bool(false))
        } else if self.accept(TokenKind::Null) {
            Ok(Expr::Null)
        } else if self.accept(TokenKind::LBrace) {
            let mut items = vec![];

            if self.peek_is_not(TokenKind::RBrace) {
                let entry = self.dict_entry()?;
                items.push(entry);

                while self.accept(TokenKind::Comma) && self.peek_is_not(TokenKind::RBrace) {
                    let entry = self.dict_entry()?;
                    items.push(entry);
                }
            }

            self.expect(TokenKind::RBrace)?;
            Ok(Expr::Dict(items))
        } else if self.accept(TokenKind::LBracket) {
            let mut items = vec![];

            if self.peek_is_not(TokenKind::RBracket) {
                let expr = self.expr()?;
                items.push(expr);

                while self.accept(TokenKind::Comma) && self.peek_is_not(TokenKind::RBracket) {
                    let expr = self.expr()?;
                    items.push(expr);
                }
            }

            self.expect(TokenKind::RBracket)?;
            Ok(Expr::List(items))
        } else if let Some(tok) = self.accept_token(TokenKind::Iden) {
            let name = tok.val.into_iden();

            if self.accept(TokenKind::LParen) {
                let mut args = vec![];

                if self.peek_is_not(TokenKind::RParen) {
                    let expr = self.expr()?;
                    args.push(expr);

                    while self.accept(TokenKind::Comma) && self.peek_is_not(TokenKind::RParen) {
                        let expr = self.expr()?;
                        args.push(expr);
                    }
                }

                self.expect(TokenKind::RParen)?;
                Ok(Expr::FuncCall(name, args))
            } else {
                Ok(Expr::Iden(name))
            }
        } else if let Some(tok) = self.accept_token(TokenKind::StrLit) {
            Ok(Expr::Str(tok.val.into_str_lit()))
        } else {
            Err(vec![ParseError::ExpectedExpression { found: tok }])
        }
    }
}
