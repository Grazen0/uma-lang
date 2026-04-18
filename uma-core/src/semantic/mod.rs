use std::collections::HashMap;

use crate::{
    parser::ast::{Expr, Func, LValue, Program, Stmt},
    util::{Position, Span, Spanned},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Variable,
    Function,
}

#[derive(Debug, Clone, Default)]
struct Scope<'a> {
    vars: HashMap<String, usize>,
    parent: Option<&'a Scope<'a>>,
}

impl<'a> Scope<'a> {
    pub fn over(parent: &'a Scope<'a>) -> Self {
        Self {
            vars: HashMap::new(),
            parent: Some(parent),
        }
    }

    pub fn get(&self, name: &str) -> Option<usize> {
        self.vars
            .get(name)
            .cloned()
            .or_else(|| self.parent.and_then(|par| par.get(name)))
    }

    pub fn insert(&mut self, name: String, sym_idx: usize) {
        self.vars.insert(name, sym_idx);
    }
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: Spanned<String>,
    pub kind: SymbolKind,
    pub refs: Vec<Span>,
}

impl Symbol {
    pub fn new(name: Spanned<String>, kind: SymbolKind) -> Self {
        Self {
            name,
            kind,
            refs: vec![],
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SemanticModel {
    symbols: Vec<Symbol>,
    refs: HashMap<Span, usize>,
}

impl From<&Program> for SemanticModel {
    fn from(program: &Program) -> Self {
        let mut model = Self::default();

        let mut global_scope = Scope::default();

        for func in &program.funcs {
            let new_sym = Symbol::new(func.val.name.clone(), SymbolKind::Function);

            let idx = model.add_symbol(new_sym);
            global_scope.insert(func.val.name.val.clone(), idx);
        }

        for func in &program.funcs {
            model.visit_func(func, &mut global_scope);
        }

        model
    }
}

impl SemanticModel {
    pub fn symbol_lookup(&self, position: Position) -> Option<&Symbol> {
        // PERF: could improve with binary search
        self.symbols
            .iter()
            .find(|sym| sym.name.span.contains(position))
            .or_else(|| {
                self.refs
                    .iter()
                    .find_map(|(span, &idx)| span.contains(position).then_some(idx))
                    .map(|sym_idx| &self.symbols[sym_idx])
            })
    }

    fn add_symbol(&mut self, symbol: Symbol) -> usize {
        let idx = self.symbols.len();
        self.symbols.push(symbol);
        idx
    }

    fn visit_func(&mut self, func: &Spanned<Func>, scope: &mut Scope<'_>) {
        let mut new_scope = Scope::over(scope);

        for arg in &func.val.args {
            let sym_idx = self.add_symbol(Symbol::new(arg.clone(), SymbolKind::Variable));
            new_scope.insert(arg.val.clone(), sym_idx);
        }

        self.visit_stmts(&func.val.stmts, &mut new_scope);
    }

    fn visit_stmts(&mut self, stmts: &[Spanned<Stmt>], scope: &mut Scope<'_>) {
        for stmt in stmts {
            self.visit_stmt(stmt, scope);
        }
    }

    fn visit_stmt(&mut self, stmt: &Spanned<Stmt>, scope: &mut Scope<'_>) {
        match &stmt.val {
            Stmt::Break | Stmt::Continue | Stmt::Return(None) => {}
            Stmt::Expr(expr) | Stmt::Return(Some(expr)) => self.visit_expr(expr, scope),
            Stmt::Block(stmts) => {
                let mut new_scope = Scope::over(scope);
                self.visit_stmts(stmts, &mut new_scope);
            }
            Stmt::If {
                cond,
                stmt,
                else_stmt,
            } => {
                self.visit_expr(cond, scope);

                let mut if_yes_scope = Scope::over(scope);
                self.visit_stmt(stmt, &mut if_yes_scope);

                if let Some(else_stmt) = else_stmt {
                    let mut if_no_scope = Scope::over(scope);
                    self.visit_stmt(else_stmt, &mut if_no_scope);
                }
            }
            Stmt::While {
                cond,
                stmt,
                cont_expr,
            } => {
                self.visit_expr(cond, scope);

                if let Some(expr) = cont_expr {
                    self.visit_expr(expr, scope);
                }

                self.visit_stmt(stmt, scope);
            }
            Stmt::Loop(stmt) => self.visit_stmt(stmt, scope),
        }
    }

    fn scoped_refer(&mut self, iden: &Spanned<String>, scope: &Scope<'_>) -> bool {
        let Some(sym_idx) = scope.get(&iden.val) else {
            return false;
        };

        let symbol = &mut self.symbols[sym_idx];
        symbol.refs.push(iden.span.clone());
        self.refs.insert(iden.span.clone(), sym_idx);
        true
    }

    fn visit_expr(&mut self, expr: &Spanned<Expr>, scope: &mut Scope<'_>) {
        match &expr.val {
            Expr::IntLit(..) | Expr::StrLit(..) | Expr::BoolLit(..) | Expr::NullLit => {}
            Expr::BinOp(_, lhs, rhs) | Expr::Rel(_, lhs, rhs) => {
                self.visit_expr(lhs, scope);
                self.visit_expr(rhs, scope);
            }
            Expr::Ternary {
                cond,
                if_yes,
                if_no,
            } => {
                self.visit_expr(cond, scope);
                self.visit_expr(if_yes, scope);
                self.visit_expr(if_no, scope);
            }
            Expr::Access { value, idx } => {
                self.visit_expr(value, scope);
                self.visit_expr(idx, scope);
            }
            Expr::UnaryOp(_, expr) => self.visit_expr(expr, scope),
            Expr::ListLit(items) => {
                for item in items {
                    self.visit_expr(item, scope);
                }
            }
            Expr::DictLit(items) => {
                for (key, val) in items {
                    self.visit_expr(key, scope);
                    self.visit_expr(val, scope);
                }
            }
            Expr::Iden(name) => {
                self.scoped_refer(name, scope);
            }

            Expr::Assign(lval, expr) => {
                if !self.visit_lval(lval, scope) {
                    if let LValue::Iden(name) = &lval.val {
                        let sym_idx =
                            self.add_symbol(Symbol::new(name.clone(), SymbolKind::Variable));
                        scope.insert(name.val.clone(), sym_idx);
                    } else {
                        todo!()
                    }
                }

                self.visit_expr(expr, scope);
            }
            Expr::Modify(_, lval, expr) => {
                self.visit_lval(lval, scope);
                self.visit_expr(expr, scope);
            }
            Expr::FuncCall(func_name, args) => {
                self.scoped_refer(func_name, scope);

                for arg in args {
                    self.visit_expr(arg, scope);
                }
            }
        }
    }

    fn visit_lval(&mut self, lval: &Spanned<LValue>, scope: &mut Scope<'_>) -> bool {
        match &lval.val {
            LValue::Access(inner_lval, _) => self.visit_lval(inner_lval, scope),
            LValue::Iden(name) => self.scoped_refer(name, scope),
        }
    }
}
