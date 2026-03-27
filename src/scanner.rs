use std::{iter::Peekable, str::Chars};

use derive_more::{Display, Error};
use kinded::Kinded;

#[derive(Debug, Clone)]
pub struct Token {
    pub val: TokenValue,
    pub line: usize,
    pub col: usize,
    pub pos: usize,
}

#[derive(Kinded, Debug, Clone, PartialEq, Display)]
#[display(rename_all = "lowercase")]
pub enum TokenValue {
    #[display("(")]
    #[kinded(rename = "'('")]
    LParen,
    #[display(")")]
    #[kinded(rename = "')'")]
    RParen,
    #[display("{{")]
    #[kinded(rename = "'{{'")]
    LBrace,
    #[display("}}")]
    #[kinded(rename = "'}}'")]
    RBrace,
    #[display("[")]
    #[kinded(rename = "'['")]
    LBracket,
    #[display("]")]
    #[kinded(rename = "']'")]
    RBracket,
    #[display("+")]
    #[kinded(rename = "'+'")]
    Add,
    #[display("-")]
    #[kinded(rename = "'-'")]
    Sub,
    #[display("*")]
    #[kinded(rename = "'*'")]
    Asterisk,
    #[display("/")]
    #[kinded(rename = "'/'")]
    Div,
    #[display("%")]
    #[kinded(rename = "'%'")]
    Mod,
    #[display("||")]
    #[kinded(rename = "'||'")]
    BoolOr,
    #[display("&&")]
    #[kinded(rename = "'&&'")]
    BoolAnd,
    #[display("!")]
    #[kinded(rename = "'!'")]
    BoolNot,
    #[display("&")]
    #[kinded(rename = "'&'")]
    Ampersand,
    #[display("==")]
    #[kinded(rename = "'=='")]
    Eq,
    #[display("=")]
    #[kinded(rename = "'='")]
    Assign,
    #[display("?")]
    #[kinded(rename = "'?'")]
    Question,
    #[display(":")]
    #[kinded(rename = "':'")]
    Colon,
    #[display("!=")]
    #[kinded(rename = "'!='")]
    Neq,
    #[display("<")]
    #[kinded(rename = "'<'")]
    Lt,
    #[display("<=")]
    #[kinded(rename = "'<='")]
    Leq,
    #[display(">")]
    #[kinded(rename = "'>'")]
    Gt,
    #[display(">=")]
    #[kinded(rename = "'>='")]
    Geq,
    #[display(",")]
    #[kinded(rename = "','")]
    Comma,
    #[display(";")]
    #[kinded(rename = "';'")]
    Semi,
    If,
    Else,
    While,
    Do,
    For,
    Return,
    Continue,
    Break,
    Void,
    Int,
    Float,
    Bool,
    True,
    False,
    Nullptr,
    Sizeof,
    Struct,
    Enum,
    Typedef,
    #[kinded(rename = "numeric literal")]
    NumLit(u32),
    FloatLit(f64),
    #[kinded(rename = "identifier")]
    Iden(String),
    #[display("\"{_0}\"")]
    #[kinded(rename = "string literal")]
    StrLit(String),
}

#[derive(Debug, Clone, Error, Display)]
pub enum ScanError {
    #[display("Unexpected char: '{_0}'")]
    UnexpectedChar(#[error(ignore)] char),
    #[display("Unexpected EOF")]
    UnexpectedEof,
    #[display("Integer overflow")]
    IntegerOverflow,
}

#[derive(Debug, Clone)]
pub struct Scanner<'a> {
    chars: Peekable<Chars<'a>>,
    pos: usize,
    line: usize,
    col: usize,
}

impl<'a> Scanner<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            chars: src.chars().peekable(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn next_char(&mut self) -> Option<char> {
        let ch = self.chars.next();
        if ch.is_some() {
            self.pos += 1;
        }
        ch
    }

    fn unexpected_char(&self, ch: char) -> ScanError {
        ScanError::UnexpectedChar(ch)
    }

    fn next_token(&mut self) -> Result<Option<Token>, ScanError> {
        while self.chars.peek().is_some_and(char::is_ascii_whitespace) {
            self.next_char();
        }

        let initial_pos = self.pos;
        let initial_line = self.line;
        let initial_col = self.col;

        let val: Option<TokenValue> = match self.next_char() {
            Some('+') => Some(TokenValue::Add),
            Some('-') => Some(TokenValue::Sub),
            Some('*') => Some(TokenValue::Asterisk),
            Some('/') => match self.chars.peek() {
                Some('/') => {
                    while self.next_char().is_some_and(|ch| ch != '\n') {}
                    return self.next_token();
                }
                Some('*') => {
                    self.next_char();

                    loop {
                        let next = self.next_char().ok_or(ScanError::UnexpectedEof)?;
                        if next == '*' && self.next_char().is_some_and(|ch| ch == '/') {
                            break;
                        }
                    }

                    return self.next_token();
                }
                _ => Some(TokenValue::Div),
            },
            Some('%') => Some(TokenValue::Mod),
            Some('(') => Some(TokenValue::LParen),
            Some(')') => Some(TokenValue::RParen),
            Some('{') => Some(TokenValue::LBrace),
            Some('}') => Some(TokenValue::RBrace),
            Some('[') => Some(TokenValue::LBracket),
            Some(']') => Some(TokenValue::RBracket),
            Some('?') => Some(TokenValue::Question),
            Some(':') => Some(TokenValue::Colon),
            Some('<') => match self.chars.peek() {
                Some('=') => {
                    self.next_char();
                    Some(TokenValue::Leq)
                }
                _ => Some(TokenValue::Lt),
            },
            Some('|') => match self.chars.peek() {
                Some('|') => {
                    self.next_char();
                    Some(TokenValue::BoolOr)
                }
                Some(&ch) => return Err(self.unexpected_char(ch)),
                None => return Err(ScanError::UnexpectedEof),
            },
            Some('&') => match self.chars.peek() {
                Some('&') => {
                    self.next_char();
                    Some(TokenValue::BoolAnd)
                }
                _ => Some(TokenValue::Ampersand),
            },
            Some('>') => match self.chars.peek() {
                Some('=') => {
                    self.next_char();
                    Some(TokenValue::Geq)
                }
                _ => Some(TokenValue::Gt),
            },
            Some('=') => match self.chars.peek() {
                Some('=') => {
                    self.next_char();
                    Some(TokenValue::Eq)
                }
                _ => Some(TokenValue::Assign),
            },
            Some('!') => match self.chars.peek() {
                Some('=') => {
                    self.next_char();
                    Some(TokenValue::Neq)
                }
                _ => Some(TokenValue::BoolNot),
            },
            Some(',') => Some(TokenValue::Comma),
            Some(';') => Some(TokenValue::Semi),
            Some('"') => {
                let mut str = String::new();

                loop {
                    let mut ch = self.next_char().ok_or(ScanError::UnexpectedEof)?;

                    if ch == '"' {
                        break;
                    }

                    if ch == '\\' {
                        ch = match self.next_char().ok_or(ScanError::UnexpectedEof)? {
                            'n' => '\n',
                            'r' => '\r',
                            '0' => '\0',
                            '\\' => '\\',
                            '"' => '"',
                            ch => return Err(ScanError::UnexpectedChar(ch)),
                        }
                    }

                    str.push(ch);
                }

                Some(TokenValue::StrLit(str))
            }
            Some(ch @ '0'..='9') => {
                let radix = if ch == '0' {
                    match self.chars.peek() {
                        Some('b' | 'B') => {
                            self.chars.next();
                            2
                        }
                        Some('x' | 'X') => {
                            self.chars.next();
                            16
                        }
                        _ => 8,
                    }
                } else {
                    10
                };

                let mut num = ch.to_digit(radix).unwrap();

                while let Some(d) = self.chars.peek().and_then(|ch| ch.to_digit(radix)) {
                    self.next_char();
                    num = num
                        .checked_mul(radix)
                        .and_then(|n| n.checked_add(d))
                        .ok_or(ScanError::IntegerOverflow)?;
                }

                match self.chars.peek() {
                    Some('.') => {
                        self.chars.next();

                        let mut factor = 0.1;
                        let mut num_f = num as f64;

                        while let Some(d) = self.chars.peek().and_then(|ch| ch.to_digit(radix)) {
                            self.next_char();
                            num_f += (d as f64) * factor;
                            factor /= 10.0;
                        }

                        Some(TokenValue::FloatLit(num_f))
                    }
                    _ => Some(TokenValue::NumLit(num)),
                }
            }
            Some(ch) if ch.is_ascii_alphanumeric() => {
                let mut iden = String::from(ch);

                while let Some(peek) = self.chars.peek()
                    && peek.is_ascii_alphanumeric()
                {
                    let ch = self.next_char().unwrap();
                    iden.push(ch);
                }

                match iden.as_str() {
                    "if" => Some(TokenValue::If),
                    "else" => Some(TokenValue::Else),
                    "while" => Some(TokenValue::While),
                    "do" => Some(TokenValue::Do),
                    "for" => Some(TokenValue::For),
                    "return" => Some(TokenValue::Return),
                    "continue" => Some(TokenValue::Continue),
                    "break" => Some(TokenValue::Break),
                    "void" => Some(TokenValue::Void),
                    "int" => Some(TokenValue::Int),
                    "float" => Some(TokenValue::Float),
                    "bool" => Some(TokenValue::Bool),
                    "true" => Some(TokenValue::True),
                    "false" => Some(TokenValue::False),
                    "nullptr" => Some(TokenValue::Nullptr),
                    "sizeof" => Some(TokenValue::Sizeof),
                    "struct" => Some(TokenValue::Struct),
                    "enum" => Some(TokenValue::Enum),
                    "typedef" => Some(TokenValue::Typedef),
                    _ => Some(TokenValue::Iden(iden)),
                }
            }
            Some(ch) => return Err(ScanError::UnexpectedChar(ch)),
            None => None,
        };

        Ok(val.map(|val| Token {
            val,
            pos: initial_pos,
            line: initial_line,
            col: initial_col,
        }))
    }
}

impl<'a> Iterator for Scanner<'a> {
    type Item = Result<Token, ScanError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token().transpose()
    }
}
