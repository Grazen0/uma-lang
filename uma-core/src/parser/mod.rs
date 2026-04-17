pub mod ast;
pub mod error;

pub use error::*;

use crate::{
    parser::ast::{BinOp, Expr, Func, ModifyOp, Program, Rel, Stmt, UnaryOp},
    scanner::{Token, TokenKind},
    util::{Combine, Spanned},
};
use std::{iter::Peekable, ops::Range};

fn make_parse_result<T>(val: T, errors: Vec<ParseError>) -> ParseResult<T> {
    if errors.is_empty() {
        Ok(val)
    } else {
        Err(errors)
    }
}
#[derive(Debug)]
pub struct UmaParser<'a, Iter: Iterator<Item = Spanned<Token>>> {
    tokens: Peekable<&'a mut Iter>,
}

impl<'a, I: Iterator<Item = Spanned<Token>>> UmaParser<'a, I> {
    pub fn new(tokens: &'a mut I) -> Self {
        Self {
            tokens: tokens.peekable(),
        }
    }

    fn accept_token(&mut self, kind: TokenKind) -> Option<Spanned<Token>> {
        self.tokens
            .peek()
            .is_some_and(|tok| tok.val.kind() == kind)
            .then(|| self.tokens.next().unwrap())
    }

    fn accept_span(&mut self, kind: TokenKind) -> Option<Range<usize>> {
        self.accept_token(kind).map(|tok| tok.span)
    }

    fn accept(&mut self, kind: TokenKind) -> bool {
        self.accept_token(kind).is_some()
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Spanned<Token>, Vec<ParseError>> {
        let tok = self.tokens.next();

        if let Some(t) = &tok
            && t.val.kind() == kind
        {
            Ok(tok.unwrap())
        } else {
            Err(vec![ParseError::UnexpectedToken {
                found: tok,
                expected: Some(kind),
            }])
        }
    }

    fn peek_is_not(&mut self, kind: TokenKind) -> bool {
        self.tokens.peek().is_none_or(|tok| tok.val.kind() != kind)
    }

    pub fn expect_done(&mut self) -> Result<(), ParseError> {
        if let Some(tok) = self.tokens.next() {
            return Err(ParseError::UnexpectedToken {
                found: Some(tok),
                expected: None,
            });
        }

        Ok(())
    }

    pub fn program_to_end(&mut self) -> ParseResult<Program> {
        let program = self.program()?;
        self.expect_done().map_err(|e| vec![e])?;
        Ok(program)
    }

    pub fn program(&mut self) -> ParseResult<Program> {
        let mut funcs = vec![];
        let mut errors = vec![];

        while let Some(beg_span) = self.accept_span(TokenKind::Fn) {
            let name_token = self.expect(TokenKind::Iden)?;
            let name = name_token.map(Token::assume_iden);

            self.expect(TokenKind::LParen)?;

            let mut args = vec![];

            if let Some(param_token) = self.accept_token(TokenKind::Iden) {
                let arg_name = param_token.map(Token::assume_iden);
                args.push(arg_name);

                while self.accept(TokenKind::Comma) {
                    let param_token = self.expect(TokenKind::Iden)?;
                    let arg_name = param_token.clone().map(Token::assume_iden);

                    if args.iter().find(|a| a.val == arg_name.val).is_some() {
                        errors.push(ParseError::DuplicateParameter { param_token });
                    } else {
                        args.push(arg_name.clone());
                    }
                }
            }

            self.expect(TokenKind::RParen)?;
            self.expect(TokenKind::LBrace)?;
            let stmts = self.stmts()?;
            let end_span = self.expect(TokenKind::RBrace)?.span;

            funcs.push(Spanned::new(
                beg_span.combine(end_span),
                Func { name, stmts, args },
            ));
        }

        make_parse_result(Program { funcs }, errors)
    }

    fn stmts(&mut self) -> ParseResult<Vec<Spanned<Stmt>>> {
        let mut stmts = vec![];

        while self.peek_is_not(TokenKind::RBrace) {
            let stmt = self.stmt()?;
            stmts.push(stmt);
        }

        Ok(stmts)
    }

    fn stmt(&mut self) -> ParseResult<Spanned<Stmt>> {
        let (beg_span, end_span, stmt) = if let Some(beg_span) = self.accept_span(TokenKind::LBrace)
        {
            let blk_stmts = self.stmts()?;
            let end_span = self.expect(TokenKind::RBrace)?.span;
            let stmt = Stmt::Block(blk_stmts);
            (beg_span, end_span, stmt)
        } else if let Some(beg_span) = self.accept_span(TokenKind::If) {
            self.expect(TokenKind::LParen)?;
            let cond = self.expr()?;
            self.expect(TokenKind::RParen)?;
            let stmt = self.stmt()?;

            let else_stmt = self
                .accept(TokenKind::Else)
                .then(|| self.stmt())
                .transpose()?;

            let end_span = else_stmt
                .as_ref()
                .map(|stmt| &stmt.span)
                .cloned()
                .unwrap_or_else(|| stmt.span.clone());

            let stmt = Stmt::If {
                cond,
                stmt: Box::new(stmt),
                else_stmt: else_stmt.map(Box::new),
            };

            (beg_span, end_span, stmt)
        } else if let Some(beg_span) = self.accept_span(TokenKind::While) {
            self.expect(TokenKind::LParen)?;
            let cond = self.expr()?;
            self.expect(TokenKind::RParen)?;

            let cont_expr = self
                .accept(TokenKind::Colon)
                .then(|| -> ParseResult<Spanned<Expr>> {
                    self.expect(TokenKind::LParen)?;
                    let expr = self.expr()?;
                    self.expect(TokenKind::RParen)?;
                    Ok(expr)
                })
                .transpose()?;

            let inner_stmt = self.stmt()?;
            let end_span = inner_stmt.span.clone();

            let stmt = Stmt::While {
                cond,
                stmt: Box::new(inner_stmt),
                cont_expr,
            };

            (beg_span, end_span, stmt)
        } else if let Some(beg_span) = self.accept_span(TokenKind::Loop) {
            let inner_stmt = self.stmt()?;
            let end_span = inner_stmt.span.clone();
            let stmt = Stmt::Loop(Box::new(inner_stmt));
            (beg_span, end_span, stmt)
        } else if let Some(beg_span) = self.accept_span(TokenKind::Return) {
            let expr = self
                .peek_is_not(TokenKind::Semi)
                .then(|| self.expr())
                .transpose()?;

            let end_span = self.expect(TokenKind::Semi)?.span;
            let stmt = Stmt::Return(expr);
            (beg_span, end_span, stmt)
        } else if let Some(beg_span) = self.accept_span(TokenKind::Break) {
            let end_span = self.expect(TokenKind::Semi)?.span;
            (beg_span, end_span, Stmt::Break)
        } else if let Some(beg_span) = self.accept_span(TokenKind::Continue) {
            let end_span = self.expect(TokenKind::Semi)?.span;
            (beg_span, end_span, Stmt::Continue)
        } else {
            let expr = self.expr()?;
            let beg_span = expr.span.clone();
            let end_span = self.expect(TokenKind::Semi)?.span;
            (beg_span, end_span, Stmt::Expr(expr))
        };

        Ok(Spanned::new(beg_span.combine(end_span), stmt))
    }

    fn expr(&mut self) -> ParseResult<Spanned<Expr>> {
        self.assign_expr()
    }

    fn assign_expr(&mut self) -> ParseResult<Spanned<Expr>> {
        let expr = self.ter_expr()?;

        let (beg_span, modify_op) = if let Some(beg_span) = self.accept_span(TokenKind::AddAssign) {
            (beg_span, ModifyOp::Add)
        } else if let Some(beg_span) = self.accept_span(TokenKind::SubAssign) {
            (beg_span, ModifyOp::Sub)
        } else if let Some(beg_span) = self.accept_span(TokenKind::MulAssign) {
            (beg_span, ModifyOp::Mul)
        } else if let Some(beg_span) = self.accept_span(TokenKind::DivAssign) {
            (beg_span, ModifyOp::Div)
        } else if let Some(beg_span) = self.accept_span(TokenKind::ModAssign) {
            (beg_span, ModifyOp::Mod)
        } else if let Some(beg_span) = self.accept_span(TokenKind::Assign) {
            let lval = expr.try_into().map_err(|e| vec![e])?;
            let src_expr = self.assign_expr()?;
            let end_span = src_expr.span.clone();

            return Ok(Spanned::new(
                beg_span.combine(end_span),
                Expr::Assign(lval, Box::new(src_expr)),
            ));
        } else {
            return Ok(expr);
        };

        let lval = expr.try_into().map_err(|e| vec![e])?;
        let src_expr = self.assign_expr()?;
        let end_span = src_expr.span.clone();
        let out_expr = Expr::Modify(modify_op, lval, Box::new(src_expr));

        Ok(Spanned::new(beg_span.combine(end_span), out_expr))
    }

    fn ter_expr(&mut self) -> ParseResult<Spanned<Expr>> {
        let expr = self.or_expr()?;

        if self.accept(TokenKind::Question) {
            let if_yes = self.expr()?;
            self.expect(TokenKind::Colon)?;
            let if_no = self.ter_expr()?;

            let beg_span = expr.span.clone();
            let end_span = if_no.span.clone();
            let ter_expr = Expr::Ternary {
                cond: Box::new(expr),
                if_yes: Box::new(if_yes),
                if_no: Box::new(if_no),
            };

            Ok(Spanned::new(beg_span.combine(end_span), ter_expr))
        } else {
            Ok(expr)
        }
    }

    fn or_expr(&mut self) -> ParseResult<Spanned<Expr>> {
        let mut expr = self.and_expr()?;

        while self.accept(TokenKind::BoolOr) {
            let right = self.and_expr()?;

            let beg_span = expr.span.clone();
            let end_span = right.span.clone();
            expr = Spanned::new(
                beg_span.combine(end_span),
                Expr::BinOp(BinOp::BoolOr, Box::new(expr), Box::new(right)),
            );
        }

        Ok(expr)
    }

    fn and_expr(&mut self) -> ParseResult<Spanned<Expr>> {
        let mut expr = self.eq_expr()?;

        while self.accept(TokenKind::BoolAnd) {
            let right = self.eq_expr()?;

            let beg_span = expr.span.clone();
            let end_span = right.span.clone();
            expr = Spanned::new(
                beg_span.combine(end_span),
                Expr::BinOp(BinOp::BoolAnd, Box::new(expr), Box::new(right)),
            );
        }

        Ok(expr)
    }

    fn eq_expr(&mut self) -> ParseResult<Spanned<Expr>> {
        let mut expr = self.ineq_expr()?;

        loop {
            let rel = if self.accept(TokenKind::Eq) {
                Rel::Eq
            } else if self.accept(TokenKind::Neq) {
                Rel::Neq
            } else {
                break;
            };

            let right = self.ineq_expr()?;

            let beg_span = expr.span.clone();
            let end_span = right.span.clone();
            expr = Spanned::new(
                beg_span.combine(end_span),
                Expr::Rel(rel, Box::new(expr), Box::new(right)),
            );
        }

        Ok(expr)
    }

    fn ineq_expr(&mut self) -> ParseResult<Spanned<Expr>> {
        let mut expr = self.add_expr()?;

        loop {
            let rel = if self.accept(TokenKind::Lt) {
                Rel::Lt
            } else if self.accept(TokenKind::Leq) {
                Rel::Leq
            } else if self.accept(TokenKind::Gt) {
                Rel::Gt
            } else if self.accept(TokenKind::Geq) {
                Rel::Geq
            } else {
                break;
            };

            let right = self.add_expr()?;

            let beg_span = expr.span.clone();
            let end_span = right.span.clone();
            expr = Spanned::new(
                beg_span.combine(end_span),
                Expr::Rel(rel, Box::new(expr), Box::new(right)),
            );
        }

        Ok(expr)
    }

    fn add_expr(&mut self) -> ParseResult<Spanned<Expr>> {
        let mut expr = self.mul_expr()?;

        loop {
            let op = if self.accept(TokenKind::Add) {
                BinOp::Add
            } else if self.accept(TokenKind::Sub) {
                BinOp::Sub
            } else {
                break;
            };

            let right = self.mul_expr()?;

            let beg_span = expr.span.clone();
            let end_span = right.span.clone();
            expr = Spanned::new(
                beg_span.combine(end_span),
                Expr::BinOp(op, Box::new(expr), Box::new(right)),
            );
        }

        Ok(expr)
    }

    fn mul_expr(&mut self) -> ParseResult<Spanned<Expr>> {
        let mut expr = self.unary_expr()?;

        loop {
            let op = if self.accept(TokenKind::Mul) {
                BinOp::Mul
            } else if self.accept(TokenKind::Div) {
                BinOp::Div
            } else if self.accept(TokenKind::Mod) {
                BinOp::Mod
            } else {
                break;
            };

            let right = self.unary_expr()?;

            let beg_span = expr.span.clone();
            let end_span = right.span.clone();
            expr = Spanned::new(
                beg_span.combine(end_span),
                Expr::BinOp(op, Box::new(expr), Box::new(right)),
            );
        }

        Ok(expr)
    }

    fn unary_expr(&mut self) -> ParseResult<Spanned<Expr>> {
        let (beg_span, unary_op) = if let Some(beg_span) = self.accept_span(TokenKind::Add) {
            (beg_span, UnaryOp::Plus)
        } else if let Some(beg_span) = self.accept_span(TokenKind::Sub) {
            (beg_span, UnaryOp::Minus)
        } else if let Some(beg_span) = self.accept_span(TokenKind::BoolNot) {
            (beg_span, UnaryOp::BoolNot)
        } else {
            return self.access_expr();
        };

        let inner_expr = self.access_expr()?;
        let end_span = inner_expr.span.clone();
        let expr = Expr::UnaryOp(unary_op, Box::new(inner_expr));

        Ok(Spanned::new(beg_span.combine(end_span), expr))
    }

    fn access_expr(&mut self) -> ParseResult<Spanned<Expr>> {
        let mut expr = self.base_expr()?;

        while self.accept(TokenKind::LBracket) {
            let idx_expr = self.expr()?;
            let beg_span = expr.span.clone();
            let end_span = self.expect(TokenKind::RBracket)?.span;

            expr = Spanned::new(
                beg_span.combine(end_span),
                Expr::Access {
                    value: Box::new(expr),
                    idx: Box::new(idx_expr),
                },
            );
        }

        Ok(expr)
    }

    fn dict_entry(&mut self) -> ParseResult<(Spanned<Expr>, Spanned<Expr>)> {
        let key_expr = self.expr()?;
        self.expect(TokenKind::Colon)?;
        let val_expr = self.expr()?;
        Ok((key_expr, val_expr))
    }

    fn base_expr(&mut self) -> ParseResult<Spanned<Expr>> {
        let tok = self.tokens.peek().cloned();

        if self.accept(TokenKind::LParen) {
            let expr = self.expr()?;
            self.expect(TokenKind::RParen)?;
            Ok(expr)
        } else if let Some(tok) = self.accept_token(TokenKind::NumLit) {
            Ok(tok.map(Token::assume_num_lit).map(Expr::Int))
        } else if let Some(tok) = self.accept_token(TokenKind::True) {
            Ok(tok.map(|_| Expr::Bool(true)))
        } else if let Some(tok) = self.accept_token(TokenKind::False) {
            Ok(tok.map(|_| Expr::Bool(false)))
        } else if let Some(tok) = self.accept_token(TokenKind::Null) {
            Ok(tok.map(|_| Expr::Null))
        } else if let Some(beg_span) = self.accept_span(TokenKind::LBrace) {
            let mut items = vec![];

            if self.peek_is_not(TokenKind::RBrace) {
                let entry = self.dict_entry()?;
                items.push(entry);

                while self.accept(TokenKind::Comma) && self.peek_is_not(TokenKind::RBrace) {
                    let entry = self.dict_entry()?;
                    items.push(entry);
                }
            }

            let end_span = self.expect(TokenKind::RBrace)?.span;

            Ok(Spanned::new(beg_span.combine(end_span), Expr::Dict(items)))
        } else if let Some(beg_span) = self.accept_span(TokenKind::LBracket) {
            let mut items = vec![];

            if self.peek_is_not(TokenKind::RBracket) {
                let expr = self.expr()?;
                items.push(expr);

                while self.accept(TokenKind::Comma) && self.peek_is_not(TokenKind::RBracket) {
                    let expr = self.expr()?;
                    items.push(expr);
                }
            }

            let end_span = self.expect(TokenKind::RBracket)?.span;
            Ok(Spanned::new(beg_span.combine(end_span), Expr::List(items)))
        } else if let Some(tok) = self.accept_token(TokenKind::Iden) {
            let name = tok.map(Token::assume_iden);

            if self.accept(TokenKind::LParen) {
                let mut args = vec![];

                if self.peek_is_not(TokenKind::RParen) {
                    let expr = self.expr()?;
                    args.push(expr);

                    while self.accept(TokenKind::Comma) && self.peek_is_not(TokenKind::RParen) {
                        let expr = self.expr()?;
                        args.push(expr);
                    }
                }

                let beg_span = name.span.clone();
                let end_span = self.expect(TokenKind::RParen)?.span;

                Ok(Spanned::new(
                    beg_span.combine(end_span),
                    Expr::FuncCall(name, args),
                ))
            } else {
                Ok(Spanned::new(name.span.clone(), Expr::Iden(name)))
            }
        } else if let Some(tok) = self.accept_token(TokenKind::StrLit) {
            Ok(tok.map(Token::assume_str_lit).map(Expr::Str))
        } else {
            Err(vec![ParseError::ExpectedExpression { found: tok }])
        }
    }
}
