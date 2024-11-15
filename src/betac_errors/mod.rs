use crate::{betac_ast::Span, betac_util::small_vec::SmallVec};
use std::fmt;
use std::sync::{Mutex, MutexGuard};

pub mod general_errors;
pub mod option;
pub mod preproc_errors;

pub struct Emitter {
    errors: SmallVec<Box<dyn Reportable>>,
}

pub static EMITTER: Mutex<Emitter> = Mutex::new(Emitter::new());

impl Emitter {
    pub const fn new() -> Self {
        Self {
            errors: SmallVec::new(),
        }
    }

    pub fn flush(&mut self, w: &mut dyn std::io::Write) -> std::io::Result<()> {
        for err in self.errors.drain() {
            let prefix = match err.level() {
                Level::Error => "ERROR",
                Level::Warning => "WARNING",
            };

            writeln!(w, "{prefix}: on {}:{}", err.line(), err.column())?;
            writeln!(w, "{}", err.message())?;
        }
        Ok(())
    }

    pub fn with<F, R>(f: F) -> R
    where
        F: FnOnce(MutexGuard<'_, Emitter>) -> R,
    {
        f(EMITTER.lock().unwrap())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SpanKind {
    Meta,
    NoMeta,
    Unset,
}

#[derive(Debug, Clone, Copy)]
pub enum Level {
    Warning,
    Error,
}

pub trait Reportable: Send + Sync + fmt::Debug {
    fn report(self);

    fn span(&self) -> Span;
    fn line(&self) -> u32;
    fn column(&self) -> u32;
    fn message(&self) -> &str;

    fn level(&self) -> Level;
}

macro_rules! impl_builder_for_reportable {
    ($ty:ty) => {
        impl $ty {
            pub const fn builder() -> Self {
                Self {
                    span: None,
                    line: 0,
                    column: 0,
                    message: None,
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

            pub fn span(mut self, span: Span, kind: SpanKind) -> Self {
                self.span = Some((span, kind));
                self
            }

            pub fn message(mut self, message: String) -> Self {
                self.message = Some(message);
                self
            }
        }
    };
}

use impl_builder_for_reportable as builder;
