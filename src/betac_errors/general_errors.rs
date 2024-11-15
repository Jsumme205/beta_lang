use crate::betac_ast::Span;

use super::SpanKind;

#[derive(Debug)]
pub struct MissingIdent {
    span: Option<(Span, SpanKind)>,
    line: u32,
    column: u32,
    message: Option<String>,
}

super::builder!(MissingIdent);

impl super::Reportable for MissingIdent {
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

    fn report(self) {
        super::EMITTER.lock().unwrap().errors.push(Box::new(self));
    }

    fn message(&self) -> &str {
        self.message.as_deref().unwrap()
    }
}

#[derive(Debug)]
pub struct UnexpectedTokenInInput {
    span: Option<(Span, SpanKind)>,
    line: u32,
    column: u32,
    message: Option<String>,
}

super::builder!(UnexpectedTokenInInput);

impl super::Reportable for UnexpectedTokenInInput {
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

    fn report(self) {
        super::EMITTER.lock().unwrap().errors.push(Box::new(self));
    }

    fn message(&self) -> &str {
        self.message.as_deref().unwrap()
    }
}
