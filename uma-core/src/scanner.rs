use std::{iter::Peekable, num::IntErrorKind, ops::Range, str::CharIndices};

use derive_more::Display;
use kinded::Kinded;

pub fn is_alphanumeric_extended(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

#[derive(Debug, Clone)]
pub struct Token {
    pub byte_range: Range<usize>,
    pub val: TokenValue,
}

#[derive(Kinded, Debug, Clone, PartialEq)]
#[kinded(kind = TokenKind)]
pub enum TokenValue {
    #[kinded(rename = "'('")]
    LParen,
    #[kinded(rename = "')'")]
    RParen,
    #[kinded(rename = "'{{'")]
    LBrace,
    #[kinded(rename = "'}}'")]
    RBrace,
    #[kinded(rename = "'['")]
    LBracket,
    #[kinded(rename = "']'")]
    RBracket,
    #[kinded(rename = "'+'")]
    Add,
    #[kinded(rename = "'+='")]
    AddAssign,
    #[kinded(rename = "'-'")]
    Sub,
    #[kinded(rename = "'-='")]
    SubAssign,
    #[kinded(rename = "'*'")]
    Mul,
    #[kinded(rename = "'*='")]
    MulAssign,
    #[kinded(rename = "'/'")]
    Div,
    #[kinded(rename = "'/='")]
    DivAssign,
    #[kinded(rename = "'%'")]
    Mod,
    #[kinded(rename = "'%='")]
    ModAssign,
    #[kinded(rename = "'||'")]
    BoolOr,
    #[kinded(rename = "'|'")]
    BitOr,
    #[kinded(rename = "'^'")]
    BitXor,
    #[kinded(rename = "'&&'")]
    BoolAnd,
    #[kinded(rename = "'&'")]
    Ampersand,
    #[kinded(rename = "'!'")]
    BoolNot,
    #[kinded(rename = "'=='")]
    Eq,
    #[kinded(rename = "'='")]
    Assign,
    #[kinded(rename = "'?'")]
    Question,
    #[kinded(rename = "':'")]
    Colon,
    #[kinded(rename = "'!='")]
    Neq,
    #[kinded(rename = "'<'")]
    Lt,
    #[kinded(rename = "'<='")]
    Leq,
    #[kinded(rename = "'>'")]
    Gt,
    #[kinded(rename = "'>='")]
    Geq,
    #[kinded(rename = "','")]
    Comma,
    #[kinded(rename = "';'")]
    Semi,
    #[kinded(rename = "'if'")]
    If,
    #[kinded(rename = "'else'")]
    Else,
    #[kinded(rename = "'while'")]
    While,
    #[kinded(rename = "'loop'")]
    Loop,
    #[kinded(rename = "'for'")]
    For,
    #[kinded(rename = "'return'")]
    Return,
    #[kinded(rename = "'continue'")]
    Continue,
    #[kinded(rename = "'break'")]
    Break,
    #[kinded(rename = "'true'")]
    True,
    #[kinded(rename = "'false'")]
    False,
    #[kinded(rename = "numeric literal")]
    NumLit(u32),
    #[kinded(rename = "string literal")]
    StrLit(String),
    #[kinded(rename = "identifier")]
    Iden(String),
    #[kinded(rename = "error")]
    Error(ScanErrorValue),
    #[kinded(rename = "'print'")]
    Print,
    #[kinded(rename = "'fn'")]
    Fn,
    #[kinded(rename = "'null'")]
    Null,
}

impl TokenValue {
    pub fn into_num(self) -> u32 {
        match self {
            Self::NumLit(n) => n,
            _ => unreachable!(),
        }
    }

    pub fn into_str(self) -> String {
        match self {
            Self::StrLit(s) => s,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Display, PartialEq, Eq)]
pub enum ScanErrorValue {
    #[display("unexpected character '{_0}'")]
    UnexpectedChar(char),
    #[display("unknown escape sequence '\\{_0}'")]
    UnknownEscapeSeq(char),
    #[display("unexpected newline")]
    UnexpectedNewLine,
    #[display("unexpected end-of-file")]
    UnexpectedEof,
    #[display("integer literal is too large")]
    IntLitOverflow,
    #[display("invalid integer literal")]
    InvalidIntLit,
    #[display("invalid float literal")]
    InvalidFloatLit,
}

#[derive(Debug, Clone)]
pub struct Scanner<'a> {
    chars: Peekable<CharIndices<'a>>,
    byte_pos: usize,
}

impl<'a> Scanner<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            chars: src.char_indices().peekable(),
            byte_pos: 0,
        }
    }

    fn next_char(&mut self) -> Option<char> {
        let (_, ch) = self.chars.next()?;

        if let Some((next_byte_pos, _)) = self.chars.peek() {
            self.byte_pos = *next_byte_pos;
        }

        Some(ch)
    }

    fn accept(&mut self, f: impl FnOnce(char) -> bool) -> Option<char> {
        if self.chars.peek().is_some_and(|&(_, ch)| f(ch)) {
            Some(self.next_char().unwrap())
        } else {
            None
        }
    }

    fn accept_char(&mut self, ch: char) -> bool {
        self.accept(|c| c == ch).is_some()
    }

    fn next_token(&mut self) -> Option<Token> {
        while self.accept(|ch| ch.is_whitespace()).is_some() {}

        let init_byte_pos = self.byte_pos;

        let val = match self.next_char()? {
            '#' => {
                while self.next_char().is_some_and(|ch| ch != '\n') {}
                return self.next_token();
            }
            '+' => {
                if self.accept_char('=') {
                    TokenValue::AddAssign
                } else {
                    TokenValue::Add
                }
            }
            '-' => {
                if self.accept_char('=') {
                    TokenValue::SubAssign
                } else {
                    TokenValue::Sub
                }
            }
            '*' => {
                if self.accept_char('=') {
                    TokenValue::MulAssign
                } else {
                    TokenValue::Mul
                }
            }
            '/' => {
                if self.accept_char('=') {
                    TokenValue::DivAssign
                } else {
                    TokenValue::Div
                }
            }
            '%' => {
                if self.accept_char('=') {
                    TokenValue::ModAssign
                } else {
                    TokenValue::Mod
                }
            }
            '(' => TokenValue::LParen,
            ')' => TokenValue::RParen,
            '{' => TokenValue::LBrace,
            '}' => TokenValue::RBrace,
            '[' => TokenValue::LBracket,
            ']' => TokenValue::RBracket,
            '?' => TokenValue::Question,
            ':' => TokenValue::Colon,
            '|' => {
                if self.accept_char('|') {
                    TokenValue::BoolOr
                } else {
                    TokenValue::BitOr
                }
            }
            '&' => {
                if self.accept_char('&') {
                    TokenValue::BoolAnd
                } else {
                    TokenValue::Ampersand
                }
            }
            '^' => TokenValue::BitXor,
            '<' => {
                if self.accept_char('=') {
                    TokenValue::Leq
                } else {
                    TokenValue::Lt
                }
            }
            '>' => {
                if self.accept_char('=') {
                    TokenValue::Geq
                } else {
                    TokenValue::Gt
                }
            }
            '=' => {
                if self.accept_char('=') {
                    TokenValue::Eq
                } else {
                    TokenValue::Assign
                }
            }
            '!' => {
                if self.accept_char('=') {
                    TokenValue::Neq
                } else {
                    TokenValue::BoolNot
                }
            }

            ',' => TokenValue::Comma,
            ';' => TokenValue::Semi,
            '"' => {
                let mut buf = String::new();

                'outer: loop {
                    match self.next_char() {
                        Some('"') => break 'outer TokenValue::StrLit(buf),
                        Some('\n') => {
                            break 'outer TokenValue::Error(ScanErrorValue::UnexpectedNewLine);
                        }
                        Some('\\') => {
                            let esc_ch = match self.next_char() {
                                Some('n') => '\n',
                                Some('r') => '\r',
                                Some('0') => '\0',
                                Some('\\') => '\\',
                                Some('"') => '"',
                                Some(ch) => {
                                    break 'outer TokenValue::Error(
                                        ScanErrorValue::UnknownEscapeSeq(ch),
                                    );
                                }
                                None => {
                                    break 'outer TokenValue::Error(ScanErrorValue::UnexpectedEof);
                                }
                            };
                            buf.push(esc_ch);
                        }
                        Some(ch) => buf.push(ch),
                        None => {
                            break 'outer TokenValue::Error(ScanErrorValue::UnexpectedEof);
                        }
                    }
                }
            }
            init_ch @ ('0'..='9') => {
                let radix = if init_ch == '0' {
                    if self.accept(|ch| ch == 'b' || ch == 'B').is_some() {
                        2
                    } else if self.accept(|ch| ch == 'x' || ch == 'X').is_some() {
                        16
                    } else {
                        8
                    }
                } else {
                    10
                };

                let mut buf = String::from(init_ch);

                while let Some(ch) = self.accept(|ch| (ch).is_digit(radix)) {
                    buf.push(ch);
                }

                buf.parse().map(TokenValue::NumLit).unwrap_or_else(|e| {
                    if e.kind() == &IntErrorKind::PosOverflow {
                        TokenValue::Error(ScanErrorValue::IntLitOverflow)
                    } else {
                        TokenValue::Error(ScanErrorValue::InvalidIntLit)
                    }
                })
            }
            ch if is_alphanumeric_extended(ch) => {
                let mut iden = String::from(ch);

                while let Some(ch) = self.accept(is_alphanumeric_extended) {
                    iden.push(ch);
                }

                match iden.as_str() {
                    "if" => TokenValue::If,
                    "else" => TokenValue::Else,
                    "while" => TokenValue::While,
                    "loop" => TokenValue::Loop,
                    "for" => TokenValue::For,
                    "return" => TokenValue::Return,
                    "continue" => TokenValue::Continue,
                    "break" => TokenValue::Break,
                    "true" => TokenValue::True,
                    "false" => TokenValue::False,
                    "print" => TokenValue::Print,
                    "fn" => TokenValue::Fn,
                    "null" => TokenValue::Null,
                    _ => TokenValue::Iden(iden),
                }
            }
            ch => TokenValue::Error(ScanErrorValue::UnexpectedChar(ch)),
        };

        let byte_range = init_byte_pos..self.byte_pos;
        Some(Token { byte_range, val })
    }
}

impl<'a> Iterator for Scanner<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}
