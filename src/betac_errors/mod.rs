use std::{rc::Rc, sync::atomic::Ordering};

use crate::betac_lexer::{
    ast_types::{Expr, Token},
    Lexer,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Warning,
    HardError,
    SoftError,
}

pub trait BetaError {
    type Token;
    type Expr;

    fn line(&self) -> usize;

    fn span(&self) -> (usize, usize);

    fn token(&self) -> Rc<Self::Token>;

    fn expression(&self) -> Option<&Self::Expr>;

    fn level(&self) -> Level;

    fn report(self);
}

pub trait ErrorBuilder {
    type Built: BetaError;

    fn line(self, line: usize) -> Self;

    fn column(self, column: usize) -> Self;

    fn token(self, token: Rc<<Self::Built as BetaError>::Token>) -> Self;

    fn expression(self, expr: <Self::Built as BetaError>::Expr) -> Self;

    fn message(self, message: String) -> Self;

    fn level(self, level: Level) -> Self;

    fn finish(self) -> Option<Self::Built>;

    fn finish_and_report(self)
    where
        Self: Sized,
    {
        self.finish().unwrap().report();
    }
}

pub trait Emitter<'a>: EmitterPrivate {
    type Builder: ErrorBuilder;

    fn emit(&'a self) -> Self::Builder;

    fn report(
        &'a self,
        line: usize,
        token: Rc<<<Self::Builder as ErrorBuilder>::Built as BetaError>::Token>,
        expr: <<Self::Builder as ErrorBuilder>::Built as BetaError>::Expr,
    ) {
        self.emit()
            .line(line)
            .token(token)
            .expression(expr)
            .finish_and_report();
    }

    fn try_reset(&mut self) -> Result<(), ()>;

    fn drain(&self);
}

trait EmitterPrivate {
    type Error;

    fn insert_error(&self, error: Self::Error);

    fn poison(&self);

    fn is_poisoned(&self) -> bool;
}

macro_rules! impl_builder_for_error {
    ($ty:ty, $built:ty) => {
        impl<'a> ErrorBuilder for $ty {
            type Built = $built;

            fn line(mut self, line: usize) -> Self {
                self.line = line;
                self
            }

            fn column(mut self, column: usize) -> Self {
                self.column = column;
                self
            }

            fn token(mut self, token: Rc<<Self::Built as BetaError>::Token>) -> Self {
                self.token = Some(token);
                self
            }

            fn expression(mut self, expr: <Self::Built as BetaError>::Expr) -> Self {
                self.expr = Some(expr);
                self
            }

            fn message(mut self, message: String) -> Self {
                self.message = Some(message);
                self
            }

            fn level(mut self, level: Level) -> Self {
                self.level = Some(level);
                self
            }

            fn finish(self) -> Option<Self::Built> {
                Some(<$built>::new(
                    self.token?,
                    self.expr,
                    self.line,
                    self.column,
                    self.emitter,
                    self.message,
                    self.level,
                ))
            }
        }
    };
}

pub struct LexerBuilder<'a> {
    token: Option<Rc<Token<'a>>>,
    expr: Option<Expr<'a>>,
    line: usize,
    column: usize,
    emitter: &'a dyn Emitter<'a, Builder = Self, Error = <Self as ErrorBuilder>::Built>,
    message: Option<String>,
    level: Option<Level>,
}

impl_builder_for_error!(LexerBuilder<'a>, LexerError<'a>);

pub struct LexerError<'a> {
    token: Rc<Token<'a>>,
    expr: Option<Expr<'a>>,
    line: usize,
    column: usize,
    emitter: &'a dyn Emitter<'a, Builder = LexerBuilder<'a>, Error = Self>,
    message: Option<String>,
    level: Level,
}

impl<'a> LexerError<'a> {
    fn new(
        token: Rc<Token<'a>>,
        expr: Option<Expr<'a>>,
        line: usize,
        column: usize,
        emitter: &'a dyn Emitter<'a, Builder = LexerBuilder<'a>, Error = Self>,
        message: Option<String>,
        level: Option<Level>,
    ) -> Self {
        Self {
            token,
            expr,
            line,
            column,
            emitter,
            message,
            level: level.unwrap_or(Level::HardError),
        }
    }
}

impl<'a> BetaError for LexerError<'a> {
    type Token = Token<'a>;
    type Expr = Expr<'a>;

    fn line(&self) -> usize {
        self.line
    }

    fn span(&self) -> (usize, usize) {
        self.token.as_span()
    }

    fn token(&self) -> Rc<Self::Token> {
        self.token.clone()
    }

    fn report(self) {
        self.emitter.insert_error(self);
    }

    fn expression(&self) -> Option<&Self::Expr> {
        self.expr.as_ref()
    }

    fn level(&self) -> Level {
        self.level
    }
}

impl<'a> EmitterPrivate for Lexer<'a> {
    type Error = LexerError<'a>;

    fn insert_error(&self, error: Self::Error) {
        self.errors.write().unwrap().push(error);
    }

    fn poison(&self) {
        self.guard.store(true, Ordering::SeqCst);
    }

    fn is_poisoned(&self) -> bool {
        self.guard.load(Ordering::Acquire)
    }
}

impl<'a> Emitter<'a> for Lexer<'a> {
    type Builder = LexerBuilder<'a>;

    fn emit(&'a self) -> Self::Builder {
        LexerBuilder {
            token: None,
            expr: None,
            line: self.nl_count(),
            column: self.column(),
            emitter: self,
            message: None,
            level: None,
        }
    }

    fn try_reset(&mut self) -> Result<(), ()> {
        todo!("put a similar loop to advance to the next expression")
    }

    fn drain(&self) {
        let mut errors = self.errors.write().unwrap();
        errors.drain(..).map(|error| {});
    }
}
