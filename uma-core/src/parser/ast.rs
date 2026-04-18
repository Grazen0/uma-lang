use derive_more::Display;

use crate::{parser::error::ParseError, util::Spanned};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub funcs: Vec<Spanned<Func>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Func {
    pub name: Spanned<String>,
    pub args: Vec<Spanned<String>>,
    pub stmts: Vec<Spanned<Stmt>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LValue {
    Iden(Spanned<String>),
    Access(Box<Spanned<Self>>, Box<Spanned<Expr>>),
}

impl TryFrom<Spanned<Expr>> for Spanned<LValue> {
    type Error = ParseError;

    fn try_from(expr: Spanned<Expr>) -> Result<Self, Self::Error> {
        expr.clone()
            .map(|e| -> Result<LValue, Self::Error> {
                match e {
                    Expr::Iden(name) => Ok(LValue::Iden(name)),
                    Expr::Access { value, idx } => {
                        let value_lval = Self::try_from(*value)?;
                        Ok(LValue::Access(Box::new(value_lval), idx))
                    }
                    _ => Err(ParseError::ExprNotAssignable(expr)),
                }
            })
            .transpose()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    Expr(Spanned<Expr>),
    Block(Vec<Spanned<Stmt>>),
    If {
        cond: Spanned<Expr>,
        stmt: Box<Spanned<Stmt>>,
        else_stmt: Option<Box<Spanned<Stmt>>>,
    },
    While {
        cond: Spanned<Expr>,
        stmt: Box<Spanned<Stmt>>,
        cont_expr: Option<Spanned<Expr>>,
    },
    Loop(Box<Spanned<Stmt>>),
    Return(Option<Spanned<Expr>>),
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
pub enum ModifyOp {
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
    Assign(Spanned<LValue>, Box<Spanned<Expr>>),
    Modify(Spanned<ModifyOp>, Spanned<LValue>, Box<Spanned<Expr>>),
    Rel(Spanned<Rel>, Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    Ternary {
        cond: Box<Spanned<Expr>>,
        if_yes: Box<Spanned<Expr>>,
        if_no: Box<Spanned<Expr>>,
    },
    BinOp(Spanned<BinOp>, Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    UnaryOp(Spanned<UnaryOp>, Box<Spanned<Expr>>),
    Iden(Spanned<String>),
    IntLit(u32),
    BoolLit(bool),
    StrLit(String),
    NullLit,
    ListLit(Vec<Spanned<Expr>>),
    DictLit(Vec<(Spanned<Expr>, Spanned<Expr>)>),
    FuncCall(Spanned<String>, Vec<Spanned<Expr>>),
    Access {
        value: Box<Spanned<Expr>>,
        idx: Box<Spanned<Expr>>,
    },
}
