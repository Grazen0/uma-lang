use crate::scanner::{Token, TokenKind, TokenValue};
use derive_more::{Display, Error};
use std::{collections::HashMap, iter::Peekable, ops::Range};

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
    #[display("function redeclaration")]
    FunctionRedeclaration { name_token: Token },
    #[display("duplicate parameter")]
    DuplicateParameter { param_token: Token },
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
            Self::FunctionRedeclaration { name_token } => {
                let found_str = fmt_opt_token(Some(name_token), src);
                format!("function {found_str} redeclared")
            }
            Self::DuplicateParameter { param_token } => {
                let found_str = fmt_opt_token(Some(param_token), src);
                format!("duplicate parameter {found_str}")
            }
        }
    }
}

impl ParseError {
    pub fn byte_range(&self) -> Option<Range<usize>> {
        match self {
            Self::UnexpectedToken { found, .. } => found.as_ref().map(|t| t.byte_range.clone()),
            Self::ExpectedExpression { found } => found.as_ref().map(|t| t.byte_range.clone()),
            Self::FunctionRedeclaration { name_token } => Some(name_token.byte_range.clone()),
            Self::DuplicateParameter { param_token } => Some(param_token.byte_range.clone()),
        }
    }
}

pub type ParseResult<T> = Result<T, Vec<ParseError>>;

#[derive(Debug, Clone)]
pub struct Program {
    pub funcs: HashMap<String, Func>,
}

#[derive(Debug, Clone)]
pub struct Func {
    pub args: Vec<String>,
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Assign(String, Expr),
    AssignInPlace(InPlaceOp, String, Expr),
    Print(Expr),
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

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Plus,
    Minus,
    BoolNot,
}

#[derive(Debug, Clone)]
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
    FuncCall(String, Vec<Expr>),
    ListAccess {
        list: Box<Expr>,
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

    fn accept_pred(&mut self, f: impl FnOnce(&TokenValue) -> bool) -> Option<Token> {
        if let Some(tok) = self.tokens.peek()
            && f(&tok.val)
        {
            let tok = self.tokens.next().unwrap();
            return Some(tok);
        }

        None
    }

    fn accept_kind(&mut self, kind: TokenKind) -> Option<Token> {
        self.accept_pred(|tok| tok.kind() == kind)
    }

    fn accept_kind_discard(&mut self, kind: TokenKind) -> bool {
        self.accept_kind(kind).is_some()
    }

    fn expect_kind(&mut self, kind: TokenKind) -> Result<Token, Vec<ParseError>> {
        let peek = self.tokens.peek();

        if let Some(tok) = peek
            && tok.val.kind() == kind
        {
            let tok = self.tokens.next().unwrap();
            return Ok(tok);
        }

        Err(vec![ParseError::UnexpectedToken {
            expected: Some(kind),
            found: peek.cloned(),
        }])
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
        let mut errors = vec![];
        let mut funcs = HashMap::new();

        while self.accept_kind_discard(TokenKind::Fn) {
            let name_token = self.expect_kind(TokenKind::Iden)?;
            let TokenValue::Iden(fn_name) = &name_token.val else {
                unreachable!()
            };

            self.expect_kind(TokenKind::LParen)?;

            let mut args = vec![];

            if let Some(param_token) = self.accept_kind(TokenKind::Iden) {
                let TokenValue::Iden(arg_name) = param_token.val else {
                    unreachable!()
                };
                args.push(arg_name);

                while self.accept_kind_discard(TokenKind::Comma) {
                    let param_token = self.expect_kind(TokenKind::Iden)?;
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

            self.expect_kind(TokenKind::RParen)?;
            self.expect_kind(TokenKind::LBrace)?;
            let stmts = self.stmts()?;
            self.expect_kind(TokenKind::RBrace)?;

            if funcs.contains_key(fn_name) {
                errors.push(ParseError::FunctionRedeclaration { name_token });
                continue;
            }

            funcs.insert(fn_name.clone(), Func { stmts, args });
        }

        make_parse_result(Program { funcs }, errors)
    }

    fn stmts(&mut self) -> ParseResult<Vec<Stmt>> {
        let mut stmts = vec![];

        while self
            .tokens
            .peek()
            .map(|tok| tok.val.kind())
            .is_some_and(|kind| {
                [
                    TokenKind::Print,
                    TokenKind::Iden,
                    TokenKind::LBrace,
                    TokenKind::If,
                    TokenKind::While,
                    TokenKind::Loop,
                    TokenKind::Return,
                    TokenKind::Break,
                    TokenKind::Continue,
                ]
                .contains(&kind)
            })
        {
            let stmt = self.stmt()?;
            stmts.push(stmt);
        }

        Ok(stmts)
    }

    fn stmt(&mut self) -> ParseResult<Stmt> {
        if self.accept_kind_discard(TokenKind::Print) {
            let expr = self.expr()?;
            self.expect_kind(TokenKind::Semi)?;
            Ok(Stmt::Print(expr))
        } else if self.accept_kind_discard(TokenKind::LBrace) {
            let blk_stmts = self.stmts()?;
            self.expect_kind(TokenKind::RBrace)?;
            Ok(Stmt::Block(blk_stmts))
        } else if self.accept_kind_discard(TokenKind::If) {
            self.expect_kind(TokenKind::LParen)?;
            let cond = self.expr()?;
            self.expect_kind(TokenKind::RParen)?;
            let stmt = self.stmt()?;

            let else_stmt = self
                .accept_kind_discard(TokenKind::Else)
                .then(|| self.stmt())
                .transpose()?;

            Ok(Stmt::If {
                cond,
                stmt: Box::new(stmt),
                else_stmt: else_stmt.map(Box::new),
            })
        } else if self.accept_kind_discard(TokenKind::While) {
            self.expect_kind(TokenKind::LParen)?;
            let cond = self.expr()?;
            self.expect_kind(TokenKind::RParen)?;
            let stmt = self.stmt()?;

            Ok(Stmt::While {
                cond,
                stmt: Box::new(stmt),
            })
        } else if self.accept_kind_discard(TokenKind::Loop) {
            let stmt = self.stmt()?;
            Ok(Stmt::Loop(Box::new(stmt)))
        } else if self.accept_kind_discard(TokenKind::Return) {
            let expr = self
                .tokens
                .peek()
                .is_some_and(|tok| tok.val != TokenValue::Semi)
                .then(|| self.expr())
                .transpose()?;

            self.expect_kind(TokenKind::Semi)?;
            Ok(Stmt::Return(expr))
        } else if self.accept_kind_discard(TokenKind::Break) {
            self.expect_kind(TokenKind::Semi)?;
            Ok(Stmt::Break)
        } else if self.accept_kind_discard(TokenKind::Continue) {
            self.expect_kind(TokenKind::Semi)?;
            Ok(Stmt::Continue)
        } else if let Some(tok) = self.accept_kind(TokenKind::Iden) {
            let TokenValue::Iden(name) = tok.val else {
                unreachable!()
            };

            let stmt = if self.accept_kind_discard(TokenKind::AddAssign) {
                let expr = self.expr()?;
                Stmt::AssignInPlace(InPlaceOp::Add, name, expr)
            } else if self.accept_kind_discard(TokenKind::SubAssign) {
                let expr = self.expr()?;
                Stmt::AssignInPlace(InPlaceOp::Sub, name, expr)
            } else if self.accept_kind_discard(TokenKind::MulAssign) {
                let expr = self.expr()?;
                Stmt::AssignInPlace(InPlaceOp::Mul, name, expr)
            } else if self.accept_kind_discard(TokenKind::DivAssign) {
                let expr = self.expr()?;
                Stmt::AssignInPlace(InPlaceOp::Div, name, expr)
            } else if self.accept_kind_discard(TokenKind::ModAssign) {
                let expr = self.expr()?;
                Stmt::AssignInPlace(InPlaceOp::Mod, name, expr)
            } else {
                self.expect_kind(TokenKind::Assign)?;
                let expr = self.expr()?;
                Stmt::Assign(name, expr)
            };

            self.expect_kind(TokenKind::Semi)?;
            Ok(stmt)
        } else {
            Err(vec![ParseError::UnexpectedToken {
                found: self.tokens.next(),
                expected: None,
            }])
        }
    }

    fn expr(&mut self) -> ParseResult<Expr> {
        self.ter_expr()
    }

    fn ter_expr(&mut self) -> ParseResult<Expr> {
        let expr = self.or_expr()?;

        if self.accept_kind_discard(TokenKind::Question) {
            let if_yes = self.expr()?;
            self.expect_kind(TokenKind::Colon)?;
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

        while self.accept_kind_discard(TokenKind::BoolOr) {
            let right = self.and_expr()?;
            expr = Expr::BinOp(BinOp::BoolOr, Box::new(expr), Box::new(right));
        }

        Ok(expr)
    }

    fn and_expr(&mut self) -> ParseResult<Expr> {
        let mut expr = self.eq_expr()?;

        while self.accept_kind_discard(TokenKind::BoolAnd) {
            let right = self.eq_expr()?;
            expr = Expr::BinOp(BinOp::BoolAnd, Box::new(expr), Box::new(right));
        }

        Ok(expr)
    }

    fn eq_expr(&mut self) -> ParseResult<Expr> {
        let mut expr = self.ineq_expr()?;

        loop {
            let rel = if self.accept_kind_discard(TokenKind::Eq) {
                Rel::Eq
            } else if self.accept_kind_discard(TokenKind::Neq) {
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
            let rel = if self.accept_kind_discard(TokenKind::Lt) {
                Rel::Lt
            } else if self.accept_kind_discard(TokenKind::Leq) {
                Rel::Leq
            } else if self.accept_kind_discard(TokenKind::Gt) {
                Rel::Gt
            } else if self.accept_kind_discard(TokenKind::Geq) {
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
            let op = if self.accept_kind_discard(TokenKind::Add) {
                BinOp::Add
            } else if self.accept_kind_discard(TokenKind::Sub) {
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
            let op = if self.accept_kind_discard(TokenKind::Mul) {
                BinOp::Mul
            } else if self.accept_kind_discard(TokenKind::Div) {
                BinOp::Div
            } else if self.accept_kind_discard(TokenKind::Mod) {
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
        if self.accept_kind_discard(TokenKind::Add) {
            let expr = self.access_expr()?;
            Ok(Expr::UnaryOp(UnaryOp::Plus, Box::new(expr)))
        } else if self.accept_kind_discard(TokenKind::Sub) {
            let expr = self.access_expr()?;
            Ok(Expr::UnaryOp(UnaryOp::Minus, Box::new(expr)))
        } else if self.accept_kind_discard(TokenKind::BoolNot) {
            let expr = self.access_expr()?;
            Ok(Expr::UnaryOp(UnaryOp::BoolNot, Box::new(expr)))
        } else {
            self.access_expr()
        }
    }

    fn access_expr(&mut self) -> ParseResult<Expr> {
        let mut expr = self.base_expr()?;

        while self.accept_kind_discard(TokenKind::LBracket) {
            let idx_expr = self.expr()?;
            self.expect_kind(TokenKind::RBracket)?;

            expr = Expr::ListAccess {
                list: Box::new(expr),
                idx: Box::new(idx_expr),
            };
        }

        Ok(expr)
    }

    fn base_expr(&mut self) -> ParseResult<Expr> {
        let tok = self.tokens.peek().cloned();

        if self.accept_kind_discard(TokenKind::LParen) {
            let expr = self.expr()?;
            self.expect_kind(TokenKind::RParen)?;
            Ok(expr)
        } else if let Some(tok) = self.accept_kind(TokenKind::NumLit) {
            Ok(Expr::Int(tok.val.into_num()))
        } else if self.accept_kind_discard(TokenKind::True) {
            Ok(Expr::Bool(true))
        } else if self.accept_kind_discard(TokenKind::False) {
            Ok(Expr::Bool(false))
        } else if self.accept_kind_discard(TokenKind::Null) {
            Ok(Expr::Null)
        } else if self.accept_kind_discard(TokenKind::LBracket) {
            let mut items = vec![];

            if self
                .tokens
                .peek()
                .is_some_and(|tok| tok.val != TokenValue::RBracket)
            {
                let expr = self.expr()?;
                items.push(expr);

                while self.accept_kind_discard(TokenKind::Comma)
                    && self
                        .tokens
                        .peek()
                        .is_some_and(|tok| tok.val != TokenValue::RBracket)
                {
                    let expr = self.expr()?;
                    items.push(expr);
                }
            }

            self.expect_kind(TokenKind::RBracket)?;
            Ok(Expr::List(items))
        } else if let Some(tok) = self.accept_kind(TokenKind::Iden) {
            let TokenValue::Iden(name) = tok.val else {
                unreachable!()
            };

            if self.accept_kind_discard(TokenKind::LParen) {
                let mut args = vec![];
                if self
                    .tokens
                    .peek()
                    .is_some_and(|tok| tok.val != TokenValue::RParen)
                {
                    let expr = self.expr()?;
                    args.push(expr);

                    while self.accept_kind_discard(TokenKind::Comma)
                        && self
                            .tokens
                            .peek()
                            .is_some_and(|tok| tok.val != TokenValue::RParen)
                    {
                        let expr = self.expr()?;
                        args.push(expr);
                    }
                }

                self.expect_kind(TokenKind::RParen)?;

                Ok(Expr::FuncCall(name, args))
            } else {
                Ok(Expr::Iden(name))
            }
        } else if let Some(tok) = self.accept_kind(TokenKind::StrLit) {
            Ok(Expr::Str(tok.val.into_str()))
        } else {
            Err(vec![ParseError::ExpectedExpression { found: tok }])
        }
    }
}
