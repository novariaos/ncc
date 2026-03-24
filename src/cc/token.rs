#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    IntLit(i32),
    StrLit(String),
    CharLit(i32),

    Ident(String),

    If,
    Else,
    While,
    For,
    Do,
    Return,
    Void,
    Int,
    Char,
    Struct,
    Const,
    Static,
    Sizeof,
    Switch,
    Case,
    Default,
    Break,
    Colon,

    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Amp,
    Bang,
    Dot,
    Arrow,
    Eq,
    EqEq,
    BangEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    AmpAmp,
    PipePipe,
    PlusPlus,
    MinusMinus,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,

    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Semi,
    Comma,
    Ellipsis,

    Eof,
}

impl Token {
    pub fn is_type_keyword(&self) -> bool {
        matches!(self, Token::Void | Token::Int | Token::Char | Token::Struct)
    }

    pub fn can_start_type(&self) -> bool {
        matches!(
            self,
            Token::Void | Token::Int | Token::Char | Token::Struct | Token::Const | Token::Static
        )
    }
}
