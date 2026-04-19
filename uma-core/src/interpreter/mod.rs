mod builtins;
mod core;
mod scope;

use std::{
    collections::HashMap,
    rc::Rc,
    sync::{Arc, LazyLock},
};

use crate::{
    interpreter::{
        core::{DictKey, ExecuteError, ExecuteResult, Value},
        scope::{Function, FunctionScope, Scope, ValueModifier},
    },
    parser::ast::{AssignOp, BinOp, Expr, LValue, Program, Rel, Stmt, UnaryOp},
    util::Spanned,
};

#[derive(Debug, Clone)]
enum ControlFlow {
    Return(Option<Value>),
    Break,
    Continue,
}

static GLOBAL_FUNCS: LazyLock<Arc<FunctionScope<Interpreter>>> = LazyLock::new(|| {
    let mut s = FunctionScope::new();

    s.insert_local("print".to_string(), Function::BuiltIn(builtins::print))
        .unwrap();
    s.insert_local("len".to_string(), Function::BuiltIn(builtins::len))
        .unwrap();

    Arc::new(s)
});

#[derive(Debug)]
pub struct Interpreter {
    global_scope: Rc<Scope>,
    user_funcs: FunctionScope<Self>,
}

impl Interpreter {
    pub fn new(program: &Program) -> ExecuteResult<Self> {
        let mut user_funcs = FunctionScope::over(GLOBAL_FUNCS.clone());

        for func in &program.funcs {
            user_funcs.insert_local(func.val.name.val.clone(), Function::UserDef(func.clone()))?;
        }

        Ok(Self {
            global_scope: Rc::default(),
            user_funcs,
        })
    }

    pub fn execute(&mut self) -> ExecuteResult<()> {
        self.execute_func("main", vec![])?;
        Ok(())
    }

    fn execute_func(&mut self, func_name: &str, args: Vec<Value>) -> ExecuteResult<Option<Value>> {
        let func = self
            .user_funcs
            .get(func_name)
            .cloned()
            .ok_or_else(|| ExecuteError::UndeclaredFunction(func_name.to_string()))?;

        match func.as_ref() {
            Function::UserDef(func) => {
                core::expect_arg_count(func.val.params.len(), args.len())?;

                let func_scope = Scope::over(self.global_scope.clone());

                for (param, arg) in func.val.params.iter().zip(args) {
                    func_scope.decl_local(
                        param.val.name.val.clone(),
                        arg,
                        param.val.mutable.is_some(),
                    )?;
                }

                let result = self.execute_stmts(&func.val.stmts, &Rc::new(func_scope))?;

                match result {
                    None => Ok(None),
                    Some(ControlFlow::Break) => Err(ExecuteError::UnexpectedBreak),
                    Some(ControlFlow::Continue) => Err(ExecuteError::UnexpectedContinue),
                    Some(ControlFlow::Return(val)) => Ok(val),
                }
            }
            Function::BuiltIn(f) => f(self, args),
        }
    }

    fn execute_stmts(
        &mut self,
        stmts: &[Spanned<Stmt>],
        scope: &Rc<Scope>,
    ) -> ExecuteResult<Option<ControlFlow>> {
        for stmt in stmts {
            if let Some(val) = self.execute_stmt(stmt, scope)? {
                return Ok(Some(val));
            }
        }

        Ok(None)
    }

    fn execute_stmt(
        &mut self,
        stmt: &Spanned<Stmt>,
        scope: &Rc<Scope>,
    ) -> ExecuteResult<Option<ControlFlow>> {
        let result = match &stmt.val {
            Stmt::VarDecl {
                name,
                init_expr,
                mutable,
            } => {
                let val = self.eval_expr(init_expr, scope)?;
                scope.decl_local(name.val.clone(), val, mutable.is_some())?;
                None
            }
            Stmt::Expr(expr) => {
                self.eval_expr(expr, scope)?;
                None
            }
            Stmt::Block(stmts) => {
                let new_scope = Rc::new(Scope::over(scope.clone()));
                self.execute_stmts(stmts, &new_scope)?
            }
            Stmt::If {
                cond,
                stmt,
                else_stmt,
            } => {
                if *self.eval_expr(cond, scope)?.as_bool()? {
                    let new_scope = Rc::new(Scope::over(scope.clone()));
                    self.execute_stmt(stmt, &new_scope)?
                } else if let Some(else_stmt) = else_stmt.as_ref() {
                    let new_scope = Rc::new(Scope::over(scope.clone()));
                    self.execute_stmt(else_stmt, &new_scope)?
                } else {
                    None
                }
            }
            Stmt::While {
                cond,
                stmt,
                cont_expr,
            } => {
                let new_scope = Rc::new(Scope::over(scope.clone()));

                loop {
                    if !*self.eval_expr(cond, scope)?.as_bool()? {
                        break None;
                    }

                    match self.execute_stmt(stmt, &new_scope)? {
                        Some(ControlFlow::Break) => break None,
                        Some(ControlFlow::Return(val)) => break Some(ControlFlow::Return(val)),
                        None | Some(ControlFlow::Continue) => {
                            if let Some(expr) = cont_expr {
                                self.eval_expr(expr, scope)?;
                            }
                        }
                    }
                }
            }
            Stmt::Loop(stmt) => {
                let new_scope = Rc::new(Scope::over(scope.clone()));

                loop {
                    match self.execute_stmt(stmt, &new_scope)? {
                        Some(ControlFlow::Break) => break None,
                        Some(ControlFlow::Continue) => {}
                        Some(ControlFlow::Return(val)) => break Some(ControlFlow::Return(val)),
                        None => {}
                    }
                }
            }
            Stmt::Return(expr) => {
                let expr_val = expr
                    .as_ref()
                    .map(|expr| self.eval_expr(expr, scope))
                    .transpose()?;

                Some(ControlFlow::Return(expr_val))
            }
            Stmt::Break => Some(ControlFlow::Break),
            Stmt::Continue => Some(ControlFlow::Continue),
        };

        Ok(result)
    }

    fn with_lval(
        &mut self,
        lval: &Spanned<LValue>,
        scope: &Rc<Scope>,
        f: Box<ValueModifier>,
    ) -> ExecuteResult<()> {
        match &lval.val {
            LValue::Iden(name) => scope.mutate(&name.val, f),
            LValue::Access(sub_lval, idx_expr) => {
                let idx_val = self.eval_expr(idx_expr, scope)?;

                self.with_lval(
                    sub_lval,
                    scope,
                    Box::new(move |val| match val {
                        Value::List(items) => {
                            let idx_int = *idx_val.as_int()?;

                            let mut items_ref = items.borrow_mut();
                            let val = idx_int
                                .try_into()
                                .ok()
                                .and_then(|i: usize| items_ref.get_mut(i))
                                .ok_or(ExecuteError::IndexOutOfBounds(idx_int))?;

                            f(val)
                        }
                        Value::Dict(items) => {
                            let idx_sym = DictKey::try_from(idx_val.clone())?;

                            let mut items_ref = items.borrow_mut();
                            let val = items_ref
                                .get_mut(&idx_sym)
                                .ok_or(ExecuteError::DictKeyNotFound(idx_sym))?;

                            f(val)
                        }
                        other => Err(ExecuteError::VarNotIndexable(other.kind())),
                    }),
                )
            }
        }
    }

    fn eval_expr(&mut self, expr: &Spanned<Expr>, scope: &Rc<Scope>) -> ExecuteResult<Value> {
        match &expr.val {
            Expr::Assign(op, lval, expr) => {
                let val = self.eval_expr(expr, scope)?;
                let val_clone = val.clone();
                let op = op.val;

                self.with_lval(
                    lval,
                    scope,
                    Box::new(move |dst| {
                        match (dst, op) {
                            (dst_val, AssignOp::Assign) => *dst_val = val_clone,
                            (Value::Int(n), AssignOp::Add) => *n += val_clone.as_int()?,
                            (Value::Int(n), AssignOp::Sub) => *n -= val_clone.as_int()?,
                            (Value::Int(n), AssignOp::Mul) => *n *= val_clone.as_int()?,
                            (Value::Int(n), AssignOp::Div) => *n /= val_clone.as_int()?,
                            (Value::Int(n), AssignOp::Mod) => *n %= val_clone.as_int()?,
                            (Value::List(items), AssignOp::Add) => {
                                let val_list = val_clone.as_list()?;
                                items.borrow_mut().append(&mut val_list.borrow().clone());
                            }
                            (Value::Str(s), AssignOp::Add) => {
                                let val_str = val_clone.to_string();
                                s.borrow_mut().push_str(&val_str);
                            }
                            (dst_val, op) => {
                                return Err(ExecuteError::InvalidAssignment {
                                    dst_kind: dst_val.kind(),
                                    op,
                                });
                            }
                        }
                        Ok(())
                    }),
                )?;

                Ok(val)
            }
            Expr::IntLit(n) => Ok(Value::Int(*n as i64)),
            Expr::BoolLit(b) => Ok(Value::Bool(*b)),
            Expr::NullLit => Ok(Value::Null),
            Expr::Iden(name) => scope
                .find(&name.val)
                .ok_or_else(|| ExecuteError::UndeclaredVariable(name.val.clone())),
            Expr::StrLit(s) => Ok(Value::str(s.clone())),
            Expr::ListLit(item_exprs) => {
                let items = item_exprs
                    .iter()
                    .map(|expr| self.eval_expr(expr, scope))
                    .collect::<Result<_, _>>()?;

                Ok(Value::list(items))
            }
            Expr::DictLit(entry_exprs) => {
                let mut items = HashMap::new();

                for (key_expr, val_expr) in entry_exprs {
                    let key = self.eval_expr(key_expr, scope)?;
                    let key_sym = DictKey::try_from(key)?;

                    let val = self.eval_expr(val_expr, scope)?;

                    items.insert(key_sym, val);
                }

                Ok(Value::dict(items))
            }
            Expr::BinOp(op, lhs, rhs) => match op.val {
                BinOp::Add => {
                    let lhs_val = self.eval_expr(lhs, scope)?;
                    let rhs_val = self.eval_expr(rhs, scope)?;

                    match (lhs_val, rhs_val) {
                        (lhs_val, Value::Str(rhs_str)) => {
                            let mut out_str = lhs_val.to_string();
                            out_str.push_str(&rhs_str.borrow());
                            Ok(Value::str(out_str))
                        }
                        (Value::Str(lhs_str), rhs_val) => {
                            let mut out_str = lhs_str.borrow().clone();
                            out_str.push_str(&rhs_val.to_string());
                            Ok(Value::str(out_str))
                        }
                        (Value::List(lhs), Value::List(rhs)) => {
                            let mut out_items = lhs.borrow().clone();
                            out_items.append(&mut rhs.borrow().clone());
                            Ok(Value::list(out_items))
                        }
                        (lhs_val, rhs_val) => Ok(Value::Int(lhs_val.as_int()? + rhs_val.as_int()?)),
                    }
                }
                BinOp::Sub => {
                    let lhs_val = *self.eval_expr(lhs, scope)?.as_int()?;
                    let rhs_val = *self.eval_expr(rhs, scope)?.as_int()?;
                    Ok(Value::Int(lhs_val - rhs_val))
                }
                BinOp::Mul => {
                    let lhs_val = *self.eval_expr(lhs, scope)?.as_int()?;
                    let rhs_val = *self.eval_expr(rhs, scope)?.as_int()?;
                    Ok(Value::Int(lhs_val * rhs_val))
                }
                BinOp::Div => {
                    let lhs_val = *self.eval_expr(lhs, scope)?.as_int()?;
                    let rhs_val = *self.eval_expr(rhs, scope)?.as_int()?;
                    Ok(Value::Int(lhs_val / rhs_val))
                }
                BinOp::Mod => {
                    let lhs_val = *self.eval_expr(lhs, scope)?.as_int()?;
                    let rhs_val = *self.eval_expr(rhs, scope)?.as_int()?;
                    Ok(Value::Int(lhs_val % rhs_val))
                }
                BinOp::BoolAnd => Ok(Value::Bool(
                    *self.eval_expr(lhs, scope)?.as_bool()?
                        && *self.eval_expr(rhs, scope)?.as_bool()?,
                )),
                BinOp::BoolOr => Ok(Value::Bool(
                    *self.eval_expr(lhs, scope)?.as_bool()?
                        || *self.eval_expr(rhs, scope)?.as_bool()?,
                )),
            },
            Expr::UnaryOp(op, expr) => {
                let val = self.eval_expr(expr, scope)?;

                match op.val {
                    UnaryOp::Plus => Ok(Value::Int(*val.as_int()?)),
                    UnaryOp::Minus => Ok(Value::Int(-*val.as_int()?)),
                    UnaryOp::BoolNot => Ok(Value::Bool(!*val.as_bool()?)),
                }
            }
            Expr::Rel(rel, lhs, rhs) => {
                let lhs_val = self.eval_expr(lhs, scope)?;
                let rhs_val = self.eval_expr(rhs, scope)?;

                let result = match rel.val {
                    Rel::Eq => lhs_val == rhs_val,
                    Rel::Neq => lhs_val != rhs_val,
                    Rel::Gt => lhs_val.as_int()? > rhs_val.as_int()?,
                    Rel::Geq => lhs_val.as_int()? >= rhs_val.as_int()?,
                    Rel::Lt => lhs_val.as_int()? < rhs_val.as_int()?,
                    Rel::Leq => lhs_val.as_int()? <= rhs_val.as_int()?,
                };

                Ok(Value::Bool(result))
            }
            Expr::Ternary {
                cond,
                if_yes,
                if_no,
            } => {
                if *self.eval_expr(cond, scope)?.as_bool()? {
                    self.eval_expr(if_yes, scope)
                } else {
                    self.eval_expr(if_no, scope)
                }
            }
            Expr::FuncCall(func_name, arg_exprs) => {
                let args = arg_exprs
                    .iter()
                    .map(|expr| self.eval_expr(expr, scope))
                    .collect::<Result<_, _>>()?;

                Ok(self
                    .execute_func(&func_name.val, args)?
                    .unwrap_or(Value::Null))
            }
            Expr::Access { value, idx } => {
                let value_val = self.eval_expr(value, scope)?;
                let idx_val = self.eval_expr(idx, scope)?;

                match value_val {
                    Value::List(items) => {
                        let idx_int = *idx_val.as_int()?;
                        let idx_usize: usize = idx_int
                            .try_into()
                            .map_err(|_| ExecuteError::IndexOutOfBounds(idx_int))?;

                        items
                            .borrow()
                            .get(idx_usize)
                            .cloned()
                            .ok_or(ExecuteError::IndexOutOfBounds(idx_int))
                    }
                    Value::Dict(items) => {
                        let idx_sym = DictKey::try_from(idx_val)?;
                        let val = items
                            .borrow()
                            .get(&idx_sym)
                            .cloned()
                            .ok_or(ExecuteError::DictKeyNotFound(idx_sym))?;

                        Ok(val)
                    }
                    other => Err(ExecuteError::VarNotIndexable(other.kind())),
                }
            }
        }
    }
}
