use std::fmt::Debug;

use crate::{betac_ast::Span, betac_tokenizer::token::Token};

use super::{Emitter, Level, Reportable};

#[derive(Debug)]
pub struct UnexpectedResult {
    line: u32,
    column: u32,
    message: String,
}

impl UnexpectedResult {
    pub fn builder() -> Self {
        Self {
            line: 0,
            column: 0,
            message: String::new(),
        }
    }

    pub fn line(mut self, line: u32) -> Self {
        self.line = line;
        self
    }

    pub fn column(mut self, column: u32) -> Self {
        self.column = column;
        self
    }

    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }
}

impl Reportable for UnexpectedResult {
    fn line(&self) -> u32 {
        self.line as u32
    }

    fn column(&self) -> u32 {
        self.column as u32
    }

    fn span(&self) -> crate::betac_ast::Span {
        Span::DUMMY
    }

    fn level(&self) -> super::Level {
        Level::Error
    }

    fn report(self) {
        Emitter::with(|mut lock| lock.errors.push(Box::new(self)))
    }

    fn message(&self) -> &str {
        &self.message
    }
}

pub trait ResultExtension {
    type Output;

    fn unwrap_or_emit(self) -> Self::Output;
}

impl<E: Debug> ResultExtension for Result<Token, E> {
    type Output = Token;

    fn unwrap_or_emit(self) -> Self::Output {
        match self {
            Self::Ok(ok) => ok,
            Self::Err(err) => {
                UnexpectedResult::builder()
                    .column(column!())
                    .line(line!())
                    .message(format!("expected: Token, found: {err:?}"))
                    .report();
                Token::DUMMMY
            }
        }
    }
}

impl ResultExtension for Option<Token> {
    type Output = Token;

    fn unwrap_or_emit(self) -> Self::Output {
        match self {
            Self::Some(ok) => ok,
            Self::None => {
                UnexpectedResult::builder()
                    .column(column!())
                    .line(line!())
                    .message(format!("expected: Token, found None"))
                    .report();
                Token::DUMMMY
            }
        }
    }
}
