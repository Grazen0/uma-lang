use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Arc};

use derive_more::Constructor;

use crate::{
    interpreter::core::{ExecuteError, ExecuteResult, Value},
    parser::ast::Func,
    util::Spanned,
};

pub type ValueModifier = dyn FnOnce(&mut Value) -> ExecuteResult<()>;

pub type BuiltInFn<I> = fn(&mut I, Vec<Value>) -> ExecuteResult<Option<Value>>;

#[derive(Debug)]
pub enum Function<I> {
    BuiltIn(BuiltInFn<I>),
    UserDef(Spanned<Func>),
}

#[derive(Debug)]
pub struct FunctionScope<I> {
    funcs: HashMap<String, Arc<Function<I>>>,
    parent: Option<Arc<FunctionScope<I>>>,
}

impl<I> FunctionScope<I> {
    pub fn new() -> Self {
        Self {
            funcs: HashMap::new(),
            parent: None,
        }
    }

    pub fn over(parent: Arc<FunctionScope<I>>) -> Self {
        Self {
            funcs: HashMap::new(),
            parent: Some(parent),
        }
    }

    pub fn get(&self, name: &str) -> Option<&Arc<Function<I>>> {
        self.funcs
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|parent| parent.get(name)))
    }

    pub fn insert_local(&mut self, name: String, value: Function<I>) -> ExecuteResult<()> {
        if self.funcs.contains_key(&name) {
            return Err(ExecuteError::FuncRedeclared(name));
        }

        self.funcs.insert(name, Arc::new(value));
        Ok(())
    }
}

#[derive(Debug, Clone, Constructor)]
pub struct Variable {
    value: Value,
    mutable: bool,
}

#[derive(Debug, Default)]
pub struct Scope {
    vars: RefCell<HashMap<String, Variable>>,
    parent: Option<Rc<Self>>,
}

impl Scope {
    pub fn over(parent: Rc<Self>) -> Self {
        Self {
            parent: Some(parent),
            ..Default::default()
        }
    }

    pub fn get_value(&self, name: &str) -> Option<Value> {
        let vars_ref = self.vars.borrow();

        vars_ref.get(name).map(|var| var.value.clone()).or_else(|| {
            self.parent
                .as_ref()
                .and_then(|parent| parent.get_value(name))
        })
    }

    pub fn mutate_var(&self, name: &str, f: Box<ValueModifier>) -> ExecuteResult<()> {
        let mut vars_ref = self.vars.borrow_mut();

        if let Some(val) = vars_ref.get_mut(name) {
            if !val.mutable {
                return Err(ExecuteError::CannotMutateVar(name.to_string()));
            }

            f(&mut val.value)?;
            Ok(())
        } else if let Some(next) = &self.parent {
            next.mutate_var(name, f)
        } else {
            Err(ExecuteError::UndeclaredVariable(name.to_string()))
        }
    }

    pub fn decl_var(&self, name: String, init_val: Value, mutable: bool) -> ExecuteResult<()> {
        let mut vars_ref = self.vars.borrow_mut();

        if vars_ref.contains_key(&name) {
            return Err(ExecuteError::VarRedeclared(name));
        }

        vars_ref.insert(name, Variable::new(init_val, mutable));
        Ok(())
    }
}
