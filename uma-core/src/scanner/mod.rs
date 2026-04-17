mod token;

use std::{iter::Peekable, num::IntErrorKind, str::CharIndices};

use crate::util::Spanned;

pub use token::*;

pub fn is_alphanumeric_ext(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
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
}

impl<'a> Iterator for Scanner<'a> {
    type Item = Spanned<Token>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.accept(|ch| ch.is_whitespace()).is_some() {}

        let init_byte_pos = self.byte_pos;

        let val = match self.next_char()? {
            '#' => {
                while self.next_char().is_some_and(|ch| ch != '\n') {}
                return self.next();
            }
            '+' => {
                if self.accept_char('=') {
                    Token::AddAssign
                } else {
                    Token::Add
                }
            }
            '-' => {
                if self.accept_char('=') {
                    Token::SubAssign
                } else {
                    Token::Sub
                }
            }
            '*' => {
                if self.accept_char('=') {
                    Token::MulAssign
                } else {
                    Token::Mul
                }
            }
            '/' => {
                if self.accept_char('=') {
                    Token::DivAssign
                } else {
                    Token::Div
                }
            }
            '%' => {
                if self.accept_char('=') {
                    Token::ModAssign
                } else {
                    Token::Mod
                }
            }
            '(' => Token::LParen,
            ')' => Token::RParen,
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            '[' => Token::LBracket,
            ']' => Token::RBracket,
            '?' => Token::Question,
            ':' => Token::Colon,
            '|' => {
                if self.accept_char('|') {
                    Token::BoolOr
                } else {
                    Token::BitOr
                }
            }
            '&' => {
                if self.accept_char('&') {
                    Token::BoolAnd
                } else {
                    Token::Ampersand
                }
            }
            '^' => Token::BitXor,
            '<' => {
                if self.accept_char('=') {
                    Token::Leq
                } else {
                    Token::Lt
                }
            }
            '>' => {
                if self.accept_char('=') {
                    Token::Geq
                } else {
                    Token::Gt
                }
            }
            '=' => {
                if self.accept_char('=') {
                    Token::Eq
                } else {
                    Token::Assign
                }
            }
            '!' => {
                if self.accept_char('=') {
                    Token::Neq
                } else {
                    Token::BoolNot
                }
            }

            ',' => Token::Comma,
            ';' => Token::Semi,
            '"' => {
                let mut buf = String::new();

                'outer: loop {
                    match self.next_char() {
                        Some('"') => break 'outer Token::StrLit(buf),
                        Some('\n') => {
                            break 'outer Token::Error(TokenError::UnexpectedNewLine);
                        }
                        Some('\\') => {
                            let esc_ch = match self.next_char() {
                                Some('n') => '\n',
                                Some('r') => '\r',
                                Some('0') => '\0',
                                Some('\\') => '\\',
                                Some('"') => '"',
                                Some(ch) => {
                                    break 'outer Token::Error(TokenError::UnknownEscapeSeq(ch));
                                }
                                None => {
                                    break 'outer Token::Error(TokenError::UnexpectedEof);
                                }
                            };
                            buf.push(esc_ch);
                        }
                        Some(ch) => buf.push(ch),
                        None => {
                            break 'outer Token::Error(TokenError::UnexpectedEof);
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

                buf.parse().map(Token::NumLit).unwrap_or_else(|e| {
                    if e.kind() == &IntErrorKind::PosOverflow {
                        Token::Error(TokenError::IntLitOverflow)
                    } else {
                        Token::Error(TokenError::InvalidIntLit)
                    }
                })
            }
            ch if is_alphanumeric_ext(ch) => {
                let mut iden = String::from(ch);

                while let Some(ch) = self.accept(is_alphanumeric_ext) {
                    iden.push(ch);
                }

                match iden.as_str() {
                    "if" => Token::If,
                    "else" => Token::Else,
                    "while" => Token::While,
                    "loop" => Token::Loop,
                    "for" => Token::For,
                    "return" => Token::Return,
                    "continue" => Token::Continue,
                    "break" => Token::Break,
                    "true" => Token::True,
                    "false" => Token::False,
                    "fn" => Token::Fn,
                    "null" => Token::Null,
                    _ => Token::Iden(iden),
                }
            }
            ch => Token::Error(TokenError::UnexpectedChar(ch)),
        };

        let span = init_byte_pos..self.byte_pos;
        Some(Spanned::new(span, val))
    }
}
