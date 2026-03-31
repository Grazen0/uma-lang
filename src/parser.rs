use crate::scanner::{Token, TokenKind, TokenValue};
use derive_more::{Display, Error};
use std::{collections::HashMap, iter::Peekable, ops::Range};

fn fmt_opt_token(tok: Option<&Token>, src: &str) -> String {
    tok.map(|tok| format!("'{}'", &src[tok.byte_range.clone()]))
        .unwrap_or_else(|| "end-of-file".to_string())
}

fn make_parse_result<T>(val: T, errors: Vec<ParseError>) -> ParseResult<T> {
    if errors.is_empty() {
        Ok(val)
    } else {
        Err(errors)
    }
}

fn is_token_type(tok: &Token, sym_table: &SymbolTable<'_>) -> bool {
    match &tok.val {
        TokenValue::Void | TokenValue::Int | TokenValue::Bool => true,
        TokenValue::Iden(iden) => sym_table.is_type(iden),
        _ => false,
    }
}

#[derive(Debug, Clone, Error, Display)]
pub enum ParseError {
    #[display("unexpected token")]
    UnexpectedToken {
        found: Option<Token>,
        expected: Option<TokenKind>,
    },
    #[display("expected expression")]
    ExpectedExpression { found: Option<Token> },
    #[display("expected type")]
    ExpectedType { found: Option<Token> },
    #[display("symbol redeclaration")]
    SymbolRedeclaration(#[error(ignore)] Token),
}

impl ParseError {
    pub fn fmt_with_src(&self, src: &str) -> String {
        match self {
            Self::UnexpectedToken { found, expected } => {
                let found_str = fmt_opt_token(found.as_ref(), src);
                match expected {
                    Some(exp) => format!("expected {exp}, found {found_str}"),
                    None => format!("unexpected {found_str}"),
                }
            }
            Self::ExpectedExpression { found } => {
                let found_str = fmt_opt_token(found.as_ref(), src);
                format!("expected expression, found {found_str}")
            }
            Self::ExpectedType { found } => {
                let found_str = fmt_opt_token(found.as_ref(), src);
                format!("expected type, found {found_str}")
            }
            Self::SymbolRedeclaration(iden) => {
                let found_str = fmt_opt_token(Some(iden), src);
                format!("{found_str} redeclared as different kind of symbol",)
            }
        }
    }
}

impl ParseError {
    pub fn byte_range(&self) -> Option<Range<usize>> {
        match self {
            Self::UnexpectedToken { found, .. } => found.as_ref().map(|t| t.byte_range.clone()),
            Self::ExpectedExpression { found } | Self::ExpectedType { found } => {
                found.as_ref().map(|t| t.byte_range.clone())
            }
            Self::SymbolRedeclaration(tok) => Some(tok.byte_range.clone()),
        }
    }
}

pub type ParseResult<T> = Result<T, Vec<ParseError>>;

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub decls: Vec<TopDecl>,
}

#[derive(Debug, Clone)]
pub enum TopDecl {
    Func(Func),
    Typedef(Typedef),
}

#[derive(Debug, Clone)]
pub struct Func {
    pub ret_type: Type,
    pub name: String,
    pub args: Vec<ArgDecl>,
    pub blk: Block,
}

#[derive(Debug, Clone)]
pub enum Type {
    Void,
    Int,
    Bool,
    Float,
    UserDef(String),
}

#[derive(Debug, Clone)]
pub struct ArgDecl {
    pub r#type: Type,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub struct Typedef {
    pub r#type: Type,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct VarDeclList {
    pub r#type: Type,
    pub decls: Vec<VarDecl>,
}

#[derive(Debug, Clone)]
pub struct VarDecl {
    pub name: String,
    pub init: Option<Expr>,
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
    VarDecl(VarDeclList),
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

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Plus,
    Minus,
    BoolNot,
    Addr,
    Deref,
    Sizeof,
    TypeCast(Type),
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
    Str(String),
    Int(u32),
    Float(f64),
    Bool(bool),
    Nullptr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Type,
    Var,
}

#[derive(Debug, Clone, Default)]
pub struct SymbolTable<'a> {
    map: HashMap<String, SymbolKind>,
    next: Option<&'a SymbolTable<'a>>,
}

impl<'a> SymbolTable<'a> {
    pub fn from_next(next: &'a SymbolTable<'a>) -> Self {
        Self {
            map: HashMap::new(),
            next: Some(next),
        }
    }

    pub fn insert(&mut self, iden: String, kind: SymbolKind) -> bool {
        self.map.insert(iden, kind).is_none()
    }

    pub fn get(&self, iden: &str) -> Option<SymbolKind> {
        self.map
            .get(iden)
            .copied()
            .or_else(|| self.next.and_then(|next| next.get(iden)))
    }

    pub fn is_type(&self, iden: &str) -> bool {
        self.get(iden).is_none_or(|kind| kind == SymbolKind::Type)
    }
}

#[derive(Debug)]
pub struct UmaParser<'a, Iter: Iterator<Item = Token>> {
    tokens: Peekable<&'a mut Iter>,
}

impl<'a, I: Iterator<Item = Token>> UmaParser<'a, I> {
    pub fn new(tokens: &'a mut I) -> Self {
        Self {
            tokens: tokens.peekable(),
        }
    }

    fn accept_pred(&mut self, f: impl FnOnce(&TokenValue) -> bool) -> Option<Token> {
        if let Some(tok) = self.tokens.peek()
            && f(&tok.val)
        {
            let tok = self.tokens.next().unwrap();
            return Some(tok);
        }

        None
    }

    fn accept_kind(&mut self, kind: TokenKind) -> Option<Token> {
        self.accept_pred(|tok| tok.kind() == kind)
    }

    fn accept_kind_discard(&mut self, kind: TokenKind) -> bool {
        self.accept_kind(kind).is_some()
    }

    fn expect_kind(&mut self, kind: TokenKind) -> Result<Token, Vec<ParseError>> {
        let peek = self.tokens.peek();

        if let Some(tok) = peek
            && tok.val.kind() == kind
        {
            let tok = self.tokens.next().unwrap();
            return Ok(tok);
        }

        Err(vec![ParseError::UnexpectedToken {
            expected: Some(kind),
            found: peek.cloned(),
        }])
    }

    pub fn source_file(&mut self) -> ParseResult<SourceFile> {
        let mut errors = vec![];
        let mut decls = vec![];
        let mut sym_table = SymbolTable::default();

        while let Some(tok) = self.tokens.peek() {
            match tok.val.kind() {
                TokenKind::Typedef => match self.typedef(&mut sym_table) {
                    Ok(typedef) => decls.push(TopDecl::Typedef(typedef)),
                    Err(mut e) => errors.append(&mut e),
                },
                _ => match self.func_decl(&mut sym_table) {
                    Ok(func) => decls.push(TopDecl::Func(func)),
                    Err(mut e) => errors.append(&mut e),
                },
            }
        }

        make_parse_result(SourceFile { decls }, errors)
    }

    fn typedef(&mut self, sym_table: &mut SymbolTable<'_>) -> ParseResult<Typedef> {
        self.expect_kind(TokenKind::Typedef)?;

        let r#type = self.r#type(sym_table)?;

        let name_tok = self.expect_kind(TokenKind::Iden)?;
        let TokenValue::Iden(name) = name_tok.val.clone() else {
            unreachable!()
        };

        self.expect_kind(TokenKind::Semi)?;

        if !sym_table.insert(name.clone(), SymbolKind::Type) {
            return Err(vec![ParseError::SymbolRedeclaration(name_tok)]);
        }

        Ok(Typedef { r#type, name })
    }

    fn func_decl(&mut self, sym_table: &mut SymbolTable<'_>) -> ParseResult<Func> {
        let mut errors = vec![];
        let ret_type = self.r#type(sym_table)?;

        let name_tok = self.expect_kind(TokenKind::Iden)?;
        let TokenValue::Iden(name) = name_tok.val.clone() else {
            unreachable!()
        };

        if !sym_table.insert(name.clone(), SymbolKind::Var) {
            errors.push(ParseError::SymbolRedeclaration(name_tok));
        }

        self.expect_kind(TokenKind::LParen)?;

        let mut args = vec![];

        if !self.accept_kind_discard(TokenKind::RParen) {
            args.push(self.param_decl(sym_table)?);

            while self.accept_kind_discard(TokenKind::Comma) {
                args.push(self.param_decl(sym_table)?);
            }

            self.expect_kind(TokenKind::RParen)?;
        }

        match self.block(sym_table) {
            Ok(blk) => {
                let func = Func {
                    ret_type,
                    name,
                    args,
                    blk,
                };

                make_parse_result(func, errors)
            }
            Err(mut e) => {
                errors.append(&mut e);
                Err(errors)
            }
        }
    }

    fn r#type(&mut self, sym_table: &SymbolTable<'_>) -> ParseResult<Type> {
        let tok = self.tokens.peek().cloned();

        if self.accept_kind_discard(TokenKind::Void) {
            Ok(Type::Void)
        } else if self.accept_kind_discard(TokenKind::Int) {
            Ok(Type::Int)
        } else if self.accept_kind_discard(TokenKind::Float) {
            Ok(Type::Float)
        } else if self.accept_kind_discard(TokenKind::Bool) {
            Ok(Type::Bool)
        } else if let Some(tok) = self.accept_kind(TokenKind::Iden) {
            let TokenValue::Iden(name) = &tok.val else {
                unreachable!()
            };

            if sym_table.is_type(name) {
                Ok(Type::UserDef(name.clone()))
            } else {
                Err(vec![ParseError::SymbolRedeclaration(tok)])
            }
        } else {
            Err(vec![ParseError::ExpectedType { found: tok }])
        }
    }

    fn param_decl(&mut self, sym_table: &SymbolTable<'_>) -> ParseResult<ArgDecl> {
        let r#type = self.r#type(sym_table)?;
        let TokenValue::Iden(name) = self.expect_kind(TokenKind::Iden)?.val else {
            unreachable!()
        };

        Ok(ArgDecl { r#type, name })
    }

    fn block(&mut self, sym_table: &SymbolTable<'_>) -> ParseResult<Block> {
        self.expect_kind(TokenKind::LBrace)?;

        let mut errors = vec![];
        let mut stmts = vec![];

        let mut sym_table = SymbolTable::from_next(sym_table);

        while self
            .tokens
            .peek()
            .is_some_and(|t| t.val != TokenValue::RBrace)
        {
            match self.stmt(&mut sym_table) {
                Ok(stmt) => stmts.push(stmt),
                Err(mut e) => {
                    errors.append(&mut e);

                    loop {
                        if self
                            .tokens
                            .peek()
                            .is_some_and(|tok| tok.val.kind() == TokenKind::RBrace)
                        {
                            break;
                        }

                        if let Some(tok) = self.tokens.next()
                            && tok.val.kind() == TokenKind::Semi
                        {
                            break;
                        }
                    }
                }
            }
        }

        self.expect_kind(TokenKind::RBrace)?;
        make_parse_result(Block { stmts }, errors)
    }

    fn stmt(&mut self, sym_table: &mut SymbolTable<'_>) -> ParseResult<Statement> {
        while self.accept_kind_discard(TokenKind::Semi) {}

        if self.accept_kind_discard(TokenKind::If) {
            self.expect_kind(TokenKind::LParen)?;
            let cond = self.expr(sym_table)?;
            self.expect_kind(TokenKind::RParen)?;
            let stmt = self.stmt(sym_table)?;

            let else_stmt = self
                .accept_kind(TokenKind::Else)
                .map(|_| self.stmt(sym_table))
                .transpose()?;

            Ok(Statement::If {
                cond,
                stmt: Box::new(stmt),
                else_stmt: else_stmt.map(Box::new),
            })
        } else if self.accept_kind_discard(TokenKind::While) {
            self.expect_kind(TokenKind::LParen)?;
            let cond = self.expr(sym_table)?;
            self.expect_kind(TokenKind::RParen)?;
            let stmt = self.stmt(sym_table)?;

            Ok(Statement::While {
                cond,
                stmt: Box::new(stmt),
            })
        } else if self.accept_kind_discard(TokenKind::Do) {
            let stmt = self.stmt(sym_table)?;
            self.expect_kind(TokenKind::While)?;
            self.expect_kind(TokenKind::LParen)?;
            let cond = self.expr(sym_table)?;
            self.expect_kind(TokenKind::RParen)?;
            self.expect_kind(TokenKind::Semi)?;

            Ok(Statement::DoWhile {
                cond,
                stmt: Box::new(stmt),
            })
        } else if self.accept_kind_discard(TokenKind::Return) {
            let expr = self.expr(sym_table)?;
            self.expect_kind(TokenKind::Semi)?;
            Ok(Statement::Return(expr))
        } else if self.accept_kind_discard(TokenKind::Continue) {
            self.expect_kind(TokenKind::Semi)?;
            Ok(Statement::Continue)
        } else if self.accept_kind_discard(TokenKind::Break) {
            self.expect_kind(TokenKind::Semi)?;
            Ok(Statement::Break)
        } else if let Some(TokenValue::LBrace) = self.tokens.peek().map(|t| &t.val) {
            self.block(sym_table).map(Statement::Block)
        } else if self
            .tokens
            .peek()
            .is_some_and(|tok| is_token_type(tok, sym_table))
        {
            let var_decl = self.var_decl_list(sym_table)?;
            Ok(Statement::VarDecl(var_decl))
        } else {
            let expr = self.expr(sym_table)?;
            self.expect_kind(TokenKind::Semi)?;
            Ok(Statement::Expr(expr))
        }
    }

    fn var_decl_list(&mut self, sym_table: &mut SymbolTable<'_>) -> ParseResult<VarDeclList> {
        let mut decls = vec![];
        let r#type = self.r#type(sym_table)?;

        let decl = self.var_decl(sym_table)?;
        decls.push(decl);

        while !self.accept_kind_discard(TokenKind::Semi) {
            self.accept_kind_discard(TokenKind::Comma);

            let decl = self.var_decl(sym_table)?;
            decls.push(decl);
        }

        Ok(VarDeclList { r#type, decls })
    }

    fn var_decl(&mut self, sym_table: &mut SymbolTable<'_>) -> ParseResult<VarDecl> {
        let name_tok = self.expect_kind(TokenKind::Iden)?;
        let TokenValue::Iden(name) = name_tok.val.clone() else {
            unreachable!()
        };

        let init = self
            .accept_kind_discard(TokenKind::Assign)
            .then(|| self.var_init(sym_table))
            .transpose()?;

        if !sym_table.insert(name.clone(), SymbolKind::Var) {
            return Err(vec![ParseError::SymbolRedeclaration(name_tok)]);
        }

        Ok(VarDecl { name, init })
    }

    fn var_init(&mut self, sym_table: &SymbolTable<'_>) -> ParseResult<Expr> {
        self.ter_expr(sym_table)
    }

    fn expr(&mut self, sym_table: &SymbolTable<'_>) -> ParseResult<Expr> {
        self.comma_expr(sym_table)
    }

    fn comma_expr(&mut self, sym_table: &SymbolTable<'_>) -> ParseResult<Expr> {
        let mut expr = self.assign_expr(sym_table)?;

        while self.accept_kind_discard(TokenKind::Comma) {
            let right = self.assign_expr(sym_table)?;
            expr = Expr::BinOp(BinOp::Comma, Box::new(expr), Box::new(right));
        }

        Ok(expr)
    }

    fn assign_expr(&mut self, sym_table: &SymbolTable<'_>) -> ParseResult<Expr> {
        let first = self.ter_expr(sym_table)?;

        if self.accept_kind_discard(TokenKind::Assign) {
            let right = self.assign_expr(sym_table)?;
            Ok(Expr::Assign {
                dst: Box::new(first),
                src: Box::new(right),
            })
        } else {
            Ok(first)
        }
    }

    fn ter_expr(&mut self, sym_table: &SymbolTable<'_>) -> ParseResult<Expr> {
        let expr = self.or_expr(sym_table)?;

        if self.accept_kind_discard(TokenKind::Question) {
            let if_yes = self.expr(sym_table)?;
            self.expect_kind(TokenKind::Colon)?;
            let if_no = self.ter_expr(sym_table)?;

            Ok(Expr::Ternary {
                cond: Box::new(expr),
                if_yes: Box::new(if_yes),
                if_no: Box::new(if_no),
            })
        } else {
            Ok(expr)
        }
    }

    fn or_expr(&mut self, sym_table: &SymbolTable<'_>) -> ParseResult<Expr> {
        let mut expr = self.and_expr(sym_table)?;

        while self.accept_kind_discard(TokenKind::BoolOr) {
            let right = self.and_expr(sym_table)?;
            expr = Expr::BinOp(BinOp::BoolOr, Box::new(expr), Box::new(right));
        }

        Ok(expr)
    }

    fn and_expr(&mut self, sym_table: &SymbolTable<'_>) -> ParseResult<Expr> {
        let mut expr = self.eq_expr(sym_table)?;

        while self.accept_kind_discard(TokenKind::BoolAnd) {
            let right = self.eq_expr(sym_table)?;
            expr = Expr::BinOp(BinOp::BoolAnd, Box::new(expr), Box::new(right));
        }

        Ok(expr)
    }

    fn eq_expr(&mut self, sym_table: &SymbolTable<'_>) -> ParseResult<Expr> {
        let mut expr = self.ineq_expr(sym_table)?;

        loop {
            let rel = if self.accept_kind_discard(TokenKind::Eq) {
                Rel::Eq
            } else if self.accept_kind_discard(TokenKind::Neq) {
                Rel::Neq
            } else {
                break;
            };

            let right = self.ineq_expr(sym_table)?;
            expr = Expr::Rel(rel, Box::new(expr), Box::new(right));
        }

        Ok(expr)
    }

    fn ineq_expr(&mut self, sym_table: &SymbolTable<'_>) -> ParseResult<Expr> {
        let mut expr = self.add_expr(sym_table)?;

        loop {
            let rel = if self.accept_kind_discard(TokenKind::Lt) {
                Rel::Lt
            } else if self.accept_kind_discard(TokenKind::Leq) {
                Rel::Leq
            } else if self.accept_kind_discard(TokenKind::Gt) {
                Rel::Gt
            } else if self.accept_kind_discard(TokenKind::Geq) {
                Rel::Geq
            } else {
                break;
            };

            let right = self.add_expr(sym_table)?;
            expr = Expr::Rel(rel, Box::new(expr), Box::new(right));
        }

        Ok(expr)
    }

    fn add_expr(&mut self, sym_table: &SymbolTable<'_>) -> ParseResult<Expr> {
        let mut expr = self.mul_expr(sym_table)?;

        loop {
            let op = if self.accept_kind_discard(TokenKind::Add) {
                BinOp::Add
            } else if self.accept_kind_discard(TokenKind::Sub) {
                BinOp::Sub
            } else {
                break;
            };

            let right = self.mul_expr(sym_table)?;
            expr = Expr::BinOp(op, Box::new(expr), Box::new(right));
        }

        Ok(expr)
    }

    fn mul_expr(&mut self, sym_table: &SymbolTable<'_>) -> ParseResult<Expr> {
        let mut expr = self.unary_expr(sym_table)?;

        loop {
            let op = if self.accept_kind_discard(TokenKind::Asterisk) {
                BinOp::Mul
            } else if self.accept_kind_discard(TokenKind::Div) {
                BinOp::Div
            } else if self.accept_kind_discard(TokenKind::Mod) {
                BinOp::Mod
            } else {
                break;
            };

            let right = self.unary_expr(sym_table)?;
            expr = Expr::BinOp(op, Box::new(expr), Box::new(right));
        }

        Ok(expr)
    }

    fn unary_expr(&mut self, sym_table: &SymbolTable<'_>) -> ParseResult<Expr> {
        if self.accept_kind_discard(TokenKind::Add) {
            let expr = self.unary_expr(sym_table)?;
            Ok(Expr::UnaryOp(UnaryOp::Plus, Box::new(expr)))
        } else if self.accept_kind_discard(TokenKind::Sub) {
            let expr = self.unary_expr(sym_table)?;
            Ok(Expr::UnaryOp(UnaryOp::Minus, Box::new(expr)))
        } else if self.accept_kind_discard(TokenKind::BoolNot) {
            let expr = self.unary_expr(sym_table)?;
            Ok(Expr::UnaryOp(UnaryOp::BoolNot, Box::new(expr)))
        } else if self.accept_kind_discard(TokenKind::Ampersand) {
            let expr = self.unary_expr(sym_table)?;
            Ok(Expr::UnaryOp(UnaryOp::Addr, Box::new(expr)))
        } else if self.accept_kind_discard(TokenKind::Asterisk) {
            let expr = self.unary_expr(sym_table)?;
            Ok(Expr::UnaryOp(UnaryOp::Deref, Box::new(expr)))
        } else if self.accept_kind_discard(TokenKind::Sizeof) {
            let expr = self.unary_expr(sym_table)?;
            Ok(Expr::UnaryOp(UnaryOp::Sizeof, Box::new(expr)))
        } else if self.accept_kind_discard(TokenKind::LParen) {
            if self
                .tokens
                .peek()
                .is_some_and(|tok| is_token_type(tok, sym_table))
            {
                let r#type = self.r#type(sym_table)?;
                self.expect_kind(TokenKind::RParen)?;
                let expr = self.unary_expr(sym_table)?;

                Ok(Expr::UnaryOp(UnaryOp::TypeCast(r#type), Box::new(expr)))
            } else {
                let expr = self.expr(sym_table)?;
                self.expect_kind(TokenKind::RParen)?;
                Ok(expr)
            }
        } else {
            self.access_expr(sym_table)
        }
    }

    fn access_expr(&mut self, sym_table: &SymbolTable<'_>) -> ParseResult<Expr> {
        let base = self.base_expr(sym_table)?;

        if self.accept_kind_discard(TokenKind::LParen) {
            let mut args = vec![];

            if !self.accept_kind_discard(TokenKind::RParen) {
                args.push(self.expr(sym_table)?);

                while self.accept_kind_discard(TokenKind::Comma) {
                    args.push(self.expr(sym_table)?);
                }
            }

            self.expect_kind(TokenKind::RParen)?;
            Ok(Expr::FuncCall(Box::new(base), args))
        } else if self.accept_kind_discard(TokenKind::LBrace) {
            let idx = self.expr(sym_table)?;
            Ok(Expr::ArrayAccess(Box::new(base), Box::new(idx)))
        } else {
            Ok(base)
        }
    }

    fn base_expr(&mut self, sym_table: &SymbolTable<'_>) -> ParseResult<Expr> {
        let tok = self.tokens.peek().cloned();

        if self.accept_kind_discard(TokenKind::LParen) {
            let expr = self.expr(sym_table)?;
            self.expect_kind(TokenKind::RParen)?;
            Ok(expr)
        } else if let Some(TokenValue::NumLit(n)) =
            self.accept_kind(TokenKind::NumLit).map(|t| t.val)
        {
            Ok(Expr::Int(n))
        } else if let Some(TokenValue::FloatLit(f)) =
            self.accept_kind(TokenKind::FloatLit).map(|t| t.val)
        {
            Ok(Expr::Float(f))
        } else if let Some(TokenValue::Iden(name)) =
            self.accept_kind(TokenKind::Iden).map(|t| t.val)
        {
            Ok(Expr::Iden(name))
        } else if let Some(TokenValue::StrLit(str)) =
            self.accept_kind(TokenKind::StrLit).map(|t| t.val)
        {
            Ok(Expr::Str(str))
        } else if self.accept_kind_discard(TokenKind::True) {
            Ok(Expr::Bool(true))
        } else if self.accept_kind_discard(TokenKind::False) {
            Ok(Expr::Bool(false))
        } else if self.accept_kind_discard(TokenKind::Nullptr) {
            Ok(Expr::Nullptr)
        } else {
            Err(vec![ParseError::ExpectedExpression { found: tok }])
        }
    }
}
