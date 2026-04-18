use derive_more::Display;
use kinded::Kinded;

#[derive(Debug, Clone, Display, PartialEq, Eq)]
pub enum TokenError {
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

#[derive(Kinded, Debug, Clone, PartialEq)]
#[kinded(kind = TokenKind)]
pub enum Token {
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
    #[kinded(rename = "bool literal")]
    BoolLit(bool),
    #[kinded(rename = "numeric literal")]
    NumLit(u32),
    #[kinded(rename = "string literal")]
    StrLit(String),
    #[kinded(rename = "identifier")]
    Iden(String),
    #[kinded(rename = "error")]
    Error(TokenError),
    #[kinded(rename = "'fn'")]
    Fn,
    #[kinded(rename = "'let'")]
    Let,
    #[kinded(rename = "'mut'")]
    Mut,
    #[kinded(rename = "'null'")]
    Null,
}

impl Token {
    pub fn assume_num_lit(self) -> u32 {
        match self {
            Self::NumLit(n) => n,
            _ => unreachable!(),
        }
    }

    pub fn assume_str_lit(self) -> String {
        match self {
            Self::StrLit(s) => s,
            _ => unreachable!(),
        }
    }

    pub fn assume_bool_lit(self) -> bool {
        match self {
            Self::BoolLit(b) => b,
            _ => unreachable!(),
        }
    }

    pub fn assume_iden(self) -> String {
        match self {
            Self::Iden(name) => name,
            _ => unreachable!(),
        }
    }
}
