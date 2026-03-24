use super::token::Token;

pub struct Lexer {
    src: Vec<char>,
    pos: usize,
    line: u32,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            src: source.chars().collect(),
            pos: 0,
            line: 1,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            if self.pos >= self.src.len() {
                tokens.push(Token::Eof);
                break;
            }
            let tok = self.next_token()?;
            tokens.push(tok);
        }
        Ok(tokens)
    }

    fn peek(&self) -> Option<char> {
        self.src.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<char> {
        self.src.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> char {
        let ch = self.src[self.pos];
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
        }
        ch
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            while self.pos < self.src.len() && self.src[self.pos].is_ascii_whitespace() {
                self.advance();
            }

            if self.pos + 1 < self.src.len() && self.src[self.pos] == '/' && self.src[self.pos + 1] == '/' {
                while self.pos < self.src.len() && self.src[self.pos] != '\n' {
                    self.pos += 1;
                }
                continue;
            }

            if self.pos + 1 < self.src.len() && self.src[self.pos] == '/' && self.src[self.pos + 1] == '*' {
                self.pos += 2;
                while self.pos + 1 < self.src.len() {
                    if self.src[self.pos] == '\n' {
                        self.line += 1;
                    }
                    if self.src[self.pos] == '*' && self.src[self.pos + 1] == '/' {
                        self.pos += 2;
                        break;
                    }
                    self.pos += 1;
                }
                continue;
            }

            break;
        }
    }

    fn next_token(&mut self) -> Result<Token, String> {
        let ch = self.peek().unwrap();

        if ch.is_ascii_digit() {
            return self.lex_number();
        }

        if ch == '"' {
            return self.lex_string();
        }

        if ch == '\'' {
            return self.lex_char();
        }

        if ch.is_ascii_alphabetic() || ch == '_' {
            return Ok(self.lex_ident());
        }

        self.lex_punct()
    }

    fn lex_number(&mut self) -> Result<Token, String> {
        let start = self.pos;

        if self.peek() == Some('0') && self.peek2().map(|c| c == 'x' || c == 'X').unwrap_or(false) {
            self.advance();
            self.advance();
            let hex_start = self.pos;
            while self.pos < self.src.len() && self.src[self.pos].is_ascii_hexdigit() {
                self.pos += 1;
            }
            let hex: String = self.src[hex_start..self.pos].iter().collect();
            let val = i32::from_str_radix(&hex, 16)
                .map_err(|_| format!("line {}: invalid hex literal 0x{}", self.line, hex))?;
            return Ok(Token::IntLit(val));
        }

        while self.pos < self.src.len() && self.src[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        let num: String = self.src[start..self.pos].iter().collect();
        let val = num
            .parse::<i32>()
            .map_err(|_| format!("line {}: invalid integer literal {}", self.line, num))?;
        Ok(Token::IntLit(val))
    }

    fn lex_string(&mut self) -> Result<Token, String> {
        self.advance();
        let mut s = String::new();
        loop {
            match self.peek() {
                None => return Err(format!("line {}: unterminated string literal", self.line)),
                Some('"') => {
                    self.advance();
                    break;
                }
                Some('\\') => {
                    self.advance();
                    let esc = self.advance();
                    s.push(match esc {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '\\' => '\\',
                        '\'' => '\'',
                        '"' => '"',
                        '0' => '\0',
                        _ => return Err(format!("line {}: unknown escape \\{}", self.line, esc)),
                    });
                }
                Some(ch) => {
                    self.advance();
                    s.push(ch);
                }
            }
        }
        Ok(Token::StrLit(s))
    }

    fn lex_char(&mut self) -> Result<Token, String> {
        self.advance();
        let val = match self.peek() {
            Some('\\') => {
                self.advance();
                let esc = self.advance();
                match esc {
                    'n' => '\n' as i32,
                    't' => '\t' as i32,
                    'r' => '\r' as i32,
                    '\\' => '\\' as i32,
                    '\'' => '\'' as i32,
                    '"' => '"' as i32,
                    '0' => 0,
                    _ => return Err(format!("line {}: unknown escape \\{}", self.line, esc)),
                }
            }
            Some(ch) => {
                self.advance();
                ch as i32
            }
            None => return Err(format!("line {}: unterminated char literal", self.line)),
        };
        if self.peek() != Some('\'') {
            return Err(format!("line {}: unterminated char literal", self.line));
        }
        self.advance();
        Ok(Token::CharLit(val))
    }

    fn lex_ident(&mut self) -> Token {
        let start = self.pos;
        while self.pos < self.src.len()
            && (self.src[self.pos].is_ascii_alphanumeric() || self.src[self.pos] == '_')
        {
            self.pos += 1;
        }
        let word: String = self.src[start..self.pos].iter().collect();
        match word.as_str() {
            "if" => Token::If,
            "else" => Token::Else,
            "while" => Token::While,
            "for" => Token::For,
            "do" => Token::Do,
            "return" => Token::Return,
            "void" => Token::Void,
            "int" => Token::Int,
            "char" => Token::Char,
            "struct" => Token::Struct,
            "const" => Token::Const,
            "static" => Token::Static,
            "sizeof" => Token::Sizeof,
            "switch" => Token::Switch,
            "case" => Token::Case,
            "default" => Token::Default,
            "break" => Token::Break,
            _ => Token::Ident(word),
        }
    }

    fn lex_punct(&mut self) -> Result<Token, String> {
        let ch = self.advance();
        match ch {
            '(' => Ok(Token::LParen),
            ')' => Ok(Token::RParen),
            '{' => Ok(Token::LBrace),
            '}' => Ok(Token::RBrace),
            '[' => Ok(Token::LBracket),
            ']' => Ok(Token::RBracket),
            ';' => Ok(Token::Semi),
            ':' => Ok(Token::Colon),
            ',' => Ok(Token::Comma),
            '~' => Ok(Token::Bang),
            '+' => {
                if self.peek() == Some('+') {
                    self.advance();
                    Ok(Token::PlusPlus)
                } else if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::PlusEq)
                } else {
                    Ok(Token::Plus)
                }
            }
            '-' => {
                if self.peek() == Some('-') {
                    self.advance();
                    Ok(Token::MinusMinus)
                } else if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::MinusEq)
                } else if self.peek() == Some('>') {
                    self.advance();
                    Ok(Token::Arrow)
                } else {
                    Ok(Token::Minus)
                }
            }
            '*' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::StarEq)
                } else {
                    Ok(Token::Star)
                }
            }
            '/' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::SlashEq)
                } else {
                    Ok(Token::Slash)
                }
            }
            '%' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::PercentEq)
                } else {
                    Ok(Token::Percent)
                }
            }
            '&' => {
                if self.peek() == Some('&') {
                    self.advance();
                    Ok(Token::AmpAmp)
                } else {
                    Ok(Token::Amp)
                }
            }
            '|' => {
                if self.peek() == Some('|') {
                    self.advance();
                    Ok(Token::PipePipe)
                } else {
                    Err(format!("line {}: unexpected '|' (bitwise OR not supported)", self.line))
                }
            }
            '!' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::BangEq)
                } else {
                    Ok(Token::Bang)
                }
            }
            '=' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::EqEq)
                } else {
                    Ok(Token::Eq)
                }
            }
            '<' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::LtEq)
                } else {
                    Ok(Token::Lt)
                }
            }
            '>' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::GtEq)
                } else {
                    Ok(Token::Gt)
                }
            }
            '.' => {
                if self.peek() == Some('.') && self.peek2() == Some('.') {
                    self.advance();
                    self.advance();
                    Ok(Token::Ellipsis)
                } else {
                    Ok(Token::Dot)
                }
            }
            _ => Err(format!("line {}: unexpected character '{}'", self.line, ch)),
        }
    }
}
