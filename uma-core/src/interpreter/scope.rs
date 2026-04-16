use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::interpreter::core::{ExecuteResult, Value};

pub type ValueModifier = dyn FnOnce(&mut Value) -> ExecuteResult<()>;

#[derive(Debug, Default)]
pub struct Scope {
    vars: RefCell<HashMap<String, Value>>,
    parent: Option<Rc<Self>>,
}

impl Scope {
    pub fn over(parent: Rc<Self>) -> Self {
        Self {
            parent: Some(parent),
            ..Default::default()
        }
    }

    pub fn get_cloned(&self, name: &str) -> Option<Value> {
        let vars_ref = self.vars.borrow();

        vars_ref.get(name).cloned().or_else(|| {
            self.parent
                .as_ref()
                .and_then(|parent| parent.get_cloned(name))
        })
    }

    pub fn with_var(&self, name: &str, f: Box<ValueModifier>) -> ExecuteResult<bool> {
        let mut vars_ref = self.vars.borrow_mut();

        if let Some(val) = vars_ref.get_mut(name) {
            f(val)?;
            Ok(true)
        } else if let Some(next) = &self.parent {
            next.with_var(name, f)
        } else {
            Ok(false)
        }
    }

    pub fn insert(&self, name: String, val: Value) {
        let val_clone = val.clone();

        let modify_result = self
            .with_var(
                &name,
                Box::new(move |dst| {
                    *dst = val_clone;
                    Ok(())
                }),
            )
            .unwrap(); // should never fail

        if !modify_result {
            let mut vars_ref = self.vars.borrow_mut();
            vars_ref.insert(name, val);
        }
    }
}
