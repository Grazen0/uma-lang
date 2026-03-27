#![allow(unused)]

use crate::scanner::{ScanError, Token, TokenValue, TokenValueKind};
use derive_more::{Display, Error, From};
use std::iter::Peekable;

fn format_opt_token(tok: &Option<Token>) -> String {
    tok.as_ref()
        .map(|t| format!("'{}'", t.val))
        .unwrap_or_else(|| "EOF".to_string())
}

#[derive(Debug, Clone, Error, Display, From)]
pub enum ParseError {
    Scan(#[from] ScanError),
    #[display("Unexpected token (found {})", format_opt_token(found))]
    UnexpectedToken {
        found: Option<Token>,
    },
    #[display(
        "Unexpected token (expected {expected}, found {})",
        format_opt_token(found)
    )]
    ExpectedToken {
        expected: TokenValueKind,
        found: Option<Token>,
    },
}

pub type ParseResult<T> = Result<T, ParseError>;

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

#[derive(Debug, Clone)]
pub struct FnCall {
    fn_name: String,
    args: Vec<Expr>,
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
    StrLit(String),
    NumLit(u32),
    FloatLit(f64),
    BoolLit(bool),
    Nullptr,
}

#[derive(Debug)]
pub struct LangParser<'a, Iter: Iterator<Item = Result<Token, ScanError>>> {
    tokens: Peekable<&'a mut Iter>,
}

impl<'a, I: Iterator<Item = Result<Token, ScanError>>> LangParser<'a, I> {
    pub fn new(tokens: &'a mut I) -> Self {
        Self {
            tokens: tokens.peekable(),
        }
    }

    pub fn peek(&mut self) -> Result<Option<&Token>, ParseError> {
        self.tokens
            .peek()
            .map(|r| r.as_ref().map_err(|e| ParseError::from(e.clone())))
            .transpose()
    }

    fn next(&mut self) -> Result<Option<Token>, ParseError> {
        self.tokens.next().transpose().map_err(ParseError::from)
    }

    fn accept(&mut self, kind: TokenValueKind) -> Result<Option<Token>, ParseError> {
        let peek = self.peek()?;

        if let Some(tok) = peek
            && tok.val.kind() == kind
        {
            let tok = self.next()?.unwrap();
            return Ok(Some(tok));
        }

        Ok(None)
    }

    fn accept_dis(&mut self, kind: TokenValueKind) -> Result<bool, ParseError> {
        self.accept(kind).map(|opt| opt.is_some())
    }

    fn expect(&mut self, kind: TokenValueKind) -> Result<Token, ParseError> {
        let peek = self.peek()?;

        if let Some(tok) = peek
            && tok.val.kind() == kind
        {
            let tok = self.next()?.unwrap();
            return Ok(tok);
        }

        Err(ParseError::ExpectedToken {
            expected: kind,
            found: peek.cloned(),
        })
    }

    pub fn ensure_done(&mut self) -> Result<(), ParseError> {
        match self.peek()? {
            Some(tok) => Err(ParseError::UnexpectedToken {
                found: Some(tok.clone()),
            }),
            None => Ok(()),
        }
    }

    pub fn program(&mut self) -> ParseResult<Program> {
        let funcs = self.funcs()?;
        Ok(Program { funcs })
    }

    fn funcs(&mut self) -> ParseResult<Vec<Func>> {
        let mut funcs = vec![];

        while self.peek()?.is_some() {
            funcs.push(self.func()?);
        }

        Ok(funcs)
    }

    fn func(&mut self) -> ParseResult<Func> {
        let ret_type = self.r#type()?;
        let TokenValue::Iden(name) = self.expect(TokenValueKind::Iden)?.val else {
            unreachable!();
        };
        self.expect(TokenValueKind::LParen)?;

        let mut args = vec![];

        if !self.accept_dis(TokenValueKind::RParen)? {
            args.push(self.arg_decl()?);

            while self.accept_dis(TokenValueKind::Comma)? {
                args.push(self.arg_decl()?);
            }

            self.expect(TokenValueKind::RParen)?;
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
        let tok = self.peek()?.cloned();

        if self.accept_dis(TokenValueKind::Void)? {
            Ok(Type::Void)
        } else if self.accept_dis(TokenValueKind::Int)? {
            Ok(Type::Int)
        } else if self.accept_dis(TokenValueKind::Bool)? {
            Ok(Type::Bool)
        } else if let Some(TokenValue::Iden(name)) =
            self.accept(TokenValueKind::Iden)?.map(|t| t.val)
        {
            Ok(Type::UserDef(name))
        } else {
            Err(ParseError::UnexpectedToken { found: tok })
        }
    }

    fn arg_decl(&mut self) -> ParseResult<ArgDecl> {
        let typ = self.r#type()?;
        let TokenValue::Iden(name) = self.expect(TokenValueKind::Iden)?.val else {
            unreachable!()
        };

        Ok(ArgDecl { typ, name })
    }

    fn block(&mut self) -> ParseResult<Block> {
        self.expect(TokenValueKind::LBrace)?;
        let mut stmts = vec![];

        while self.peek()?.is_some_and(|t| t.val != TokenValue::RBrace) {
            stmts.push(self.stmt()?);
        }

        self.expect(TokenValueKind::RBrace)?;

        Ok(Block { stmts })
    }

    fn stmt(&mut self) -> ParseResult<Statement> {
        while self.accept_dis(TokenValueKind::Semi)? {}

        if self.accept_dis(TokenValueKind::If)? {
            self.expect(TokenValueKind::LParen)?;
            let cond = self.expr()?;
            self.expect(TokenValueKind::RParen)?;
            let stmt = self.stmt()?;

            let else_stmt = self
                .accept(TokenValueKind::Else)?
                .map(|_| self.stmt())
                .transpose()?;

            Ok(Statement::If {
                cond,
                stmt: Box::new(stmt),
                else_stmt: else_stmt.map(Box::new),
            })
        } else if self.accept_dis(TokenValueKind::While)? {
            self.expect(TokenValueKind::LParen)?;
            let cond = self.expr()?;
            self.expect(TokenValueKind::RParen)?;
            let stmt = self.stmt()?;

            Ok(Statement::While {
                cond,
                stmt: Box::new(stmt),
            })
        } else if self.accept_dis(TokenValueKind::Do)? {
            let stmt = self.stmt()?;
            self.expect(TokenValueKind::While)?;
            self.expect(TokenValueKind::LParen)?;
            let cond = self.expr()?;
            self.expect(TokenValueKind::RParen)?;
            self.expect(TokenValueKind::Semi)?;

            Ok(Statement::DoWhile {
                cond,
                stmt: Box::new(stmt),
            })
        } else if self.accept_dis(TokenValueKind::Return)? {
            let expr = self.expr()?;
            self.expect(TokenValueKind::Semi)?;
            Ok(Statement::Return(expr))
        } else if self.accept_dis(TokenValueKind::Continue)? {
            self.expect(TokenValueKind::Semi)?;
            Ok(Statement::Continue)
        } else if self.accept_dis(TokenValueKind::Break)? {
            self.expect(TokenValueKind::Semi)?;
            Ok(Statement::Break)
        } else if let Some(TokenValue::LBrace) = self.peek()?.map(|t| &t.val) {
            self.block().map(Statement::Block)
        } else {
            let expr = self.expr()?;
            self.expect(TokenValueKind::Semi);
            Ok(Statement::Expr(expr))
        }
    }

    fn expr(&mut self) -> ParseResult<Expr> {
        self.comma_expr()
    }

    fn comma_expr(&mut self) -> ParseResult<Expr> {
        let mut expr = self.assign_expr()?;

        while self.accept_dis(TokenValueKind::Comma)? {
            let right = self.assign_expr()?;
            expr = Expr::BinOp(BinOp::Comma, Box::new(expr), Box::new(right));
        }

        Ok(expr)
    }

    fn assign_expr(&mut self) -> ParseResult<Expr> {
        let first = self.ter_expr()?;

        if self.accept_dis(TokenValueKind::Assign)? {
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

        if self.accept_dis(TokenValueKind::Question)? {
            let if_yes = self.expr()?;
            self.expect(TokenValueKind::Colon)?;
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

        while self.accept_dis(TokenValueKind::BoolOr)? {
            let right = self.and_expr()?;
            expr = Expr::BinOp(BinOp::BoolOr, Box::new(expr), Box::new(right));
        }

        Ok(expr)
    }

    fn and_expr(&mut self) -> ParseResult<Expr> {
        let mut expr = self.eq_expr()?;

        while self.accept_dis(TokenValueKind::BoolAnd)? {
            let right = self.eq_expr()?;
            expr = Expr::BinOp(BinOp::BoolAnd, Box::new(expr), Box::new(right));
        }

        Ok(expr)
    }

    fn eq_expr(&mut self) -> ParseResult<Expr> {
        let mut expr = self.ineq_expr()?;

        loop {
            if self.accept_dis(TokenValueKind::Eq)? {
                let right = self.ineq_expr()?;
                expr = Expr::Rel(Rel::Eq, Box::new(expr), Box::new(right));
            } else if self.accept_dis(TokenValueKind::Neq)? {
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
            if self.accept_dis(TokenValueKind::Lt)? {
                let right = self.add_expr()?;
                expr = Expr::Rel(Rel::Lt, Box::new(expr), Box::new(right));
            } else if self.accept_dis(TokenValueKind::Leq)? {
                let right = self.add_expr()?;
                expr = Expr::Rel(Rel::Leq, Box::new(expr), Box::new(right));
            } else if self.accept_dis(TokenValueKind::Gt)? {
                let right = self.add_expr()?;
                expr = Expr::Rel(Rel::Gt, Box::new(expr), Box::new(right));
            } else if self.accept_dis(TokenValueKind::Geq)? {
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
            if self.accept_dis(TokenValueKind::Add)? {
                let right = self.mul_expr()?;
                expr = Expr::BinOp(BinOp::Add, Box::new(expr), Box::new(right));
            } else if self.accept_dis(TokenValueKind::Sub)? {
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
            if self.accept_dis(TokenValueKind::Asterisk)? {
                let right = self.unary_expr()?;
                expr = Expr::BinOp(BinOp::Mul, Box::new(expr), Box::new(right));
            } else if self.accept_dis(TokenValueKind::Div)? {
                let right = self.unary_expr()?;
                expr = Expr::BinOp(BinOp::Div, Box::new(expr), Box::new(right));
            } else if self.accept_dis(TokenValueKind::Mod)? {
                let right = self.unary_expr()?;
                expr = Expr::BinOp(BinOp::Mod, Box::new(expr), Box::new(right));
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn unary_expr(&mut self) -> ParseResult<Expr> {
        if self.accept_dis(TokenValueKind::Add)? {
            let expr = self.unary_expr()?;
            Ok(Expr::UnaryOp(UnaryOp::Plus, Box::new(expr)))
        } else if self.accept_dis(TokenValueKind::Sub)? {
            let expr = self.unary_expr()?;
            Ok(Expr::UnaryOp(UnaryOp::Minus, Box::new(expr)))
        } else if self.accept_dis(TokenValueKind::BoolNot)? {
            let expr = self.unary_expr()?;
            Ok(Expr::UnaryOp(UnaryOp::BoolNot, Box::new(expr)))
        } else if self.accept_dis(TokenValueKind::Ampersand)? {
            let expr = self.unary_expr()?;
            Ok(Expr::UnaryOp(UnaryOp::Addr, Box::new(expr)))
        } else if self.accept_dis(TokenValueKind::Asterisk)? {
            let expr = self.unary_expr()?;
            Ok(Expr::UnaryOp(UnaryOp::Deref, Box::new(expr)))
        } else if self.accept_dis(TokenValueKind::Sizeof)? {
            let expr = self.unary_expr()?;
            Ok(Expr::UnaryOp(UnaryOp::Sizeof, Box::new(expr)))
        } else {
            self.access_expr()
        }
    }

    fn access_expr(&mut self) -> ParseResult<Expr> {
        let base = self.base_expr()?;

        if self.accept_dis(TokenValueKind::LParen)? {
            let mut args = vec![];

            if !self.accept_dis(TokenValueKind::RParen)? {
                args.push(self.expr()?);

                while self.accept_dis(TokenValueKind::Comma)? {
                    args.push(self.expr()?);
                }
            }

            self.expect(TokenValueKind::RParen)?;

            Ok(Expr::FuncCall(Box::new(base), args))
        } else if self.accept_dis(TokenValueKind::LBrace)? {
            let idx = self.expr()?;
            Ok(Expr::ArrayAccess(Box::new(base), Box::new(idx)))
        } else {
            Ok(base)
        }
    }

    fn base_expr(&mut self) -> ParseResult<Expr> {
        let tok = self.peek()?.cloned();

        if self.accept_dis(TokenValueKind::LParen)? {
            let expr = self.expr()?;
            self.expect(TokenValueKind::RParen)?;
            Ok(expr)
        } else if let Some(TokenValue::NumLit(n)) =
            self.accept(TokenValueKind::NumLit)?.map(|t| t.val)
        {
            Ok(Expr::NumLit(n))
        } else if let Some(TokenValue::FloatLit(f)) =
            self.accept(TokenValueKind::FloatLit)?.map(|t| t.val)
        {
            Ok(Expr::FloatLit(f))
        } else if let Some(TokenValue::Iden(name)) =
            self.accept(TokenValueKind::Iden)?.map(|t| t.val)
        {
            Ok(Expr::Iden(name))
        } else if let Some(TokenValue::StrLit(str)) =
            self.accept(TokenValueKind::StrLit)?.map(|t| t.val)
        {
            Ok(Expr::StrLit(str))
        } else if self.accept_dis(TokenValueKind::True)? {
            Ok(Expr::BoolLit(true))
        } else if self.accept_dis(TokenValueKind::False)? {
            Ok(Expr::BoolLit(false))
        } else if self.accept_dis(TokenValueKind::Nullptr)? {
            Ok(Expr::Nullptr)
        } else {
            Err(ParseError::UnexpectedToken { found: tok })
        }
    }
}
