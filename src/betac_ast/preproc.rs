use crate::betac_tokenizer::token::Token;

use super::{AstType, Span};

#[derive(Debug, Clone, Copy)]
pub struct PreprocessorStmt {
    kind: PreprocKind,
}

impl PreprocessorStmt {
    pub const fn dummy() -> Self {
        Self {
            kind: PreprocKind::AtStart {
                start_id_name: Token::DUMMMY,
                len: 0,
            },
        }
    }

    pub const fn new(kind: PreprocKind) -> Self {
        Self { kind }
    }
}

impl AstType for PreprocessorStmt {
    fn kind(&self) -> super::AstKind {
        match self.kind {
            PreprocKind::AtStart { .. } => super::AstKind::Pproc(PprocType::Start),
            PreprocKind::AtMacro { .. } => super::AstKind::Pproc(PprocType::Macro),
            PreprocKind::AtDef { .. } => super::AstKind::Pproc(PprocType::Def),
            PreprocKind::AtEval { .. } => super::AstKind::Pproc(PprocType::Eval),
        }
    }

    fn is_dummy(&self) -> bool {
        false
    }

    fn args_span(&self) -> Option<Span> {
        None
    }

    fn name_ident(&self) -> Token {
        match self.kind {
            PreprocKind::AtDef { ident_name, .. } => ident_name,
            PreprocKind::AtStart { start_id_name, .. } => start_id_name,
            _ => panic!(),
        }
    }

    fn type_ident(&self) -> Token {
        panic!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PreprocKind {
    AtStart { start_id_name: Token, len: u16 },
    AtDef { ident_name: Token, len: u16 },
    AtEval { eval_span: Span },
    AtMacro { args_span: Span },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PprocType {
    Start,
    Def,
    Eval,
    Macro,
}
