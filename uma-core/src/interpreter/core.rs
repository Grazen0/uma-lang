use std::{cell::RefCell, collections::HashMap, rc::Rc};

use derive_more::{Display, Error};
use kinded::Kinded;

use crate::{parser::ast::ModifyOp, util::Spanned};

#[derive(Debug, Clone, Display, Error)]
pub enum ExecuteError {
    #[display("expected {expected}, found {found}.")]
    UnexpectedType {
        expected: ValueKind,
        found: ValueKind,
    },

    #[display("undeclared function `{_0}`.")]
    UndeclaredFunction(#[error(ignore)] String),

    #[display("'break' not used within a loop")]
    UnexpectedBreak,

    #[display("'continue' not used within a loop")]
    UnexpectedContinue,

    #[display("undeclared variable `{}`.", _0.val)]
    UndeclaredVariable(#[error(ignore)] Spanned<String>),

    #[display("function expected {expected} argument(s), got {got}.")]
    MismatchedFuncArgs { expected: usize, got: usize },

    #[display("cannot use '{op}' on variable of type '{dst_kind}'.")]
    InvalidAssignOp { dst_kind: ValueKind, op: ModifyOp },

    #[display("list index out of bounds (tried to access position {_0})")]
    IndexOutOfBounds(#[error(ignore)] i64),

    #[display("cannot use {found} as dictionary key")]
    InvalidDictKey { found: ValueKind },

    #[display("key '{_0}' not found")]
    DictKeyNotFound(#[error(ignore)] DictKey),

    #[display("function did not return a value")]
    FuncDidNotReturnValue,

    #[display("function redeclared")]
    FuncRedeclaration,
}

pub type ExecuteResult<T> = Result<T, ExecuteError>;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Display)]
pub enum DictKey {
    Int(i64),
    Bool(bool),
    Str(String),
    Null,
}

impl TryFrom<Value> for DictKey {
    type Error = ExecuteError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Int(n) => Ok(Self::Int(n)),
            Value::Bool(b) => Ok(Self::Bool(b)),
            Value::Str(s) => Ok(Self::Str(s.borrow().clone())),
            Value::Null => Ok(Self::Null),
            _ => Err(ExecuteError::InvalidDictKey {
                found: value.kind(),
            }),
        }
    }
}

pub fn expect_arg_count(expected: usize, got: usize) -> ExecuteResult<()> {
    (expected == got)
        .then_some(())
        .ok_or(ExecuteError::MismatchedFuncArgs { expected, got })
}

#[derive(Debug, Clone, PartialEq, Eq, Display, Kinded)]
#[display(rename_all = "lowercase")]
#[kinded(display = "lowercase")]
pub enum Value {
    Int(i64),
    Bool(bool),
    Null,
    #[display("{}", _0.borrow())]
    Str(Rc<RefCell<String>>),
    #[display("[{}]", _0.borrow().iter().map(Value::to_string).collect::<Vec<_>>().join(", "))]
    List(Rc<RefCell<Vec<Value>>>),
    #[display("{{{}}}", _0.borrow().iter().map(|(k, v)| format!("{k}: {v}")).collect::<Vec<_>>().join(", "))]
    Dict(Rc<RefCell<HashMap<DictKey, Value>>>),
}

impl Value {
    pub fn str(s: String) -> Self {
        Self::Str(Rc::new(RefCell::new(s)))
    }

    pub fn list(items: Vec<Value>) -> Self {
        Self::List(Rc::new(RefCell::new(items)))
    }

    pub fn dict(items: HashMap<DictKey, Value>) -> Self {
        Self::Dict(Rc::new(RefCell::new(items)))
    }

    pub fn unexpected_type(&self, expected: ValueKind) -> ExecuteError {
        ExecuteError::UnexpectedType {
            expected,
            found: self.kind(),
        }
    }

    pub fn as_int(&self) -> ExecuteResult<&i64> {
        match self {
            Self::Int(n) => Ok(n),
            _ => Err(self.unexpected_type(ValueKind::Int)),
        }
    }

    pub fn as_bool(&self) -> ExecuteResult<&bool> {
        match self {
            Self::Bool(b) => Ok(b),
            _ => Err(self.unexpected_type(ValueKind::Bool)),
        }
    }

    pub fn as_list(&self) -> ExecuteResult<&Rc<RefCell<Vec<Value>>>> {
        match self {
            Self::List(items) => Ok(items),
            _ => Err(self.unexpected_type(ValueKind::List)),
        }
    }
}
