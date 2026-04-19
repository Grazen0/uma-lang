use std::collections::HashMap;

use derive_more::Display;

use crate::{
    parser::ast::{Expr, Func, LValue, Program, Stmt},
    util::{Position, Span, Spanned},
};

#[derive(Debug, Clone, Default)]
struct Scope<'a> {
    vars: HashMap<String, usize>,
    parent: Option<&'a Scope<'a>>,
}

impl<'a> Scope<'a> {
    pub fn over(parent: &'a Scope) -> Self {
        Self {
            vars: HashMap::new(),
            parent: Some(parent),
        }
    }

    pub fn get(&self, name: &str) -> Option<usize> {
        self.vars
            .get(name)
            .cloned()
            .or_else(|| self.parent.as_ref().and_then(|par| par.get(name)))
    }

    #[must_use]
    pub fn insert(&mut self, name: String, sym_idx: usize) -> bool {
        if self.vars.contains_key(&name) {
            return false;
        }

        self.vars.insert(name, sym_idx);
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamCount {
    Fixed(usize),
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
pub enum SymbolKind {
    #[display("variable")]
    ImmutableVariable,

    #[display("variable")]
    MutableVariable { mutated: bool },

    #[display("function")]
    Function(ParamCount),
}

impl SymbolKind {
    pub fn variable(mutable: bool) -> Self {
        if mutable {
            Self::MutableVariable { mutated: false }
        } else {
            Self::ImmutableVariable
        }
    }
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub span: Option<Span>,
    pub kind: SymbolKind,
    pub refs: Vec<Span>,
    pub anon_ref: bool,
}

impl Symbol {
    pub fn new(name: String, span: Option<Span>, kind: SymbolKind) -> Self {
        Self {
            name,
            span,
            kind,
            refs: vec![],
            anon_ref: false,
        }
    }

    pub fn from_spanned(name: Spanned<String>, kind: SymbolKind) -> Self {
        Self {
            name: name.val,
            span: Some(name.span),
            kind,
            refs: vec![],
            anon_ref: false,
        }
    }

    pub fn is_used(&self) -> bool {
        !self.refs.is_empty() || self.anon_ref
    }
}

#[derive(Debug, Clone, Display)]
pub enum SemanticError {
    #[display("undefined variable: `{}`", _0.val)]
    UndefinedVar(Spanned<String>),

    #[display("undefined function: `{}`", _0.val)]
    UndefinedFunc(Spanned<String>),

    #[display("variable is not callable: `{}`", _0.val)]
    VarNotCallable(Spanned<String>),

    #[display("function redeclared: `{}`", _0.val)]
    FuncRedeclared(Spanned<String>),

    #[display("variable redeclared: `{}`", _0.val)]
    VarRedeclared(Spanned<String>),

    #[display("function is not assignable: `{}`", _0.val)]
    FuncNotAssignable(Spanned<String>),

    #[display("cannot mutate immutable variable `{}`", _0.val)]
    CannotMutateVar(Spanned<String>),

    #[display("duplicate function parameter: `{}`", _0.val)]
    DuplicateParam(Spanned<String>),

    #[display("function `{}` expected {expected} arguments, got {got}", func_name.val)]
    ParamCountMismatch {
        func_name: Spanned<String>,
        expected: usize,
        got: usize,
    },
}

impl SemanticError {
    pub fn span(&self) -> &Span {
        match self {
            Self::UndefinedVar(name)
            | Self::UndefinedFunc(name)
            | Self::VarNotCallable(name)
            | Self::FuncRedeclared(name)
            | Self::VarRedeclared(name)
            | Self::FuncNotAssignable(name)
            | Self::CannotMutateVar(name)
            | Self::DuplicateParam(name) => &name.span,
            Self::ParamCountMismatch { func_name, .. } => &func_name.span,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SemanticModel {
    symbols: Vec<Symbol>,
    refs: HashMap<Span, usize>,
    errors: Vec<SemanticError>,
}

impl From<&Program> for SemanticModel {
    fn from(program: &Program) -> Self {
        let mut model = Self::default();
        model.visit_program(program);
        model
    }
}

impl SemanticModel {
    pub fn symbols(&self) -> &[Symbol] {
        &self.symbols
    }

    pub fn errors(&self) -> &[SemanticError] {
        &self.errors
    }

    pub fn symbol_lookup(&self, pos: Position) -> Option<&Symbol> {
        // PERF: could improve with binary search
        self.symbols
            .iter()
            .find(|sym| sym.span.as_ref().is_some_and(|span| span.contains(pos)))
            .or_else(|| {
                self.refs
                    .iter()
                    .find_map(|(span, &idx)| span.contains(pos).then_some(idx))
                    .map(|sym_idx| &self.symbols[sym_idx])
            })
    }

    fn add_symbol(&mut self, symbol: Symbol) -> usize {
        let idx = self.symbols.len();
        self.symbols.push(symbol);
        idx
    }

    fn add_anonymous_symbol(&mut self, name: String, kind: SymbolKind, scope: &mut Scope<'_>) {
        let sym_idx = self.add_symbol(Symbol::new(name.clone(), None, kind));
        assert!(scope.insert(name, sym_idx));
    }

    fn visit_program(&mut self, program: &Program) {
        let mut std_scope = Scope::default();

        self.add_anonymous_symbol(
            "print".to_string(),
            SymbolKind::Function(ParamCount::Any),
            &mut std_scope,
        );
        self.add_anonymous_symbol(
            "len".to_string(),
            SymbolKind::Function(ParamCount::Fixed(1)),
            &mut std_scope,
        );

        let mut file_scope = Scope::over(&std_scope);

        for func in &program.funcs {
            let new_sym = Symbol::from_spanned(
                func.val.name.clone(),
                SymbolKind::Function(ParamCount::Fixed(func.val.params.len())),
            );
            let sym_idx = self.add_symbol(new_sym);

            if !file_scope.insert(func.val.name.val.clone(), sym_idx) {
                self.errors
                    .push(SemanticError::FuncRedeclared(func.val.name.clone()));
            }

            if func.val.name.val == "main" {
                self.add_anonymous_ref(sym_idx);
            }
        }

        for func in &program.funcs {
            self.visit_func(func, &mut file_scope);
        }
    }

    fn visit_func(&mut self, func: &Spanned<Func>, scope: &mut Scope<'_>) {
        let mut new_scope = Scope::over(scope);

        for param in &func.val.params {
            let sym_idx = self.add_symbol(Symbol::from_spanned(
                param.val.name.clone(),
                SymbolKind::variable(param.val.mutable),
            ));

            if !new_scope.insert(param.val.name.val.clone(), sym_idx) {
                self.errors
                    .push(SemanticError::DuplicateParam(param.val.name.clone()));
            }
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
            Stmt::VarDecl {
                name,
                init_expr,
                mutable,
            } => {
                let new_sym = SymbolKind::variable(*mutable);
                let sym_idx = self.add_symbol(Symbol::from_spanned(name.clone(), new_sym));

                if !scope.insert(name.val.clone(), sym_idx) {
                    self.errors.push(SemanticError::VarRedeclared(name.clone()));

                    // This should count as a non-definition reference to the variable
                    self.add_symbol_ref(sym_idx, &name.span);
                }

                self.visit_expr(init_expr, scope);
            }
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

    fn add_symbol_ref(&mut self, symbol_idx: usize, span: &Span) {
        let symbol = &mut self.symbols[symbol_idx];
        symbol.refs.push(span.clone());
        self.refs.insert(span.clone(), symbol_idx);
    }

    fn add_anonymous_ref(&mut self, symbol_idx: usize) {
        let symbol = &mut self.symbols[symbol_idx];
        symbol.anon_ref = true;
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
                if let Some(sym_idx) = scope.get(&name.val) {
                    self.add_symbol_ref(sym_idx, &name.span);
                } else {
                    self.errors.push(SemanticError::UndefinedVar(name.clone()));
                }
            }
            Expr::Assign(_, lval, expr) => {
                self.visit_lval(lval, scope);
                self.visit_expr(expr, scope);
            }
            Expr::FuncCall(func_name, args) => {
                if let Some(sym_idx) = scope.get(&func_name.val) {
                    self.add_symbol_ref(sym_idx, &func_name.span);

                    let func_sym = &self.symbols[sym_idx];

                    let SymbolKind::Function(param_cnt) = func_sym.kind else {
                        self.errors
                            .push(SemanticError::VarNotCallable(func_name.clone()));
                        return;
                    };

                    if let ParamCount::Fixed(n) = param_cnt
                        && n != args.len()
                    {
                        self.errors.push(SemanticError::ParamCountMismatch {
                            func_name: func_name.clone(),
                            expected: n,
                            got: args.len(),
                        });
                    }
                } else {
                    self.errors
                        .push(SemanticError::UndefinedFunc(func_name.clone()));
                }

                for arg in args {
                    self.visit_expr(arg, scope);
                }
            }
        }
    }

    fn visit_lval(&mut self, lval: &Spanned<LValue>, scope: &mut Scope<'_>) {
        match &lval.val {
            LValue::Access(inner_lval, _) => self.visit_lval(inner_lval, scope),
            LValue::Iden(name) => {
                let Some(sym_idx) = scope.get(&name.val) else {
                    self.errors.push(SemanticError::UndefinedVar(name.clone()));
                    return;
                };

                self.add_symbol_ref(sym_idx, &name.span);
                let sym = &mut self.symbols[sym_idx];

                match &mut sym.kind {
                    SymbolKind::Function(..) => {
                        self.errors
                            .push(SemanticError::FuncNotAssignable(name.clone()));
                    }
                    SymbolKind::ImmutableVariable => {
                        self.errors
                            .push(SemanticError::CannotMutateVar(name.clone()));
                    }
                    SymbolKind::MutableVariable { mutated, .. } => {
                        *mutated = true;
                    }
                }
            }
        }
    }
}
