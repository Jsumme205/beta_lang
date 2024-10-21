pub mod ast_types;

use std::rc::Rc;

use ast_types::{AnyMetadata, Metadata, Token, CONSTEXPR, PUBLIC, STATIC};

use crate::{
    betac_pp::cursor::CursorLike,
    betac_util::{self, session::Session, sso::Bytes, Yarn},
};

pub struct SourceCodeReader<'a> {
    input: Bytes<'a>,
    last: u8,
    next: u8,
    column: u32,
    pos: u32,
}

impl<'a> SourceCodeReader<'a> {
    pub fn init(input: &'a Yarn<'a>) -> Self {
        let iter = input.bytes();
        Self {
            input: iter,
            last: 0,
            next: input[1],
            column: 0,
            pos: 0,
        }
    }

    pub fn next_token(&mut self) -> Rc<ast_types::Token<'a>> {
        let start = self.pos;
        let (raw_token, pos) = match self.bump().unwrap_or('\0') {
            '\0' => (ast_types::RawToken::Eof, 1),
            '(' => (ast_types::RawToken::LeftParen, 1),
            '{' => (ast_types::RawToken::LeftBrace, 1),
            '[' => (ast_types::RawToken::LeftBracket, 1),
            ')' => (ast_types::RawToken::LeftParen, 1),
            '}' => (ast_types::RawToken::RightBrace, 1),
            ']' => (ast_types::RawToken::RightBracket, 1),
            ':' if CursorLike::next(self) == ':' => {
                self.bump();
                (ast_types::RawToken::Path, 2)
            }
            ':' => (ast_types::RawToken::Colon, 1),
            ';' => (ast_types::RawToken::Semi, 1),
            '=' if CursorLike::next(self) == '>' => {
                self.bump();
                (ast_types::RawToken::Assign, 2)
            }
            '\n' => {
                self.column = 0;
                (ast_types::RawToken::NewLine, 0)
            }
            c if c.is_numeric() => self.number(),
            c if betac_util::is_whitespace(c as u8) => (ast_types::RawToken::Whitespace, 1),
            c if betac_util::is_id_start(c as u8) => self.ident(c),
            c => {
                println!("failed at: {c}");
                todo!()
            }
        };

        self.column += pos;
        Rc::new(Token::new(raw_token, start, start + pos))
    }

    pub fn column(&self) -> u32 {
        self.column
    }

    fn number(&mut self) -> (ast_types::RawToken<'a>, u32) {
        let number: Yarn<'a> = unsafe {
            Yarn::from_utf8_unchecked_owned(
                self.bump_while(|c| {
                    let c = *c;
                    c.is_numeric() || c != 'x' || c != 'X' || c != 'b' || c != 'B'
                })
                .into_iter()
                .map(|c| c as u8),
            )
        };
        let len = number.len();
        (ast_types::RawToken::Number(number), len as u32)
    }

    fn ident(&mut self, c: char) -> (ast_types::RawToken<'a>, u32) {
        let mut buf = vec![c];
        let ident: Yarn<'a> = unsafe {
            Yarn::from_utf8_unchecked_owned(
                self.bump_while(|c| {
                    println!("c_69: {c}");
                    betac_util::is_id_continue(*c as u8)
                })
                .into_iter()
                .collect_into(&mut buf)
                .into_iter()
                .map(|c| *c as u8),
            )
        };
        let len = ident.len() as u32;
        (ast_types::RawToken::Ident(ident), len)
    }
}

impl<'a> Iterator for SourceCodeReader<'a> {
    type Item = Rc<ast_types::Token<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.next_token();
        if *next == ast_types::RawToken::Eof {
            None
        } else {
            Some(next)
        }
    }
}

impl<'a> CursorLike for SourceCodeReader<'a> {
    type Element = char;

    fn as_yarn(&self) -> Yarn<'_> {
        self.input.as_yarn()
    }

    fn prev(&self) -> Self::Element {
        self.last as char
    }

    fn next(&self) -> Self::Element {
        self.input.clone().next().unwrap_or(0) as char
    }

    fn second(&self) -> Self::Element {
        self.input.clone().next().unwrap_or(0) as char
    }

    fn nth(&self, n: usize) -> Self::Element {
        let mut iter = self.input.clone();
        for _ in 0..(n - 1) {
            iter.next();
        }
        iter.next().unwrap_or(0) as char
    }

    fn bump(&mut self) -> Option<Self::Element> {
        let c = self.input.next()?;
        self.last = c;
        self.pos += 1;
        Some(c as char)
    }

    fn is_at_end(&self) -> bool {
        self.input.is_end()
    }

    fn pos_within_token(&self) -> usize {
        unimplemented!()
    }

    fn reset_pos_within_token(&mut self) {
        unimplemented!()
    }
}

pub struct Lexer<'a> {
    token_reader: SourceCodeReader<'a>,
    last_significant_token: Rc<ast_types::Token<'a>>,
    currently_evaluated_token: Rc<ast_types::Token<'a>>,
    session: &'a mut Session,
    nl_count: usize,
}

impl<'a> Lexer<'a> {
    pub fn init(input: &'a Yarn<'a>, session: &'a mut Session) -> Self {
        let mut token_reader = SourceCodeReader::init(input);
        Self {
            last_significant_token: Rc::default(),
            currently_evaluated_token: token_reader.next_token(),
            token_reader,
            session,
            nl_count: 0,
        }
    }

    pub(self) fn advance(&mut self) {
        if *self.currently_evaluated_token.as_raw() != ast_types::RawToken::Whitespace {
            self.last_significant_token = self.currently_evaluated_token.clone();
        }
        self.currently_evaluated_token = self.token_reader.next_token();
        if *self.currently_evaluated_token.as_raw() == ast_types::RawToken::NewLine {
            self.nl_count += 1;
        }
    }

    pub fn parse_next_expr(&mut self) -> Option<ast_types::Expr<'a>> {
        println!("initial_token: {:#?}", self.currently_evaluated_token);
        loop {
            println!("got to 177");
            println!("current: {:#?}", self.currently_evaluated_token);
            let mut meta = AnyMetadata::init();
            match &*self.currently_evaluated_token {
                expr_begin if self.currently_evaluated_token.ident_is_expr_start() => {
                    println!("got to 182");
                    match expr_begin.as_ident().unwrap().as_str() {
                        "let" => {
                            println!("got to 183");
                            return self.assignment(meta.to_assignment());
                        }
                        _ => todo!(),
                    }
                }
                modifier if self.currently_evaluated_token.ident_is_modifier() => {
                    println!("got to 192");
                    match modifier.as_ident().unwrap().as_str() {
                        "pub" => meta = meta.add_flag(PUBLIC),
                        "constexpr" => meta = meta.add_flag(CONSTEXPR),
                        "static" => meta = meta.add_flag(STATIC),
                        _ => unreachable!(),
                    }
                }
                ident if self.currently_evaluated_token.is_ident() => {
                    let ident = ident.as_raw();
                }
                _ if self.currently_evaluated_token.is_whitespace() => continue,
                _ => {
                    println!("failed at {:#?}", self.currently_evaluated_token);
                    todo!()
                }
            }
            self.advance();
        }
    }
}
