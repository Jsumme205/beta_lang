use super::Tokenizer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub start: u32,
}

impl Token {
    pub const DUMMMY: Self = Self {
        kind: TokenKind::Eof,
        start: 0,
    };

    pub fn len(&self) -> Option<u32> {
        use TokenKind::*;
        match self.kind {
            At | Eq | LeftBrace | RightBrace | LeftParen | RightParen | LeftBracket
            | RightBracket | Ampersand | Pipe | Star | Semi | Colon | Comma | Whitespace | Lt
            | Gt | Eof => Some(1),
            AndAnd | PipePipe | FatArrow | EqEq | NotEq | LtEq | GtEq => Some(2),
            _ => None,
        }
    }

    pub fn kind(&self) -> TokenKind {
        self.kind
    }

    pub fn is_whitespace_or_newline(&self) -> bool {
        match self.kind {
            TokenKind::Eof | TokenKind::Whitespace => true,
            _ => false,
        }
    }
}

impl std::ops::Sub for Token {
    type Output = u32;

    /// for `Token` this subtracts the `start` from rhs
    fn sub(self, rhs: Self) -> Self::Output {
        self.start - rhs.start
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum TokenKind {
    At = b'@',
    Ampersand = b'&',
    Pipe = b'|',
    Star = b'*',
    Eq = b'=',
    Not = b'!',
    Lt = b'<',
    Gt = b'>',
    LeftParen = b'(',
    RightParen = b')',
    LeftBrace = b'[',
    RightBrace = b']',
    LeftBracket = b'{',
    RightBracket = b'}',
    Semi = b';',
    Colon = b':',
    Comma = b',',
    Whitespace = b' ',
    Quote = b'"',
    Pound = b'#',
    Dollar = b'$',
    Question = b'?',
    Percent = b'%',
    SingleQuote = b'\'',
    Minus = b'-',
    Dot = b'.',
    ForwardSlash = b'/',
    BackSlash = b'\\',
    Carat = b'^',
    Underscore = b'_',
    Dash = b'`',
    Tilde = b'~',
    Eof = b'\0',
    /// &&
    AndAnd,
    /// ||
    PipePipe,
    /// =>
    FatArrow,
    /// ==
    EqEq,
    /// !=
    NotEq,
    /// <=
    LtEq,
    /// >=
    GtEq,
    /// ::
    Path,
    Ident,
    Lifetime,
    Literal,
    NewLine,
    Unknown,
}

impl TokenKind {
    fn single_char(c: u8) -> Option<TokenKind> {
        match c {
            // SAFETY: TokenKind is guaranteed to be valid at these ranges
            32..=47 | 58..=64 | 91..=96 | 123..=126 => unsafe { Some(std::mem::transmute(c)) },
            _ => None,
        }
    }
}

impl<'a> Tokenizer<'a> {
    pub fn advance_token(&mut self) -> Token {
        let start = self.idx;
        let kind = match self.bump().unwrap_or('\0') {
            '=' if self.nth_next(2) == '>' => {
                println!("found => at: {start}");
                self.bump();
                TokenKind::FatArrow
            }
            '&' if self.nth_next(2) == '&' => {
                self.bump();
                TokenKind::AndAnd
            }
            '|' if self.nth_next(2) == '|' => {
                self.bump();
                TokenKind::PipePipe
            }
            '=' if self.nth_next(2) == '=' => {
                self.bump();
                TokenKind::EqEq
            }
            '!' if self.nth_next(2) == '=' => {
                self.bump();
                TokenKind::NotEq
            }
            '>' if self.nth_next(2) == '=' => {
                self.bump();
                TokenKind::GtEq
            }
            '<' if self.nth_next(2) == '=' => {
                self.bump();
                TokenKind::LtEq
            }
            ':' if self.nth_next(2) == ':' => {
                self.bump();
                TokenKind::Path
            }
            '"' => self.handle_literal_string(),
            '\'' => self.handle_literal_char(),
            ident if ident.is_ascii_alphabetic() || ident == '_' => self.handle_ident(),
            c if let Some(kind) = TokenKind::single_char(c as u8) => kind,
            num if num.is_ascii_digit() => self.handle_number(),
            '\n' => TokenKind::NewLine,
            c => {
                if c == '\n' {
                    println!("newline found");
                }
                println!("unexpected: {c}");
                TokenKind::Unknown
            }
        };
        Token {
            kind,
            start: start as u32,
        }
    }

    fn handle_number(&mut self) -> TokenKind {
        self.eat_while(|c| c.is_ascii_hexdigit());
        TokenKind::Literal
    }

    fn handle_literal_char(&mut self) -> TokenKind {
        match self.bump().unwrap_or('\0') {
            '\0' => return TokenKind::Eof,
            '\\' => {
                self.bump();
            }
            _ => {}
        }
        TokenKind::Literal
    }

    fn handle_literal_string(&mut self) -> TokenKind {
        self.eat_while(|c| c != '"');
        TokenKind::Literal
    }

    fn handle_ident(&mut self) -> TokenKind {
        self.eat_while(|c| c.is_ascii_alphanumeric());
        self.bump();
        TokenKind::Ident
    }
}
