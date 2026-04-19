pub mod ast;
pub mod error;

pub use error::*;

use crate::{
    parser::ast::{AssignOp, BinOp, Expr, Func, FuncParam, Program, Rel, Stmt, UnaryOp},
    scanner::{Token, TokenKind},
    util::Spanned,
};
use std::iter::Peekable;

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

        while let Some(fn_tok) = self.accept_token(TokenKind::Fn) {
            let name_tok = self.expect(TokenKind::Iden)?;
            let name = name_tok.map(Token::assume_iden);

            self.expect(TokenKind::LParen)?;

            let mut params = vec![];

            if self.peek_is_not(TokenKind::RParen) {
                let param = self.func_param()?;
                params.push(param);

                while self.accept(TokenKind::Comma) {
                    let param = self.func_param()?;
                    params.push(param);
                }
            }

            self.expect(TokenKind::RParen)?;
            self.expect(TokenKind::LBrace)?;

            let stmts = self.stmts()?;
            let rb_tok = self.expect(TokenKind::RBrace)?;

            let func = Func {
                name,
                stmts,
                params,
            };
            funcs.push(Spanned::merge(fn_tok, rb_tok, |_, _| func));
        }

        Ok(Program { funcs })
    }

    fn func_param(&mut self) -> ParseResult<Spanned<FuncParam>> {
        if let Some(mut_tok) = self.accept_token(TokenKind::Mut) {
            let name = self.expect(TokenKind::Iden)?.map(Token::assume_iden);

            Ok(Spanned::merge(mut_tok, name, |mut_tok, name| FuncParam {
                name,
                mutable: Some(mut_tok.span),
            }))
        } else {
            let name = self.expect(TokenKind::Iden)?.map(Token::assume_iden);

            Ok(Spanned::new(
                name.span.clone(),
                FuncParam {
                    name,
                    mutable: None,
                },
            ))
        }
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
        if let Some(lb_tok) = self.accept_token(TokenKind::LBrace) {
            let blk_stmts = self.stmts()?;
            let rb_tok = self.expect(TokenKind::RBrace)?;

            Ok(Spanned::merge(lb_tok, rb_tok, |_, _| {
                Stmt::Block(blk_stmts)
            }))
        } else if let Some(let_tok) = self.accept_token(TokenKind::Let) {
            let mut_tok = self.accept_token(TokenKind::Mut);
            let name = self.expect(TokenKind::Iden)?.map(Token::assume_iden);
            self.expect(TokenKind::Assign)?;
            let expr = self.expr()?;
            let semi_tok = self.expect(TokenKind::Semi)?;

            Ok(Spanned::merge(let_tok, semi_tok, |_, _| Stmt::VarDecl {
                name,
                init_expr: Box::new(expr),
                mutable: mut_tok.map(|tok| tok.span),
            }))
        } else if let Some(if_tok) = self.accept_token(TokenKind::If) {
            self.expect(TokenKind::LParen)?;
            let cond = self.expr()?;
            self.expect(TokenKind::RParen)?;
            let stmt = self.stmt()?;

            if self.accept(TokenKind::Else) {
                let else_stmt = self.stmt()?;

                Ok(Spanned::merge(if_tok, else_stmt, |_, else_stmt| Stmt::If {
                    cond,
                    stmt: Box::new(stmt),
                    else_stmt: Some(Box::new(else_stmt)),
                }))
            } else {
                Ok(Spanned::merge(if_tok, stmt, |_, stmt| Stmt::If {
                    cond,
                    stmt: Box::new(stmt),
                    else_stmt: None,
                }))
            }
        } else if let Some(while_tok) = self.accept_token(TokenKind::While) {
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

            Ok(Spanned::merge(while_tok, inner_stmt, |_, stmt| {
                Stmt::While {
                    cond,
                    stmt: Box::new(stmt),
                    cont_expr,
                }
            }))
        } else if let Some(loop_tok) = self.accept_token(TokenKind::Loop) {
            let inner_stmt = self.stmt()?;
            Ok(Spanned::merge(loop_tok, inner_stmt, |_, stmt| {
                Stmt::Loop(Box::new(stmt))
            }))
        } else if let Some(ret_tok) = self.accept_token(TokenKind::Return) {
            let expr = self
                .peek_is_not(TokenKind::Semi)
                .then(|| self.expr())
                .transpose()?;

            let semi_tok = self.expect(TokenKind::Semi)?;
            Ok(Spanned::merge(ret_tok, semi_tok, |_, _| Stmt::Return(expr)))
        } else if let Some(break_tok) = self.accept_token(TokenKind::Break) {
            let semi_tok = self.expect(TokenKind::Semi)?;
            Ok(Spanned::merge(break_tok, semi_tok, |_, _| Stmt::Break))
        } else if let Some(cont_tok) = self.accept_token(TokenKind::Continue) {
            let semi_tok = self.expect(TokenKind::Semi)?;
            Ok(Spanned::merge(cont_tok, semi_tok, |_, _| Stmt::Continue))
        } else {
            let expr = self.expr()?;
            let semi_tok = self.expect(TokenKind::Semi)?;
            Ok(Spanned::merge(expr, semi_tok, |expr, _| Stmt::Expr(expr)))
        }
    }

    fn expr(&mut self) -> ParseResult<Spanned<Expr>> {
        self.assign_expr()
    }

    fn assign_expr(&mut self) -> ParseResult<Spanned<Expr>> {
        let expr = self.ter_expr()?;

        let op = if let Some(tok) = self.accept_token(TokenKind::Assign) {
            tok.map(|_| AssignOp::Assign)
        } else if let Some(tok) = self.accept_token(TokenKind::AddAssign) {
            tok.map(|_| AssignOp::Add)
        } else if let Some(tok) = self.accept_token(TokenKind::SubAssign) {
            tok.map(|_| AssignOp::Sub)
        } else if let Some(tok) = self.accept_token(TokenKind::MulAssign) {
            tok.map(|_| AssignOp::Mul)
        } else if let Some(tok) = self.accept_token(TokenKind::DivAssign) {
            tok.map(|_| AssignOp::Div)
        } else if let Some(tok) = self.accept_token(TokenKind::ModAssign) {
            tok.map(|_| AssignOp::Mod)
        } else {
            return Ok(expr);
        };

        let lval = expr.try_into().map_err(|e| vec![e])?;
        let src_expr = self.assign_expr()?;

        Ok(Spanned::merge(lval, src_expr, |lval, src_expr| {
            Expr::Assign(op, lval, Box::new(src_expr))
        }))
    }

    fn ter_expr(&mut self) -> ParseResult<Spanned<Expr>> {
        let expr = self.or_expr()?;

        if self.accept(TokenKind::Question) {
            let if_yes = self.expr()?;
            self.expect(TokenKind::Colon)?;
            let if_no = self.ter_expr()?;

            Ok(Spanned::merge(expr, if_no, |expr, if_no| Expr::Ternary {
                cond: Box::new(expr),
                if_yes: Box::new(if_yes),
                if_no: Box::new(if_no),
            }))
        } else {
            Ok(expr)
        }
    }

    fn or_expr(&mut self) -> ParseResult<Spanned<Expr>> {
        let mut expr = self.and_expr()?;

        while let Some(or_tok) = self.accept_token(TokenKind::BoolOr) {
            let op = or_tok.map(|_| BinOp::BoolOr);
            let right = self.and_expr()?;

            expr = Spanned::merge(expr, right, |l, r| {
                Expr::BinOp(op, Box::new(l), Box::new(r))
            });
        }

        Ok(expr)
    }

    fn and_expr(&mut self) -> ParseResult<Spanned<Expr>> {
        let mut expr = self.eq_expr()?;

        while let Some(and_tok) = self.accept_token(TokenKind::BoolAnd) {
            let op = and_tok.map(|_| BinOp::BoolAnd);
            let right = self.eq_expr()?;

            expr = Spanned::merge(expr, right, |l, r| {
                Expr::BinOp(op, Box::new(l), Box::new(r))
            });
        }

        Ok(expr)
    }

    fn eq_expr(&mut self) -> ParseResult<Spanned<Expr>> {
        let mut expr = self.ineq_expr()?;

        loop {
            let rel = if let Some(tok) = self.accept_token(TokenKind::Eq) {
                tok.map(|_| Rel::Eq)
            } else if let Some(tok) = self.accept_token(TokenKind::Neq) {
                tok.map(|_| Rel::Neq)
            } else {
                break;
            };

            let right = self.ineq_expr()?;

            expr = Spanned::merge(expr, right, |l, r| Expr::Rel(rel, Box::new(l), Box::new(r)));
        }

        Ok(expr)
    }

    fn ineq_expr(&mut self) -> ParseResult<Spanned<Expr>> {
        let mut expr = self.add_expr()?;

        loop {
            let rel = if let Some(tok) = self.accept_token(TokenKind::Lt) {
                tok.map(|_| Rel::Lt)
            } else if let Some(tok) = self.accept_token(TokenKind::Leq) {
                tok.map(|_| Rel::Leq)
            } else if let Some(tok) = self.accept_token(TokenKind::Gt) {
                tok.map(|_| Rel::Gt)
            } else if let Some(tok) = self.accept_token(TokenKind::Geq) {
                tok.map(|_| Rel::Geq)
            } else {
                break;
            };

            let right = self.add_expr()?;

            expr = Spanned::merge(expr, right, |l, r| Expr::Rel(rel, Box::new(l), Box::new(r)));
        }

        Ok(expr)
    }

    fn add_expr(&mut self) -> ParseResult<Spanned<Expr>> {
        let mut expr = self.mul_expr()?;

        loop {
            let op = if let Some(tok) = self.accept_token(TokenKind::Add) {
                tok.map(|_| BinOp::Add)
            } else if let Some(tok) = self.accept_token(TokenKind::Sub) {
                tok.map(|_| BinOp::Sub)
            } else {
                break;
            };

            let right = self.mul_expr()?;

            expr = Spanned::merge(expr, right, |l, r| {
                Expr::BinOp(op, Box::new(l), Box::new(r))
            });
        }

        Ok(expr)
    }

    fn mul_expr(&mut self) -> ParseResult<Spanned<Expr>> {
        let mut expr = self.unary_expr()?;

        loop {
            let op = if let Some(tok) = self.accept_token(TokenKind::Mul) {
                tok.map(|_| BinOp::Mul)
            } else if let Some(tok) = self.accept_token(TokenKind::Div) {
                tok.map(|_| BinOp::Div)
            } else if let Some(tok) = self.accept_token(TokenKind::Mod) {
                tok.map(|_| BinOp::Mod)
            } else {
                break;
            };

            let right = self.unary_expr()?;

            expr = Spanned::merge(expr, right, |l, r| {
                Expr::BinOp(op, Box::new(l), Box::new(r))
            });
        }

        Ok(expr)
    }

    fn unary_expr(&mut self) -> ParseResult<Spanned<Expr>> {
        let op_tok = if let Some(tok) = self.accept_token(TokenKind::Add) {
            tok.map(|_| UnaryOp::Plus)
        } else if let Some(tok) = self.accept_token(TokenKind::Sub) {
            tok.map(|_| UnaryOp::Minus)
        } else if let Some(tok) = self.accept_token(TokenKind::BoolNot) {
            tok.map(|_| UnaryOp::BoolNot)
        } else {
            return self.access_expr();
        };

        let inner_expr = self.access_expr()?;

        Ok(Spanned::merge(op_tok, inner_expr, |op, expr| {
            Expr::UnaryOp(op, Box::new(expr))
        }))
    }

    fn access_expr(&mut self) -> ParseResult<Spanned<Expr>> {
        let mut expr = self.base_expr()?;

        while self.accept(TokenKind::LBracket) {
            let idx_expr = self.expr()?;
            let rbracket = self.expect(TokenKind::RBracket)?;

            expr = Spanned::merge(expr, rbracket, |expr, _| Expr::Access {
                value: Box::new(expr),
                idx: Box::new(idx_expr),
            });
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
            Ok(tok.map(Token::assume_num_lit).map(Expr::IntLit))
        } else if let Some(tok) = self.accept_token(TokenKind::BoolLit) {
            Ok(tok.map(Token::assume_bool_lit).map(Expr::BoolLit))
        } else if let Some(tok) = self.accept_token(TokenKind::Null) {
            Ok(tok.map(|_| Expr::NullLit))
        } else if let Some(lb_tok) = self.accept_token(TokenKind::LBrace) {
            let mut items = vec![];

            if self.peek_is_not(TokenKind::RBrace) {
                let entry = self.dict_entry()?;
                items.push(entry);

                while self.accept(TokenKind::Comma) && self.peek_is_not(TokenKind::RBrace) {
                    let entry = self.dict_entry()?;
                    items.push(entry);
                }
            }

            let rb_tok = self.expect(TokenKind::RBrace)?;
            Ok(Spanned::merge(lb_tok, rb_tok, |_, _| Expr::DictLit(items)))
        } else if let Some(lb_tok) = self.accept_token(TokenKind::LBracket) {
            let mut items = vec![];

            if self.peek_is_not(TokenKind::RBracket) {
                let expr = self.expr()?;
                items.push(expr);

                while self.accept(TokenKind::Comma) && self.peek_is_not(TokenKind::RBracket) {
                    let expr = self.expr()?;
                    items.push(expr);
                }
            }

            let rb_tok = self.expect(TokenKind::RBracket)?;
            Ok(Spanned::merge(lb_tok, rb_tok, |_, _| Expr::ListLit(items)))
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

                let rp_tok = self.expect(TokenKind::RParen)?;

                Ok(Spanned::merge(name, rp_tok, |name, _| {
                    Expr::FuncCall(name, args)
                }))
            } else {
                Ok(Spanned::new(name.span.clone(), Expr::Iden(name)))
            }
        } else if let Some(tok) = self.accept_token(TokenKind::StrLit) {
            Ok(tok.map(Token::assume_str_lit).map(Expr::StrLit))
        } else {
            Err(vec![ParseError::ExpectedExpression { found: tok }])
        }
    }
}
