use std::{
    cell::RefCell,
    collections::{HashMap, hash_map::Entry},
    rc::Rc,
};

use derive_more::{Display, Error};
use kinded::Kinded;

use crate::parser::{BinOp, Expr, InPlaceOp, Program, Rel, Stmt, UnaryOp};

#[derive(Debug, Clone, Display, Error)]
pub enum ExecuteError {
    #[display("expected {expected}, found {found}.")]
    UnexpectedType {
        expected: PrimitiveKind,
        found: PrimitiveKind,
    },

    #[display("undeclared function `{_0}`.")]
    UndeclaredFunction(#[error(ignore)] String),

    #[display("'break' not used within a loop")]
    UnexpectedBreak,

    #[display("'continue' not used within a loop")]
    UnexpectedContinue,

    #[display("undeclared variable `{_0}`.")]
    UndeclaredVariable(#[error(ignore)] String),

    #[display("function `{func_name}` expected {expected} argument(s), got {got}.")]
    MismatchedFuncArgs {
        func_name: String,
        expected: usize,
        got: usize,
    },

    #[display("cannot use '{op}' on variable of type '{dst_kind}'.")]
    InvalidAssignOp {
        dst_kind: PrimitiveKind,
        op: InPlaceOp,
    },

    #[display("list index out of bounds (tried to access position {_0})")]
    ListIndexOutOfBounds(#[error(ignore)] i64),
}

pub type ExecuteResult<T> = Result<T, ExecuteError>;

#[derive(Debug, Clone, PartialEq, Eq, Display, Kinded)]
#[display(rename_all = "lowercase")]
#[kinded(display = "lowercase")]
pub enum Primitive {
    Int(i64),
    Bool(bool),
    Null,
    #[display("{}", _0.borrow())]
    Str(Rc<RefCell<String>>),
    #[display("[{}]", _0.borrow().iter().map(Primitive::to_string).collect::<Vec<_>>().join(", "))]
    List(Rc<RefCell<Vec<Primitive>>>),
}

impl Primitive {
    fn str(s: String) -> Self {
        Self::Str(Rc::new(RefCell::new(s)))
    }

    fn list(items: Vec<Primitive>) -> Self {
        Self::List(Rc::new(RefCell::new(items)))
    }

    // fn object(obj: Object) -> Self {
    //     let rc = Rc::new(RefCell::new(obj));
    //     Self::Object(rc)
    // }

    fn unexpected_type(&self, expected: PrimitiveKind) -> ExecuteError {
        ExecuteError::UnexpectedType {
            expected,
            found: self.kind(),
        }
    }

    fn as_int(&self) -> ExecuteResult<&i64> {
        match self {
            Self::Int(n) => Ok(n),
            _ => Err(self.unexpected_type(PrimitiveKind::Int)),
        }
    }

    fn as_bool(&self) -> ExecuteResult<&bool> {
        match self {
            Self::Bool(b) => Ok(b),
            _ => Err(self.unexpected_type(PrimitiveKind::Bool)),
        }
    }

    fn as_list(&self) -> ExecuteResult<&Rc<RefCell<Vec<Primitive>>>> {
        match self {
            Self::List(items) => Ok(items),
            _ => Err(self.unexpected_type(PrimitiveKind::List)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Kinded)]
pub enum Object {
    Str(String),
    List(Vec<Primitive>),
}

impl std::fmt::Display for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::List(items) => {
                write!(f, "[")?;

                if !items.is_empty() {
                    write!(f, "{}", items[0])?;

                    for item in items.iter().skip(1) {
                        write!(f, ", {}", item)?;
                    }
                }

                write!(f, "]")?;
            }
            Self::Str(s) => write!(f, "{s}")?,
        }
        Ok(())
    }
}

impl Object {
    fn as_str_mut(&mut self) -> &mut String {
        match self {
            Self::Str(s) => s,
            _ => unreachable!(),
        }
    }

    fn as_list_mut(&mut self) -> &mut Vec<Primitive> {
        match self {
            Self::List(items) => items,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Default)]
struct SymbolTable {
    tables: Vec<HashMap<String, Primitive>>,
}

impl SymbolTable {
    fn enter_scope(&mut self) {
        self.tables.push(HashMap::default());
    }

    fn exit_scope(&mut self) {
        self.tables.pop();
    }

    fn get(&self, name: &str) -> Option<&Primitive> {
        self.tables.iter().rev().find_map(|tbl| tbl.get(name))
    }

    fn get_mut(&mut self, name: &str) -> Option<&mut Primitive> {
        self.tables
            .iter_mut()
            .rev()
            .find_map(|tbl| tbl.get_mut(name))
    }

    fn set(&mut self, name: String, val: Primitive) {
        for table in self.tables.iter_mut().rev() {
            if let Entry::Occupied(mut e) = table.entry(name.clone()) {
                e.insert(val);
                return;
            }
        }

        self.tables.last_mut().unwrap().insert(name, val);
    }
}

#[derive(Debug, Clone)]
enum ControlAction {
    Return(Option<Primitive>),
    Break,
    Continue,
}

#[derive(Debug, Clone)]
pub struct Executor<'a> {
    program: &'a Program,
}

impl<'a> Executor<'a> {
    pub fn new(program: &'a Program) -> Self {
        Self { program }
    }

    pub fn execute(&mut self) -> ExecuteResult<()> {
        self.execute_func("main", vec![])?;
        Ok(())
    }

    fn execute_func(&mut self, func_name: &str, args: Vec<Primitive>) -> ExecuteResult<Primitive> {
        let func = self
            .program
            .funcs
            .get(func_name)
            .ok_or_else(|| ExecuteError::UndeclaredFunction(func_name.to_string()))?;

        if args.len() != func.args.len() {
            return Err(ExecuteError::MismatchedFuncArgs {
                func_name: func_name.to_string(),
                expected: func.args.len(),
                got: args.len(),
            });
        }

        let mut sym_table = SymbolTable::default();
        sym_table.enter_scope();

        for (arg_name, arg) in func.args.iter().zip(args) {
            sym_table.set(arg_name.clone(), arg);
        }

        let result = self.execute_stmts(&func.stmts, &mut sym_table)?;
        sym_table.exit_scope();

        match result {
            None => Ok(Primitive::Null),
            Some(ControlAction::Break) => Err(ExecuteError::UnexpectedBreak),
            Some(ControlAction::Continue) => Err(ExecuteError::UnexpectedContinue),
            Some(ControlAction::Return(val)) => Ok(val.unwrap_or(Primitive::Null)),
        }
    }

    fn execute_stmts(
        &mut self,
        stmts: &[Stmt],
        sym_table: &mut SymbolTable,
    ) -> ExecuteResult<Option<ControlAction>> {
        for stmt in stmts {
            if let Some(val) = self.execute_stmt(stmt, sym_table)? {
                return Ok(Some(val));
            }
        }

        Ok(None)
    }

    fn execute_stmt(
        &mut self,
        stmt: &Stmt,
        sym_table: &mut SymbolTable,
    ) -> ExecuteResult<Option<ControlAction>> {
        let result = match stmt {
            Stmt::Assign(dst_name, expr) => {
                let val = self.eval_expr(expr, sym_table)?;
                sym_table.set(dst_name.clone(), val);
                None
            }
            Stmt::Print(expr) => {
                let val = self.eval_expr(expr, sym_table)?;
                println!("{val}");
                None
            }
            Stmt::Block(stmts) => {
                sym_table.enter_scope();
                let result = self.execute_stmts(stmts, sym_table)?;
                sym_table.exit_scope();
                result
            }
            Stmt::If {
                cond,
                stmt,
                else_stmt,
            } => {
                if *self.eval_expr(cond, sym_table)?.as_bool()? {
                    sym_table.enter_scope();
                    let result = self.execute_stmt(stmt, sym_table)?;
                    sym_table.exit_scope();
                    result
                } else if let Some(else_stmt) = else_stmt.as_ref() {
                    self.execute_stmt(else_stmt, sym_table)?
                } else {
                    None
                }
            }
            Stmt::While { cond, stmt } => {
                sym_table.enter_scope();

                let result = loop {
                    if !*self.eval_expr(cond, sym_table)?.as_bool()? {
                        break None;
                    }

                    match self.execute_stmt(stmt, sym_table)? {
                        Some(ControlAction::Break) => break None,
                        Some(ControlAction::Continue) => {}
                        Some(ControlAction::Return(val)) => break Some(ControlAction::Return(val)),
                        None => {}
                    }
                };

                sym_table.exit_scope();
                result
            }
            Stmt::Loop(stmt) => {
                sym_table.enter_scope();

                let result = loop {
                    match self.execute_stmt(stmt, sym_table)? {
                        Some(ControlAction::Break) => break None,
                        Some(ControlAction::Continue) => {}
                        Some(ControlAction::Return(val)) => break Some(ControlAction::Return(val)),
                        None => {}
                    }
                };

                sym_table.exit_scope();
                result
            }
            Stmt::Return(expr) => {
                let expr_val = expr
                    .as_ref()
                    .map(|expr| self.eval_expr(expr, sym_table))
                    .transpose()?;

                Some(ControlAction::Return(expr_val))
            }
            Stmt::Break => Some(ControlAction::Break),
            Stmt::Continue => Some(ControlAction::Continue),
            Stmt::AssignInPlace(op, dst_name, expr) => {
                let val = self.eval_expr(expr, sym_table)?;

                let dst_val = sym_table
                    .get_mut(dst_name)
                    .ok_or_else(|| ExecuteError::UndeclaredVariable(dst_name.clone()))?;

                match dst_val {
                    Primitive::Int(n) => match op {
                        InPlaceOp::Add => *n += val.as_int()?,
                        InPlaceOp::Sub => *n -= val.as_int()?,
                        InPlaceOp::Mul => *n *= val.as_int()?,
                        InPlaceOp::Div => *n /= val.as_int()?,
                        InPlaceOp::Mod => *n %= val.as_int()?,
                    },
                    Primitive::List(items) => items.borrow_mut().push(val),
                    Primitive::Str(s) => {
                        let val_str = val.to_string();
                        s.borrow_mut().push_str(&val_str);
                    }
                    _ => {
                        return Err(ExecuteError::InvalidAssignOp {
                            dst_kind: dst_val.kind(),
                            op: *op,
                        });
                    }
                }

                None
            }
        };

        Ok(result)
    }

    fn eval_expr(&mut self, expr: &Expr, sym_table: &mut SymbolTable) -> ExecuteResult<Primitive> {
        match expr {
            Expr::Int(n) => Ok(Primitive::Int(*n as i64)),
            Expr::Bool(b) => Ok(Primitive::Bool(*b)),
            Expr::Null => Ok(Primitive::Null),
            Expr::Iden(name) => sym_table
                .get(name)
                .cloned()
                .ok_or_else(|| ExecuteError::UndeclaredVariable(name.clone())),
            Expr::Str(s) => Ok(Primitive::str(s.clone())),
            Expr::List(item_exprs) => {
                let items = item_exprs
                    .iter()
                    .map(|expr| self.eval_expr(expr, sym_table))
                    .collect::<Result<_, _>>()?;

                Ok(Primitive::list(items))
            }
            Expr::BinOp(op, lhs, rhs) => match op {
                BinOp::Add => {
                    let lhs_val = self.eval_expr(lhs, sym_table)?;
                    let rhs_val = self.eval_expr(rhs, sym_table)?;

                    match (lhs_val, rhs_val) {
                        (lhs_val, Primitive::Str(rhs_str)) => {
                            let mut s_result = lhs_val.to_string();
                            s_result.push_str(&*rhs_str.borrow());
                            Ok(Primitive::str(s_result))
                        }
                        (Primitive::Str(lhs_str), rhs_val) => {
                            let mut s_result = lhs_str.borrow().clone();
                            s_result.push_str(&rhs_val.to_string());
                            Ok(Primitive::str(s_result))
                        }
                        (lhs_val, rhs_val) => {
                            Ok(Primitive::Int(lhs_val.as_int()? + rhs_val.as_int()?))
                        }
                    }
                }
                BinOp::Sub => {
                    let lhs_val = *self.eval_expr(lhs, sym_table)?.as_int()?;
                    let rhs_val = *self.eval_expr(rhs, sym_table)?.as_int()?;
                    Ok(Primitive::Int(lhs_val - rhs_val))
                }
                BinOp::Mul => {
                    let lhs_val = *self.eval_expr(lhs, sym_table)?.as_int()?;
                    let rhs_val = *self.eval_expr(rhs, sym_table)?.as_int()?;
                    Ok(Primitive::Int(lhs_val * rhs_val))
                }
                BinOp::Div => {
                    let lhs_val = *self.eval_expr(lhs, sym_table)?.as_int()?;
                    let rhs_val = *self.eval_expr(rhs, sym_table)?.as_int()?;
                    Ok(Primitive::Int(lhs_val / rhs_val))
                }
                BinOp::Mod => {
                    let lhs_val = *self.eval_expr(lhs, sym_table)?.as_int()?;
                    let rhs_val = *self.eval_expr(rhs, sym_table)?.as_int()?;
                    Ok(Primitive::Int(lhs_val % rhs_val))
                }
                BinOp::BoolAnd => Ok(Primitive::Bool(
                    *self.eval_expr(lhs, sym_table)?.as_bool()?
                        && *self.eval_expr(rhs, sym_table)?.as_bool()?,
                )),
                BinOp::BoolOr => Ok(Primitive::Bool(
                    *self.eval_expr(lhs, sym_table)?.as_bool()?
                        || *self.eval_expr(rhs, sym_table)?.as_bool()?,
                )),
            },
            Expr::UnaryOp(op, expr) => {
                let val = self.eval_expr(expr, sym_table)?;

                match op {
                    UnaryOp::Plus => Ok(Primitive::Int(*val.as_int()?)),
                    UnaryOp::Minus => Ok(Primitive::Int(-*val.as_int()?)),
                    UnaryOp::BoolNot => Ok(Primitive::Bool(!*val.as_bool()?)),
                }
            }
            Expr::Rel(rel, lhs, rhs) => {
                let lhs_val = self.eval_expr(lhs, sym_table)?;
                let rhs_val = self.eval_expr(rhs, sym_table)?;

                let result = match rel {
                    Rel::Eq => lhs_val == rhs_val,
                    Rel::Neq => lhs_val != rhs_val,
                    Rel::Gt => lhs_val.as_int()? > rhs_val.as_int()?,
                    Rel::Geq => lhs_val.as_int()? >= rhs_val.as_int()?,
                    Rel::Lt => lhs_val.as_int()? < rhs_val.as_int()?,
                    Rel::Leq => lhs_val.as_int()? <= rhs_val.as_int()?,
                };

                Ok(Primitive::Bool(result))
            }
            Expr::Ternary {
                cond,
                if_yes,
                if_no,
            } => {
                if *self.eval_expr(cond, sym_table)?.as_bool()? {
                    self.eval_expr(if_yes, sym_table)
                } else {
                    self.eval_expr(if_no, sym_table)
                }
            }
            Expr::FuncCall(func_name, arg_exprs) => {
                let args = arg_exprs
                    .iter()
                    .map(|expr| self.eval_expr(expr, sym_table))
                    .collect::<Result<_, _>>()?;

                self.execute_func(func_name, args)
            }
            Expr::ListAccess { list, idx } => {
                let list_val = self.eval_expr(list, sym_table)?;
                let idx_val = *self.eval_expr(idx, sym_table)?.as_int()?;

                let idx_usize: usize = idx_val
                    .try_into()
                    .map_err(|_| ExecuteError::ListIndexOutOfBounds(idx_val))?;

                list_val
                    .as_list()?
                    .borrow()
                    .get(idx_usize)
                    .cloned()
                    .ok_or(ExecuteError::ListIndexOutOfBounds(idx_val))
            }
        }
    }
}
