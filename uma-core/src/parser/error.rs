use std::ops::Range;

use derive_more::{Display, Error};

use crate::{
    fmt::DisplayWithSrc,
    parser::ast::Expr,
    scanner::{Token, TokenKind},
    util::Spanned,
};

impl<T: DisplayWithSrc> DisplayWithSrc for Option<T> {
    fn fmt_with_src(&self, f: &mut std::fmt::Formatter<'_>, src: &str) -> std::fmt::Result {
        match self {
            Some(tok) => tok.fmt_with_src(f, src),
            None => write!(f, "end-of-file"),
        }
    }
}

impl DisplayWithSrc for Spanned<Token> {
    fn fmt_with_src(&self, f: &mut std::fmt::Formatter<'_>, src: &str) -> std::fmt::Result {
        write!(f, "'{}'", &src[self.span.clone()])
    }
}

#[derive(Debug, Clone, Error, Display)]
pub enum ParseError {
    #[display("unexpected token")]
    UnexpectedToken {
        found: Option<Spanned<Token>>,
        expected: Option<TokenKind>,
    },

    #[display("expected expression")]
    ExpectedExpression { found: Option<Spanned<Token>> },

    #[display("duplicate parameter")]
    DuplicateParameter { param_token: Spanned<Token> },

    #[display("expression is not assignable")]
    ExprNotAssignable(#[error(ignore)] Spanned<Expr>),
}

impl DisplayWithSrc for ParseError {
    fn fmt_with_src(&self, f: &mut std::fmt::Formatter<'_>, src: &str) -> std::fmt::Result {
        match self {
            Self::UnexpectedToken { found, expected } => {
                if let Some(exp) = expected {
                    write!(f, "expected {exp}, found ")?;
                    found.fmt_with_src(f, src)?;
                } else {
                    write!(f, "unexpected ")?;
                    found.fmt_with_src(f, src)?;
                }
                Ok(())
            }
            Self::ExpectedExpression { found } => {
                write!(f, "expected expression, found ")?;
                found.fmt_with_src(f, src)?;
                Ok(())
            }
            Self::DuplicateParameter { param_token } => {
                write!(f, "duplicate parameter ")?;
                param_token.fmt_with_src(f, src)?;
                Ok(())
            }
            _ => todo!(),
            // Self::ExprNotAssignable(..) => "cannot assign to expression".to_string(),
        }
    }
}

impl ParseError {
    pub fn byte_range(&self) -> Option<Range<usize>> {
        match self {
            Self::UnexpectedToken { found, .. } => found.as_ref().map(|t| t.span.clone()),
            Self::ExpectedExpression { found } => found.as_ref().map(|t| t.span.clone()),
            Self::DuplicateParameter { param_token } => Some(param_token.span.clone()),
            Self::ExprNotAssignable(..) => None,
        }
    }
}

pub type ParseResult<T> = Result<T, Vec<ParseError>>;
