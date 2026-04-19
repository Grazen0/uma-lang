use std::collections::HashMap;

use derive_more::Display;
use kinded::Kinded;

use crate::{
    parser::ast::{BinOp, Expr, Func, LValue, Program, Rel, Stmt},
    util::{Combine, Position, Span, Spanned},
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

    pub fn find(&self, name: &str) -> Option<usize> {
        self.vars
            .get(name)
            .copied()
            .or_else(|| self.parent.as_ref().and_then(|par| par.find(name)))
    }

    #[must_use]
    pub fn insert_local(&mut self, name: String, sym_idx: usize) -> bool {
        if self.vars.contains_key(&name) {
            return false;
        }

        self.vars.insert(name, sym_idx);
        true
    }

    pub fn insert_local_shadowing(&mut self, name: String, sym_idx: usize) -> bool {
        self.vars.insert(name, sym_idx).is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamCount {
    Fixed(usize),
    Any,
}

#[derive(Debug, Clone, PartialEq, Eq, Display)]
pub enum SymbolValue {
    #[display("variable")]
    ImmutableVariable(Option<Value>),

    #[display("variable")]
    MutableVariable { mut_span: Span, mutated: bool },

    #[display("function")]
    Function(ParamCount),
}

impl SymbolValue {
    pub fn variable(mutable: Option<Span>, init_value: EvalValue) -> Self {
        if let Some(mut_span) = mutable {
            Self::MutableVariable {
                mut_span,
                mutated: false,
            }
        } else {
            Self::ImmutableVariable(init_value.map_const_or(None, Some))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Kinded)]
#[kinded(display = "lowercase")]
pub enum Value {
    Int(i64),
    Bool(bool),
    Null,
    Str(String),
    #[kinded("!")]
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvalValue {
    Const(Value),
    Unknown,
}

impl EvalValue {
    pub fn map_const_or<F, O>(self, default: O, f: F) -> O
    where
        F: FnOnce(Value) -> O,
    {
        match self {
            Self::Const(val) => f(val),
            Self::Unknown => default,
        }
    }

    pub fn const_zip_map<F>(self, other: EvalValue, f: F) -> EvalValue
    where
        F: FnOnce(Value, Value) -> Value,
    {
        match (self, other) {
            (Self::Const(lhs), Self::Const(rhs)) => EvalValue::Const(f(lhs, rhs)),
            _ => EvalValue::Unknown,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub span: Option<Span>,
    pub kind: SymbolValue,
    pub refs: Vec<Span>,
    pub anon_ref: bool,
}

impl Symbol {
    pub fn new(name: String, span: Option<Span>, kind: SymbolValue) -> Self {
        Self {
            name,
            span,
            kind,
            refs: vec![],
            anon_ref: false,
        }
    }

    pub fn from_spanned(name: Spanned<String>, kind: SymbolValue) -> Self {
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

    pub fn is_unnecessarily_mut(&self) -> bool {
        matches!(
            self.kind,
            SymbolValue::MutableVariable { mutated: false, .. }
        )
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

    #[display("expected '{expected}', got '{got}'")]
    UnexpectedType {
        expected: ValueKind,
        got: ValueKind,
        span: Span,
    },

    #[display("cannot use operator `{op}` on types '{lhs}' and '{rhs}'")]
    BinOpTypeMismatch {
        op: BinOp,
        lhs: ValueKind,
        rhs: ValueKind,
        span: Span,
    },

    #[display("cannot use relation `{rel}` on types '{lhs}' and '{rhs}'")]
    RelTypeMismatch {
        rel: Rel,
        lhs: ValueKind,
        rhs: ValueKind,
        span: Span,
    },

    #[display("cannot use function as value: `{}`", _0.val)]
    FuncUsedAsValue(Spanned<String>),

    #[display("cannot use break/continue outside a loop")]
    InvalidControlFlow(Span),
}

impl SemanticError {
    pub fn span(&self) -> &Span {
        match self {
            Self::UndefinedVar(name)
            | Self::UndefinedFunc(name)
            | Self::VarNotCallable(name)
            | Self::FuncRedeclared(name)
            | Self::FuncNotAssignable(name)
            | Self::CannotMutateVar(name)
            | Self::FuncUsedAsValue(name)
            | Self::DuplicateParam(name) => &name.span,
            Self::ParamCountMismatch { func_name, .. } => &func_name.span,
            Self::UnexpectedType { span, .. }
            | Self::BinOpTypeMismatch { span, .. }
            | Self::RelTypeMismatch { span, .. }
            | Self::InvalidControlFlow(span) => span,
        }
    }
}

#[derive(Debug, Clone, Display)]
pub enum SemanticWarning {
    #[display("this condition is always {}", _0.val)]
    ConstantCondition(Spanned<bool>),
}

impl SemanticWarning {
    pub fn span(&self) -> &Span {
        match self {
            Self::ConstantCondition(cond) => &cond.span,
        }
    }
}

#[derive(Debug, Clone, Display)]
pub enum SemanticHint {
    #[display("unreachable code")]
    DeadCode(Span),
}

impl SemanticHint {
    pub fn span(&self) -> &Span {
        match self {
            Self::DeadCode(span) => span,
        }
    }

    pub fn tag_unnecessary(&self) -> bool {
        match self {
            Self::DeadCode(..) => true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SemanticModel {
    symbols: Vec<Symbol>,
    refs: HashMap<Span, usize>,
    errors: Vec<SemanticError>,
    warnings: Vec<SemanticWarning>,
    hints: Vec<SemanticHint>,
}

impl From<&Program> for SemanticModel {
    fn from(program: &Program) -> Self {
        let mut model = Self::default();
        model.visit_program(program);
        model
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ControlFlow {
    Return,
    Break(Span),
}

impl SemanticModel {
    pub fn symbols(&self) -> &[Symbol] {
        &self.symbols
    }

    pub fn errors(&self) -> &[SemanticError] {
        &self.errors
    }

    pub fn warnings(&self) -> &[SemanticWarning] {
        &self.warnings
    }

    pub fn hints(&self) -> &[SemanticHint] {
        &self.hints
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

    fn add_anonymous_symbol(&mut self, name: String, kind: SymbolValue, scope: &mut Scope<'_>) {
        let sym_idx = self.add_symbol(Symbol::new(name.clone(), None, kind));
        assert!(scope.insert_local(name, sym_idx));
    }

    fn visit_program(&mut self, program: &Program) {
        let mut std_scope = Scope::default();

        self.add_anonymous_symbol(
            "print".to_string(),
            SymbolValue::Function(ParamCount::Any),
            &mut std_scope,
        );
        self.add_anonymous_symbol(
            "len".to_string(),
            SymbolValue::Function(ParamCount::Fixed(1)),
            &mut std_scope,
        );

        let mut file_scope = Scope::over(&std_scope);

        for func in &program.funcs {
            let new_sym = Symbol::from_spanned(
                func.val.name.clone(),
                SymbolValue::Function(ParamCount::Fixed(func.val.params.len())),
            );
            let sym_idx = self.add_symbol(new_sym);

            if !file_scope.insert_local(func.val.name.val.clone(), sym_idx) {
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
                SymbolValue::variable(param.val.mutable.clone(), EvalValue::Unknown),
            ));

            if !new_scope.insert_local(param.val.name.val.clone(), sym_idx) {
                self.errors
                    .push(SemanticError::DuplicateParam(param.val.name.clone()));
            }
        }

        if let Some(ControlFlow::Break(span)) = self.visit_stmts(&func.val.stmts, &mut new_scope) {
            self.errors.push(SemanticError::InvalidControlFlow(span));
        }
    }

    fn visit_stmts(
        &mut self,
        stmts: &[Spanned<Stmt>],
        scope: &mut Scope<'_>,
    ) -> Option<ControlFlow> {
        let mut iter = stmts.iter();

        while let Some(stmt) = iter.next() {
            if let Some(flow) = self.visit_stmt(stmt, scope) {
                if let Some(next) = iter.next() {
                    let last = iter.last().unwrap_or(next);
                    let dead_span = next.span.combine(&last.span);
                    self.hints.push(SemanticHint::DeadCode(dead_span));
                }

                return Some(flow);
            }
        }

        None
    }

    fn visit_stmt(&mut self, stmt: &Spanned<Stmt>, scope: &mut Scope<'_>) -> Option<ControlFlow> {
        match &stmt.val {
            Stmt::Break | Stmt::Continue => Some(ControlFlow::Break(stmt.span.clone())),
            Stmt::Return(None) => Some(ControlFlow::Return),
            Stmt::VarDecl {
                name,
                init_expr,
                mutable,
            } => {
                let init_val = self.visit_expr(init_expr, scope);
                let new_sym = SymbolValue::variable(mutable.clone(), init_val);
                let sym_idx = self.add_symbol(Symbol::from_spanned(name.clone(), new_sym));
                scope.insert_local_shadowing(name.val.clone(), sym_idx);
                None
            }
            Stmt::Expr(expr) | Stmt::Return(Some(expr)) => {
                self.visit_expr(expr, scope);
                None
            }
            Stmt::Block(stmts) => {
                let mut new_scope = Scope::over(scope);
                self.visit_stmts(stmts, &mut new_scope)
            }
            Stmt::If {
                cond,
                stmt,
                else_stmt,
            } => match self.visit_expect_bool(cond, scope) {
                Some(cond_bool) => {
                    self.warnings
                        .push(SemanticWarning::ConstantCondition(Spanned::new(
                            cond.span.clone(),
                            cond_bool,
                        )));

                    let (live, dead) = if cond_bool {
                        (Some(stmt), else_stmt.as_ref())
                    } else {
                        (else_stmt.as_ref(), Some(stmt))
                    };

                    dead.inspect(|dead| self.hints.push(SemanticHint::DeadCode(dead.span.clone())));
                    live.and_then(|live| self.visit_stmt(live, scope))
                }
                None => {
                    let yes_flow = self.visit_stmt(stmt, &mut Scope::over(scope));
                    let no_flow = else_stmt
                        .as_ref()
                        .and_then(|stmt| self.visit_stmt(stmt, scope));

                    use ControlFlow::*;

                    match (yes_flow, no_flow) {
                        (y, n) if y == n => y,
                        (Some(Break(span)), Some(_)) | (Some(_), Some(Break(span))) => {
                            Some(Break(span))
                        }
                        _ => None,
                    }
                }
            },
            Stmt::While {
                cond,
                stmt,
                cont_expr,
            } => {
                let cond_val = self.visit_expect_bool(cond, scope);

                if let Some(cond_bool) = cond_val {
                    self.warnings
                        .push(SemanticWarning::ConstantCondition(Spanned::new(
                            cond.span.clone(),
                            cond_bool,
                        )));
                }

                match cond_val {
                    Some(false) => {
                        self.hints.push(SemanticHint::DeadCode(stmt.span.clone()));
                        None
                    }
                    cond_val => {
                        let inner_flow = self.visit_stmt(stmt, &mut Scope::over(scope));

                        cont_expr.as_ref().inspect(|expr| {
                            self.visit_expr(expr, scope);
                        });

                        (inner_flow == Some(ControlFlow::Return) && cond_val == Some(true))
                            .then_some(ControlFlow::Return)
                    }
                }
            }
            Stmt::Loop(stmt) => {
                let inner_flow = self.visit_stmt(stmt, &mut Scope::over(scope));
                (inner_flow == Some(ControlFlow::Return)).then_some(ControlFlow::Return)
            }
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

    fn visit_expect_bool(&mut self, expr: &Spanned<Expr>, scope: &mut Scope<'_>) -> Option<bool> {
        match self.visit_expr(expr, scope) {
            EvalValue::Const(Value::Bool(b)) => Some(b),
            EvalValue::Const(other) => {
                self.errors.push(SemanticError::UnexpectedType {
                    expected: ValueKind::Bool,
                    got: other.kind(),
                    span: expr.span.clone(),
                });
                None
            }
            EvalValue::Unknown => None,
        }
    }

    fn visit_expr(&mut self, expr: &Spanned<Expr>, scope: &mut Scope<'_>) -> EvalValue {
        match &expr.val {
            Expr::IntLit(n) => EvalValue::Const(Value::Int(*n as i64)),
            Expr::StrLit(s) => EvalValue::Const(Value::Str(s.clone())),
            Expr::BoolLit(b) => EvalValue::Const(Value::Bool(*b)),
            Expr::NullLit => EvalValue::Const(Value::Null),
            Expr::BinOp(op, lhs, rhs) => {
                let lhs = self.visit_expr(lhs, scope);
                let rhs = self.visit_expr(rhs, scope);

                lhs.const_zip_map(rhs, |l, r| match (op.val, l, r) {
                    (BinOp::Add, Value::Int(l), Value::Int(r)) => Value::Int(l + r),
                    (BinOp::Sub, Value::Int(l), Value::Int(r)) => Value::Int(l - r),
                    (BinOp::Mul, Value::Int(l), Value::Int(r)) => Value::Int(l * r),
                    (BinOp::Div, Value::Int(l), Value::Int(r)) => Value::Int(l / r),
                    (BinOp::Mod, Value::Int(l), Value::Int(r)) => Value::Int(l % r),
                    (BinOp::BoolAnd, Value::Bool(l), Value::Bool(r)) => Value::Bool(l && r),
                    (BinOp::BoolOr, Value::Bool(l), Value::Bool(r)) => Value::Bool(l || r),
                    (op, lhs, rhs) => {
                        self.errors.push(SemanticError::BinOpTypeMismatch {
                            op,
                            lhs: lhs.kind(),
                            rhs: rhs.kind(),
                            span: expr.span.clone(),
                        });
                        Value::Error
                    }
                })
            }
            Expr::Rel(op, lhs, rhs) => {
                let lhs = self.visit_expr(lhs, scope);
                let rhs = self.visit_expr(rhs, scope);

                lhs.const_zip_map(rhs, |l, r| match (op.val, l, r) {
                    (Rel::Eq, l, r) => Value::Bool(l == r),
                    (Rel::Neq, l, r) => Value::Bool(l != r),
                    (Rel::Lt, Value::Int(l), Value::Int(r)) => Value::Bool(l < r),
                    (Rel::Leq, Value::Int(l), Value::Int(r)) => Value::Bool(l <= r),
                    (Rel::Gt, Value::Int(l), Value::Int(r)) => Value::Bool(l > r),
                    (Rel::Geq, Value::Int(l), Value::Int(r)) => Value::Bool(l >= r),
                    (rel, lhs, rhs) => {
                        self.errors.push(SemanticError::RelTypeMismatch {
                            rel,
                            lhs: lhs.kind(),
                            rhs: rhs.kind(),
                            span: expr.span.clone(),
                        });
                        Value::Error
                    }
                })
            }
            Expr::Ternary {
                cond,
                if_yes,
                if_no,
            } => match self.visit_expect_bool(cond, scope) {
                Some(cond_bool) => {
                    self.warnings
                        .push(SemanticWarning::ConstantCondition(Spanned::new(
                            cond.span.clone(),
                            cond_bool,
                        )));

                    let (live, dead) = if cond_bool {
                        (if_yes, if_no)
                    } else {
                        (if_no, if_yes)
                    };

                    self.hints.push(SemanticHint::DeadCode(dead.span.clone()));
                    self.visit_expr(live, scope)
                }
                None => {
                    self.visit_expr(if_yes, scope);
                    self.visit_expr(if_no, scope);
                    EvalValue::Unknown
                }
            },
            Expr::Access { value, idx } => {
                self.visit_expr(value, scope);
                self.visit_expr(idx, scope);
                EvalValue::Unknown
            }
            Expr::UnaryOp(_, expr) => self.visit_expr(expr, scope),
            Expr::ListLit(items) => {
                for item in items {
                    self.visit_expr(item, scope);
                }

                EvalValue::Unknown
            }
            Expr::DictLit(items) => {
                for (key, val) in items {
                    self.visit_expr(key, scope);
                    self.visit_expr(val, scope);
                }

                EvalValue::Unknown
            }
            Expr::Iden(name) => {
                if let Some(sym_idx) = scope.find(&name.val) {
                    self.add_symbol_ref(sym_idx, &name.span);
                    let symbol = &self.symbols[sym_idx];

                    match &symbol.kind {
                        SymbolValue::Function(..) => {
                            self.errors
                                .push(SemanticError::FuncUsedAsValue(name.clone()));

                            EvalValue::Unknown
                        }
                        SymbolValue::MutableVariable { .. } => EvalValue::Unknown,
                        SymbolValue::ImmutableVariable(val) => {
                            val.clone().map_or(EvalValue::Unknown, EvalValue::Const)
                        }
                    }
                } else {
                    self.errors.push(SemanticError::UndefinedVar(name.clone()));
                    EvalValue::Unknown
                }
            }
            Expr::Assign(_, lval, expr) => {
                self.visit_lval(lval, scope);
                self.visit_expr(expr, scope)
            }
            Expr::FuncCall(func_name, args) => {
                if let Some(sym_idx) = scope.find(&func_name.val) {
                    self.add_symbol_ref(sym_idx, &func_name.span);

                    let func_sym = &self.symbols[sym_idx];

                    let SymbolValue::Function(param_cnt) = func_sym.kind else {
                        self.errors
                            .push(SemanticError::VarNotCallable(func_name.clone()));
                        return EvalValue::Unknown;
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

                EvalValue::Unknown
            }
        }
    }

    fn visit_lval(&mut self, lval: &Spanned<LValue>, scope: &mut Scope<'_>) {
        match &lval.val {
            LValue::Access(inner_lval, _) => self.visit_lval(inner_lval, scope),
            LValue::Iden(name) => {
                let Some(sym_idx) = scope.find(&name.val) else {
                    self.errors.push(SemanticError::UndefinedVar(name.clone()));
                    return;
                };

                self.add_symbol_ref(sym_idx, &name.span);
                let sym = &mut self.symbols[sym_idx];

                match &mut sym.kind {
                    SymbolValue::Function(..) => {
                        self.errors
                            .push(SemanticError::FuncNotAssignable(name.clone()));
                    }
                    SymbolValue::ImmutableVariable(..) => {
                        self.errors
                            .push(SemanticError::CannotMutateVar(name.clone()));
                    }
                    SymbolValue::MutableVariable { mutated, .. } => {
                        *mutated = true;
                    }
                }
            }
        }
    }
}
