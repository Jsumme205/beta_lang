use super::Tokenizer;
use std::{num::NonZero, sync::mpsc};

#[derive(Debug, Clone, Copy)]
pub struct Token {
    kind: TokenKind,
    start: u32,
}

impl Token {
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

    Ident,
    Lifetime,
    Literal,
    Unknown,
}

impl TokenKind {
    fn single_char(c: u8) -> Option<TokenKind> {
        match c {
            32..=47 | 58..=64 | 91..=96 | 123..=126 => unsafe { Some(std::mem::transmute(c)) },
            _ => None,
        }
    }
}

impl<'a> Tokenizer<'a> {
    pub fn advance_token(&mut self) -> Token {
        let start = self.idx;
        let kind = match self.bump().unwrap_or('\0') {
            '=' if self.next() == '>' => {
                self.bump();
                TokenKind::FatArrow
            }
            '&' if self.next() == '&' => {
                self.bump();
                TokenKind::AndAnd
            }
            '|' if self.next() == '|' => {
                self.bump();
                TokenKind::PipePipe
            }
            '=' if self.next() == '=' => {
                self.bump();
                TokenKind::EqEq
            }
            '!' if self.next() == '=' => {
                self.bump();
                TokenKind::NotEq
            }
            '>' if self.next() == '=' => {
                self.bump();
                TokenKind::GtEq
            }
            '<' if self.next() == '=' => {
                self.bump();
                TokenKind::LtEq
            }
            '"' => self.handle_literal_string(),
            '\'' => self.handle_literal_char(),
            ident if ident.is_ascii_alphabetic() || ident == '_' => self.handle_ident(),
            c if let Some(kind) = TokenKind::single_char(c as u8) => kind,
            num if num.is_ascii_digit() => self.handle_number(),
            _ => TokenKind::Unknown,
        };
        self.bump();
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
        TokenKind::Ident
    }
}

pub fn run_tokenizer(input: String) -> (mpsc::Receiver<Token>, std::thread::JoinHandle<()>) {
    let (tx, rx) = mpsc::channel();

    let handle = std::thread::spawn(move || {
        let mut tokenizer = Tokenizer::new(&*input);
        let iter = std::iter::from_fn(|| {
            let next = tokenizer.advance_token();
            if next.kind == TokenKind::Eof {
                None
            } else {
                Some(next)
            }
        });

        for token in iter {
            tx.send(token).unwrap();
        }
    });

    (rx, handle)
}
