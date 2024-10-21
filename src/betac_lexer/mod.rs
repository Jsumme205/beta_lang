pub mod ast_types;

use std::rc::Rc;

use ast_types::{AnyMetadata, Metadata, CONSTEXPR, PUBLIC, STATIC};

use crate::{
    betac_pp::cursor::CursorLike,
    betac_util::{self, session::Session, sso::Bytes, Yarn},
};

pub struct SourceCodeReader<'a> {
    input: Bytes<'a>,
    last: u8,
    next: u8,
}

impl<'a> SourceCodeReader<'a> {
    pub fn init(input: &'a Yarn<'a>) -> Self {
        let iter = input.bytes();
        Self {
            input: iter,
            last: 0,
            next: input[1],
        }
    }

    pub fn next_token(&mut self) -> Rc<ast_types::Token<'a>> {
        let token = match self.bump().unwrap_or('\0') {
            '\0' => ast_types::Token::Eof,
            '(' => ast_types::Token::LeftParen,
            '{' => ast_types::Token::LeftBrace,
            '[' => ast_types::Token::LeftBracket,
            ')' => ast_types::Token::LeftParen,
            '}' => ast_types::Token::RightBrace,
            ']' => ast_types::Token::RightBracket,
            ':' if CursorLike::next(self) == ':' => {
                self.bump();
                ast_types::Token::Path
            }
            ':' => ast_types::Token::Colon,
            ';' => ast_types::Token::Semi,
            '=' if CursorLike::next(self) == '>' => {
                self.bump();
                ast_types::Token::Assign
            }
            c if c.is_numeric() => self.number(),
            c if betac_util::is_whitespace(c as u8) => ast_types::Token::Whitespace,
            c if betac_util::is_id_start(c as u8) => self.ident(c),
            _ => todo!(),
        };
        Rc::new(token)
    }

    fn number(&mut self) -> ast_types::Token<'a> {
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
        ast_types::Token::Number(number)
    }

    fn ident(&mut self, c: char) -> ast_types::Token<'a> {
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
        ast_types::Token::Ident(ident)
    }
}

impl<'a> Iterator for SourceCodeReader<'a> {
    type Item = Rc<ast_types::Token<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.next_token();
        if *next == ast_types::Token::Eof {
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
        self.next as char
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
}

impl<'a> Lexer<'a> {
    pub fn init(input: &'a Yarn<'a>, session: &'a mut Session) -> Self {
        let mut token_reader = SourceCodeReader::init(input);
        let last_significant_token = token_reader.next_token();
        Self {
            last_significant_token,
            currently_evaluated_token: token_reader.next_token(),
            token_reader,
            session,
        }
    }

    pub fn advance(&mut self) {
        if *self.currently_evaluated_token != ast_types::Token::Whitespace {
            self.last_significant_token = std::mem::take(&mut self.currently_evaluated_token);
        }
        self.currently_evaluated_token = self.token_reader.next_token();
    }

    pub fn parse_next_expr(&mut self) -> Option<ast_types::Expr<'a>> {
        loop {
            self.advance();
            let mut meta = AnyMetadata::init();
            match &*self.currently_evaluated_token {
                expr_begin if self.currently_evaluated_token.ident_is_expr_start() => {
                    match expr_begin.as_ident().unwrap().as_str() {
                        "let" => return self.assignment(meta.to_assignment()),
                        _ => todo!(),
                    }
                }
                modifier if self.currently_evaluated_token.ident_is_modifier() => {
                    match modifier.as_ident().unwrap().as_str() {
                        "pub" => meta = meta.add_flag(PUBLIC),
                        "constexpr" => meta = meta.add_flag(CONSTEXPR),
                        "static" => meta = meta.add_flag(STATIC),
                        _ => unreachable!(),
                    }
                }
                ast_types::Token::Ident(id) => {}
                ast_types::Token::Whitespace => continue,
                _ => todo!(),
            }
        }
    }
}
