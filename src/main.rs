#![allow(dead_code)]

use std::{iter::Peekable, str::Chars};

use clap::Parser;
use clap_stdin::FileOrStdin;
use derive_more::{Display, Error, IsVariant};
use kinded::Kinded;

#[derive(Kinded, Debug, Clone, PartialEq, Eq, Display)]
enum Token {
    #[display("(")]
    LeftParen,
    #[display(")")]
    RightParen,
    #[display("+")]
    Add,
    #[display("-")]
    Sub,
    #[display("*")]
    Mul,
    #[display("/")]
    Div,
    Num(u32),
}

#[derive(Debug, Clone, Error, Display)]
enum LexError {
    #[display("Unexpected char: '{_0}'")]
    UnexpectedChar(#[error(ignore)] char),
    #[display("Integer overflow")]
    IntegerOverflow,
}

type LexResult = Result<Token, LexError>;

#[derive(Debug, Clone)]
struct Lexer<'a> {
    iter: Peekable<Chars<'a>>,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            iter: src.chars().peekable(),
        }
    }
}

impl<'a> Lexer<'a> {
    fn next_token(&mut self) -> Result<Option<Token>, LexError> {
        while self.iter.next_if(char::is_ascii_whitespace).is_some() {}

        match self.iter.next() {
            Some('+') => Ok(Some(Token::Add)),
            Some('-') => Ok(Some(Token::Sub)),
            Some('*') => Ok(Some(Token::Mul)),
            Some('/') => Ok(Some(Token::Div)),
            Some('(') => Ok(Some(Token::LeftParen)),
            Some(')') => Ok(Some(Token::RightParen)),
            Some(ch @ '0'..='9') => {
                const RADIX: u32 = 10;
                let mut num = ch.to_digit(RADIX).unwrap();

                while let Some(d) = self.iter.peek().and_then(|ch| ch.to_digit(RADIX)) {
                    self.iter.next();
                    num = num
                        .checked_mul(10)
                        .and_then(|n| n.checked_add(d))
                        .ok_or(LexError::IntegerOverflow)?;
                }

                Ok(Some(Token::Num(num)))
            }
            Some(ch) => Err(LexError::UnexpectedChar(ch)),
            None => Ok(None),
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token, LexError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token().transpose()
    }
}

fn format_opt_token(tok: &Option<Token>) -> String {
    tok.as_ref()
        .map(|t| format!("'{t}'"))
        .unwrap_or_else(|| "EOF".to_string())
}

#[derive(Debug, Clone, Error, Display)]
enum ParseError {
    #[display("Unexpected token (found {})", format_opt_token(found))]
    UnexpectedToken { found: Option<Token> },
    #[display(
        "Unexpected token (expected '{expected}', found {})",
        format_opt_token(found)
    )]
    ExpectedToken {
        expected: TokenKind,
        found: Option<Token>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl BinOp {
    pub fn eval(&self, lhs: u32, rhs: u32) -> u32 {
        match self {
            Self::Add => lhs + rhs,
            Self::Sub => lhs - rhs,
            Self::Mul => lhs * rhs,
            Self::Div => lhs / rhs,
        }
    }
}

#[derive(Debug, Clone, IsVariant)]
enum Node {
    BinOp(BinOp, Box<Node>, Box<Node>),
    Num(u32),
}

impl Node {
    fn eval(&self) -> u32 {
        match self {
            Self::BinOp(op, l, r) => op.eval(l.eval(), r.eval()),
            Self::Num(n) => *n,
        }
    }
}

#[derive(Debug, Clone)]
struct LangParser<Iter: Iterator<Item = LexResult>> {
    tokens: Peekable<Iter>,
    idx: usize,
}

impl<I: Iterator<Item = LexResult>> LangParser<I> {
    fn new(tokens: I) -> Self {
        Self {
            tokens: tokens.peekable(),
            idx: 0,
        }
    }

    fn peek(&mut self) -> Option<&LexResult> {
        self.tokens.peek()
    }

    fn consume(&mut self) -> Option<&LexResult> {
        self.tokens.next()
    }

    fn consume_expected(&mut self) -> Result<&LexResult, ParseError> {
        self.consume()
            .ok_or(ParseError::UnexpectedToken { found: None })
    }

    fn accept(&mut self, kind: TokenKind) -> Option<LexResult> {
        if let Some(tok) = self.peek()
            && tok?.kind() == kind
        {
            let tok = tok.clone();
            self.consume();
            return Some(tok);
        }
        None
    }

    fn expect(&mut self, kind: TokenKind) -> Result<(), ParseError> {
        let tok = self.peek();
        if let Some(tok) = tok
            && tok.kind() == kind
        {
            self.consume();
            return Ok(());
        }

        Err(ParseError::ExpectedToken {
            expected: kind,
            found: tok.cloned(),
        })
    }

    fn expr(&mut self) -> Result<Node, ParseError> {
        let mut node = self.term()?;

        loop {
            match self.peek() {
                Some(Token::Add) => {
                    self.consume();
                    let right = self.term()?;
                    node = Node::BinOp(BinOp::Add, Box::new(node), Box::new(right))
                }
                Some(Token::Sub) => {
                    self.consume();
                    let right = self.term()?;
                    node = Node::BinOp(BinOp::Sub, Box::new(node), Box::new(right))
                }
                _ => break,
            }
        }

        Ok(node)
    }

    fn term(&mut self) -> Result<Node, ParseError> {
        let mut node = self.factor()?;

        loop {
            match self.peek() {
                Some(Token::Mul) => {
                    self.consume();
                    let right = self.factor()?;
                    node = Node::BinOp(BinOp::Mul, Box::new(node), Box::new(right))
                }
                Some(Token::Div) => {
                    self.consume();
                    let right = self.factor()?;
                    node = Node::BinOp(BinOp::Div, Box::new(node), Box::new(right))
                }
                _ => break,
            }
        }

        Ok(node)
    }

    fn factor(&mut self) -> Result<Node, ParseError> {
        match self.consume_expected()? {
            Token::Num(n) => Ok(Node::Num(n)),
            Token::LeftParen => {
                let expr = self.expr()?;
                self.expect(TokenKind::RightParen)?;
                Ok(expr)
            }
            tok => Err(ParseError::UnexpectedToken { found: Some(tok) }),
        }
    }
}

#[derive(clap::Parser)]
struct Args {
    /// Input file
    input: FileOrStdin,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let input = args.input.contents()?;

    let mut tokens = vec![];
    let mut lexer = Lexer::new(&input);

    for tok in &mut lexer {
        tokens.push(tok?);
    }

    let mut parser = LangParser::new(lexer.peekable());
    let expr = parser.expr()?;

    println!("expr: {:?}", expr);
    println!("result: {}", expr.eval());
    Ok(())
}
