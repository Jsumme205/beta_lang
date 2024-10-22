use std::{
    fmt::Debug,
    io::{self, Write},
    ops::Deref,
    rc::Rc,
    sync::atomic::Ordering,
};

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

    fn drain(&self) -> io::Result<()>;
}

trait EmitterPrivate {
    type Error;

    fn insert_error(&self, error: Self::Error);

    fn poison(&self);

    fn is_poisoned(&self) -> bool;
}

macro_rules! impl_builder_for_error {
    ($ty:ty, $built:ty, $entry:ty) => {
        impl<'a, E: Emitter<'a, Error = $entry>> ErrorBuilder for $ty {
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

pub struct LexerBuilder<'a, E: Emitter<'a>> {
    token: Option<Rc<Token>>,
    expr: Option<Expr>,
    line: usize,
    column: usize,
    emitter: &'a E,
    message: Option<String>,
    level: Option<Level>,
}

impl_builder_for_error!(LexerBuilder<'a, E>, LexerError<'a, E>, LexerEntry);

pub struct LexerError<'a, E: Emitter<'a>> {
    token: Rc<Token>,
    expr: Option<Expr>,
    line: usize,
    column: usize,
    emitter: &'a E,
    message: Option<String>,
    level: Level,
}

pub struct LexerEntry {
    token: Rc<Token>,
    expr: Option<Expr>,
    line: usize,
    column: usize,
    message: Option<String>,
    level: Level,
}

impl<'a, E: Emitter<'a>> LexerError<'a, E> {
    fn new(
        token: Rc<Token>,
        expr: Option<Expr>,
        line: usize,
        column: usize,
        emitter: &'a E,
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

impl<'a, E: Emitter<'a, Error = LexerEntry>> BetaError for LexerError<'a, E> {
    type Token = Token;
    type Expr = Expr;

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
        let entry = LexerEntry {
            token: self.token,
            expr: self.expr,
            line: self.line,
            column: self.column,
            message: self.message,
            level: self.level,
        };
        self.emitter.insert_error(entry);
    }

    fn expression(&self) -> Option<&Self::Expr> {
        self.expr.as_ref()
    }

    fn level(&self) -> Level {
        self.level
    }
}

impl<'a> EmitterPrivate for Lexer<'a> {
    type Error = LexerEntry;

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
    type Builder = LexerBuilder<'a, Self>;

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

    fn drain(&self) -> io::Result<()> {
        let mut errors = self.errors.write().unwrap();
        for error in errors.drain(..) {
            let (mut io, msg) = match error.level {
                Level::HardError | Level::SoftError => (Io::Err(io::stderr().lock()), "ERROR:"),
                Level::Warning => (Io::Out(io::stdout().lock()), "WARNING:"),
            };

            writeln!(
                io,
                "{msg} {} at {}:{}",
                error.message.unwrap_or_default(),
                error.line,
                error.column,
            )?;
            writeln!(io, "current_token: {:#?}", error.token.as_raw())?;
            if error.expr.is_some() {
                writeln!(io, "expression: {:#?}", error.expr.unwrap())?;
            }
        }
        self.guard.store(false, Ordering::Relaxed);
        Ok(())
    }
}

pub enum Io<'a> {
    Err(io::StderrLock<'a>),
    Out(io::StdoutLock<'a>),
}

impl<'a> Write for Io<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Io::Err(e) => e.write(buf),
            Io::Out(o) => o.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Io::Err(e) => e.flush(),
            Io::Out(o) => o.flush(),
        }
    }

    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        match self {
            Io::Err(e) => e.write_vectored(bufs),
            Io::Out(o) => o.write_vectored(bufs),
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        match self {
            Io::Err(err) => err.write_all(buf),
            Io::Out(out) => out.write_all(buf),
        }
    }

    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }

    fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> io::Result<()> {
        match self {
            Io::Err(err) => err.write_fmt(fmt),
            Io::Out(out) => out.write_fmt(fmt),
        }
    }
}
