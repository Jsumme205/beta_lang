use super::{AstKind, AstType, LenMeta, Metadata, Span, SpanUnion};
use crate::betac_tokenizer::token::Token;
use std::fmt::Debug;

#[derive(Clone, Copy, PartialEq)]
pub struct Assign {
    id_name: Token,
    id_type: Token,
    span: Span,
}

impl Assign {
    pub const fn dummy() -> Self {
        Self {
            id_name: Token::DUMMMY,
            id_type: Token::DUMMMY,
            span: Span::DUMMY,
        }
    }

    pub const fn new(
        id_name: Token,
        id_type: Token,
        meta: Metadata,
        start_pos: u16,
        len: u8,
    ) -> Self {
        Self {
            id_name,
            id_type,
            span: Span {
                start_pos,
                end_or_len_and_meta: SpanUnion {
                    len_and_meta: LenMeta { len, meta },
                },
            },
        }
    }
}

impl Debug for Assign {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if *self == Self::dummy() {
            f.write_str("<DUMMY>")
        } else {
            f.debug_struct("Assign")
                .field("id_name", &self.id_name)
                .field("id_type", &self.id_type)
                .finish()
        }
    }
}

impl AstType for Assign {
    fn args_span(&self) -> Option<Span> {
        None
    }

    fn name_ident(&self) -> Token {
        self.id_name
    }

    fn type_ident(&self) -> Token {
        self.id_type
    }

    fn kind(&self) -> AstKind {
        AstKind::Assign
    }

    fn is_dummy(&self) -> bool {
        *self == Self::dummy()
    }
}
