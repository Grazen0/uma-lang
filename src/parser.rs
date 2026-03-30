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
    #[display("expected type")]
    ExpectedType { found: Option<Token> },
    #[display("expected end-of-file")]
    ExpectedEof { found: Token },
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
            e @ (Self::ExpectedExpression { found } | Self::ExpectedType { found }) => {
                let found_str = fmt_opt_token(found.as_ref(), src);
                format!("{e}, found {found_str}")
            }
            e @ Self::ExpectedEof { found } => {
                let found_str = fmt_opt_token(Some(found), src);
                format!("{e}, found {found_str}")
            }
        }
    }
}

impl ParseError {
    pub fn byte_range(&self) -> Option<Range<usize>> {
        match self {
            Self::UnexpectedToken { found, .. } => found.as_ref().map(|t| t.byte_range.clone()),
            Self::ExpectedExpression { found } | Self::ExpectedType { found } => {
                found.as_ref().map(|t| t.byte_range.clone())
            }
            Self::ExpectedEof { found } => Some(found.byte_range.clone()),
        }
    }
}

pub type ParseResult<T> = Result<T, Vec<ParseError>>;

#[derive(Debug, Clone)]
pub struct Program {
    funcs: Vec<Func>,
}

#[derive(Debug, Clone)]
pub struct Func {
    ret_type: Type,
    name: String,
    args: Vec<ArgDecl>,
    blk: Block,
}

#[derive(Debug, Clone)]
pub enum Type {
    Void,
    Int,
    Bool,
    UserDef(String),
}

#[derive(Debug, Clone)]
pub struct ArgDecl {
    typ: Type,
    name: String,
}

#[derive(Debug, Clone)]
pub struct Block {
    stmts: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub enum Statement {
    If {
        cond: Expr,
        stmt: Box<Statement>,
        else_stmt: Option<Box<Statement>>,
    },
    While {
        cond: Expr,
        stmt: Box<Statement>,
    },
    DoWhile {
        stmt: Box<Statement>,
        cond: Expr,
    },
    Return(Expr),
    Continue,
    Break,
    Block(Block),
    Expr(Expr),
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
    BoolAnd,
    BoolOr,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Comma,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Plus,
    Minus,
    BoolNot,
    Addr,
    Deref,
    Sizeof,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Assign {
        dst: Box<Expr>,
        src: Box<Expr>,
    },
    Rel(Rel, Box<Expr>, Box<Expr>),
    Ternary {
        cond: Box<Expr>,
        if_yes: Box<Expr>,
        if_no: Box<Expr>,
    },
    BinOp(BinOp, Box<Expr>, Box<Expr>),
    UnaryOp(UnaryOp, Box<Expr>),
    FuncCall(Box<Expr>, Vec<Expr>),
    ArrayAccess(Box<Expr>, Box<Expr>),
    Iden(String),
    Str(String),
    Int(u32),
    Float(f64),
    Bool(bool),
    Nullptr,
}

#[derive(Debug)]
pub struct LangParser<'a, Iter: Iterator<Item = Token>> {
    tokens: Peekable<&'a mut Iter>,
}

impl<'a, I: Iterator<Item = Token>> LangParser<'a, I> {
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

    fn skip_until(&mut self, pred: impl Fn(TokenValue) -> bool) {
        while let Some(tok) = self.tokens.next() {
            if pred(tok.val) {
                break;
            }
        }
    }

    pub fn program(&mut self) -> ParseResult<Program> {
        let funcs = self.funcs()?;
        Ok(Program { funcs })
    }

    fn funcs(&mut self) -> ParseResult<Vec<Func>> {
        let mut funcs = vec![];

        while self.tokens.peek().is_some() {
            funcs.push(self.func()?);
        }

        Ok(funcs)
    }

    fn func(&mut self) -> ParseResult<Func> {
        let ret_type = self.r#type()?;
        let TokenValue::Iden(name) = self.expect_kind(TokenKind::Iden)?.val else {
            unreachable!();
        };
        self.expect_kind(TokenKind::LParen)?;

        let mut args = vec![];

        if !self.accept_kind_discard(TokenKind::RParen) {
            args.push(self.param_decl()?);

            while self.accept_kind_discard(TokenKind::Comma) {
                args.push(self.param_decl()?);
            }

            self.expect_kind(TokenKind::RParen)?;
        }

        let blk = self.block()?;

        Ok(Func {
            ret_type,
            name,
            args,
            blk,
        })
    }

    fn r#type(&mut self) -> ParseResult<Type> {
        let tok = self.tokens.peek().cloned();

        if self.accept_kind_discard(TokenKind::Void) {
            Ok(Type::Void)
        } else if self.accept_kind_discard(TokenKind::Int) {
            Ok(Type::Int)
        } else if self.accept_kind_discard(TokenKind::Bool) {
            Ok(Type::Bool)
        } else if let Some(TokenValue::Iden(name)) =
            self.accept_kind(TokenKind::Iden).map(|t| t.val)
        {
            Ok(Type::UserDef(name))
        } else {
            Err(vec![ParseError::ExpectedType { found: tok }])
        }
    }

    fn param_decl(&mut self) -> ParseResult<ArgDecl> {
        let typ = self.r#type()?;
        let TokenValue::Iden(name) = self.expect_kind(TokenKind::Iden)?.val else {
            unreachable!()
        };

        Ok(ArgDecl { typ, name })
    }

    fn block(&mut self) -> ParseResult<Block> {
        self.expect_kind(TokenKind::LBrace)?;
        let mut errors = vec![];
        let mut stmts = vec![];

        while self
            .tokens
            .peek()
            .is_some_and(|t| t.val != TokenValue::RBrace)
        {
            match self.stmt() {
                Ok(stmt) => stmts.push(stmt),
                Err(mut e) => {
                    errors.append(&mut e);

                    loop {
                        if self
                            .tokens
                            .peek()
                            .is_some_and(|tok| tok.val.kind() == TokenKind::RBrace)
                        {
                            break;
                        }

                        if let Some(tok) = self.tokens.next()
                            && tok.val.kind() == TokenKind::Semi
                        {
                            break;
                        }
                    }
                }
            }
        }

        self.expect_kind(TokenKind::RBrace)?;
        make_parse_result(Block { stmts }, errors)
    }

    fn stmt(&mut self) -> ParseResult<Statement> {
        while self.accept_kind_discard(TokenKind::Semi) {}

        if self.accept_kind_discard(TokenKind::If) {
            self.expect_kind(TokenKind::LParen)?;
            let cond = self.expr()?;
            self.expect_kind(TokenKind::RParen)?;
            let stmt = self.stmt()?;

            let else_stmt = self
                .accept_kind(TokenKind::Else)
                .map(|_| self.stmt())
                .transpose()?;

            Ok(Statement::If {
                cond,
                stmt: Box::new(stmt),
                else_stmt: else_stmt.map(Box::new),
            })
        } else if self.accept_kind_discard(TokenKind::While) {
            self.expect_kind(TokenKind::LParen)?;
            let cond = self.expr()?;
            self.expect_kind(TokenKind::RParen)?;
            let stmt = self.stmt()?;

            Ok(Statement::While {
                cond,
                stmt: Box::new(stmt),
            })
        } else if self.accept_kind_discard(TokenKind::Do) {
            let stmt = self.stmt()?;
            self.expect_kind(TokenKind::While)?;
            self.expect_kind(TokenKind::LParen)?;
            let cond = self.expr()?;
            self.expect_kind(TokenKind::RParen)?;
            self.expect_kind(TokenKind::Semi)?;

            Ok(Statement::DoWhile {
                cond,
                stmt: Box::new(stmt),
            })
        } else if self.accept_kind_discard(TokenKind::Return) {
            let expr = self.expr()?;
            self.expect_kind(TokenKind::Semi)?;
            Ok(Statement::Return(expr))
        } else if self.accept_kind_discard(TokenKind::Continue) {
            self.expect_kind(TokenKind::Semi)?;
            Ok(Statement::Continue)
        } else if self.accept_kind_discard(TokenKind::Break) {
            self.expect_kind(TokenKind::Semi)?;
            Ok(Statement::Break)
        } else if let Some(TokenValue::LBrace) = self.tokens.peek().map(|t| &t.val) {
            self.block().map(Statement::Block)
        } else {
            let expr = self.expr()?;
            self.expect_kind(TokenKind::Semi)?;
            Ok(Statement::Expr(expr))
        }
    }

    fn expr(&mut self) -> ParseResult<Expr> {
        self.comma_expr()
    }

    fn comma_expr(&mut self) -> ParseResult<Expr> {
        let mut expr = self.assign_expr()?;

        while self.accept_kind_discard(TokenKind::Comma) {
            let right = self.assign_expr()?;
            expr = Expr::BinOp(BinOp::Comma, Box::new(expr), Box::new(right));
        }

        Ok(expr)
    }

    fn assign_expr(&mut self) -> ParseResult<Expr> {
        let first = self.ter_expr()?;

        if self.accept_kind_discard(TokenKind::Assign) {
            let right = self.assign_expr()?;
            Ok(Expr::Assign {
                dst: Box::new(first),
                src: Box::new(right),
            })
        } else {
            Ok(first)
        }
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
            if self.accept_kind_discard(TokenKind::Eq) {
                let right = self.ineq_expr()?;
                expr = Expr::Rel(Rel::Eq, Box::new(expr), Box::new(right));
            } else if self.accept_kind_discard(TokenKind::Neq) {
                let right = self.ineq_expr()?;
                expr = Expr::Rel(Rel::Neq, Box::new(expr), Box::new(right));
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn ineq_expr(&mut self) -> ParseResult<Expr> {
        let mut expr = self.add_expr()?;

        loop {
            if self.accept_kind_discard(TokenKind::Lt) {
                let right = self.add_expr()?;
                expr = Expr::Rel(Rel::Lt, Box::new(expr), Box::new(right));
            } else if self.accept_kind_discard(TokenKind::Leq) {
                let right = self.add_expr()?;
                expr = Expr::Rel(Rel::Leq, Box::new(expr), Box::new(right));
            } else if self.accept_kind_discard(TokenKind::Gt) {
                let right = self.add_expr()?;
                expr = Expr::Rel(Rel::Gt, Box::new(expr), Box::new(right));
            } else if self.accept_kind_discard(TokenKind::Geq) {
                let right = self.add_expr()?;
                expr = Expr::Rel(Rel::Geq, Box::new(expr), Box::new(right));
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn add_expr(&mut self) -> ParseResult<Expr> {
        let mut expr = self.mul_expr()?;

        loop {
            if self.accept_kind_discard(TokenKind::Add) {
                let right = self.mul_expr()?;
                expr = Expr::BinOp(BinOp::Add, Box::new(expr), Box::new(right));
            } else if self.accept_kind_discard(TokenKind::Sub) {
                let right = self.mul_expr()?;
                expr = Expr::BinOp(BinOp::Sub, Box::new(expr), Box::new(right));
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn mul_expr(&mut self) -> ParseResult<Expr> {
        let mut expr = self.unary_expr()?;

        loop {
            if self.accept_kind_discard(TokenKind::Asterisk) {
                let right = self.unary_expr()?;
                expr = Expr::BinOp(BinOp::Mul, Box::new(expr), Box::new(right));
            } else if self.accept_kind_discard(TokenKind::Div) {
                let right = self.unary_expr()?;
                expr = Expr::BinOp(BinOp::Div, Box::new(expr), Box::new(right));
            } else if self.accept_kind_discard(TokenKind::Mod) {
                let right = self.unary_expr()?;
                expr = Expr::BinOp(BinOp::Mod, Box::new(expr), Box::new(right));
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn unary_expr(&mut self) -> ParseResult<Expr> {
        if self.accept_kind_discard(TokenKind::Add) {
            let expr = self.unary_expr()?;
            Ok(Expr::UnaryOp(UnaryOp::Plus, Box::new(expr)))
        } else if self.accept_kind_discard(TokenKind::Sub) {
            let expr = self.unary_expr()?;
            Ok(Expr::UnaryOp(UnaryOp::Minus, Box::new(expr)))
        } else if self.accept_kind_discard(TokenKind::BoolNot) {
            let expr = self.unary_expr()?;
            Ok(Expr::UnaryOp(UnaryOp::BoolNot, Box::new(expr)))
        } else if self.accept_kind_discard(TokenKind::Ampersand) {
            let expr = self.unary_expr()?;
            Ok(Expr::UnaryOp(UnaryOp::Addr, Box::new(expr)))
        } else if self.accept_kind_discard(TokenKind::Asterisk) {
            let expr = self.unary_expr()?;
            Ok(Expr::UnaryOp(UnaryOp::Deref, Box::new(expr)))
        } else if self.accept_kind_discard(TokenKind::Sizeof) {
            let expr = self.unary_expr()?;
            Ok(Expr::UnaryOp(UnaryOp::Sizeof, Box::new(expr)))
        } else {
            self.access_expr()
        }
    }

    fn access_expr(&mut self) -> ParseResult<Expr> {
        let base = self.base_expr()?;

        if self.accept_kind_discard(TokenKind::LParen) {
            let mut args = vec![];

            if !self.accept_kind_discard(TokenKind::RParen) {
                args.push(self.expr()?);

                while self.accept_kind_discard(TokenKind::Comma) {
                    args.push(self.expr()?);
                }
            }

            self.expect_kind(TokenKind::RParen)?;
            Ok(Expr::FuncCall(Box::new(base), args))
        } else if self.accept_kind_discard(TokenKind::LBrace) {
            let idx = self.expr()?;
            Ok(Expr::ArrayAccess(Box::new(base), Box::new(idx)))
        } else {
            Ok(base)
        }
    }

    fn base_expr(&mut self) -> ParseResult<Expr> {
        let tok = self.tokens.peek().cloned();

        if self.accept_kind_discard(TokenKind::LParen) {
            let expr = self.expr()?;
            self.expect_kind(TokenKind::RParen)?;
            Ok(expr)
        } else if let Some(TokenValue::NumLit(n)) =
            self.accept_kind(TokenKind::NumLit).map(|t| t.val)
        {
            Ok(Expr::Int(n))
        } else if let Some(TokenValue::FloatLit(f)) =
            self.accept_kind(TokenKind::FloatLit).map(|t| t.val)
        {
            Ok(Expr::Float(f))
        } else if let Some(TokenValue::Iden(name)) =
            self.accept_kind(TokenKind::Iden).map(|t| t.val)
        {
            Ok(Expr::Iden(name))
        } else if let Some(TokenValue::StrLit(str)) =
            self.accept_kind(TokenKind::StrLit).map(|t| t.val)
        {
            Ok(Expr::Str(str))
        } else if self.accept_kind_discard(TokenKind::True) {
            Ok(Expr::Bool(true))
        } else if self.accept_kind_discard(TokenKind::False) {
            Ok(Expr::Bool(false))
        } else if self.accept_kind_discard(TokenKind::Nullptr) {
            Ok(Expr::Nullptr)
        } else {
            Err(vec![ParseError::ExpectedExpression { found: tok }])
        }
    }
}
