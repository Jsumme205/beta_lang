use std::rc::Rc;

use crate::betac_lexer::ast_types::{Expr, Token};

pub trait BetaError {
    type Token;
    type Expr;

    fn line(&self) -> usize;

    fn span(&self) -> (usize, usize);

    fn token(&self) -> Rc<Self::Token>;

    fn expression(&self) -> Option<&Self::Expr>;

    fn report(self);
}

pub trait ErrorBuilder {
    type Built: BetaError;

    fn line(self, line: usize) -> Self;

    fn column(self, column: usize) -> Self;

    fn token(self, token: Rc<<Self::Built as BetaError>::Token>) -> Self;

    fn expression(self, expr: <Self::Built as BetaError>::Expr) -> Self;

    fn finish(self) -> Option<Self::Built>;

    fn finish_and_report(self)
    where
        Self: Sized,
    {
        self.finish().unwrap().report();
    }
}

pub trait Emitter {
    type Builder: ErrorBuilder;

    fn emit(&self) -> Self::Builder;

    fn report(
        &self,
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

            fn finish(self) -> Option<Self::Built> {
                Some(<$built>::new(
                    self.token?,
                    self.expr,
                    self.line,
                    self.column,
                    self.emitter,
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
    emitter: &'a dyn Emitter<Builder = Self>,
}

impl_builder_for_error!(LexerBuilder<'a>, LexerError<'a>);

pub struct LexerError<'a> {
    token: Rc<Token<'a>>,
    expr: Option<Expr<'a>>,
    line: usize,
    column: usize,
    emitter: &'a dyn Emitter<Builder = LexerBuilder<'a>>,
}

impl<'a> LexerError<'a> {
    fn new(
        token: Rc<Token<'a>>,
        expr: Option<Expr<'a>>,
        line: usize,
        column: usize,
        emitter: &'a dyn Emitter<Builder = LexerBuilder<'a>>,
    ) -> Self {
        Self {
            token,
            expr,
            line,
            column,
            emitter,
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
        todo!()
    }

    fn expression(&self) -> Option<&Self::Expr> {
        self.expr.as_ref()
    }
}
