use crate::betac_ast::Span;

use super::{builder, Emitter, Reportable, SpanKind, EMITTER};

#[derive(Debug)]
pub struct UnrecognizedPreprocMacro {
    span: Option<(Span, SpanKind)>,
    line: u32,
    column: u32,
    message: Option<String>,
}

builder!(UnrecognizedPreprocMacro);

impl Reportable for UnrecognizedPreprocMacro {
    fn line(&self) -> u32 {
        self.line
    }

    fn column(&self) -> u32 {
        self.column
    }

    fn span(&self) -> Span {
        self.span.unwrap().0
    }

    fn level(&self) -> super::Level {
        super::Level::Error
    }

    fn message(&self) -> &str {
        self.message.as_ref().unwrap()
    }

    fn report(self) {
        Emitter::with(|mut lock| lock.errors.push(Box::new(self)))
    }
}
