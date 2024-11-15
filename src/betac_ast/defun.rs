use super::{AstKind, AstList, AstType, LenMeta, Metadata, Span, SpanUnion};
use crate::betac_tokenizer::token::Token;
use std::{fmt::Debug, sync::atomic::AtomicU8};

pub struct Defun {
    id_name: u16,
    id_name_end: u16,
    id_ret: u16,
    args_start: u16,
    args_len: u8,
    meta: Metadata,
    requires_span: Span,
    children: AstList,
}

#[derive(Clone, Copy, PartialEq)]
pub struct DefunInner {
    id_name: u16,
    id_ret: u16,
    args_start: u16,
    args_len: u8,
    metadata: Metadata,
    requires_span: Span,
}

impl Debug for Defun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_dummy() {
            f.write_str("<DUMMY>")
        } else {
            // TODO
            struct NoTree<'a>(&'a Defun);
            struct FullTree<'a>(&'a Defun);

            f.debug_struct("Defun")
                .field("id_name", &self.id_name)
                .field("id_name_end", &self.id_name_end)
                .field("id_ret", &self.id_ret)
                .field(
                    "args_span",
                    &(self.args_start..self.args_start + self.args_len as u16),
                )
                .field("metadata", &self.meta)
                .field("requires_span", &self.requires_span)
                .finish_non_exhaustive()
        }
    }
}

impl DefunInner {
    pub fn dummy() -> Self {
        Self {
            id_name: 0,
            id_ret: 0,
            args_start: 0,
            args_len: 0,
            metadata: Metadata::DUMMY,
            requires_span: Span::DUMMY,
        }
    }
}

impl Defun {
    fn as_inner(&self) -> DefunInner {
        let inner = DefunInner {
            id_ret: self.id_ret,
            id_name: self.id_name,
            args_start: self.args_start,
            args_len: self.args_len,
            metadata: self.meta,
            requires_span: self.requires_span,
        };
        inner
    }

    pub fn new(
        id_name: u16,
        id_name_end: u16,
        id_ret: u16,
        args_start: u16,
        args_len: u8,
        meta: &AtomicU8,
        requires_span: Span,
        children: AstList,
    ) -> Self {
        Self {
            id_name: id_name as u16,
            id_name_end,
            id_ret,
            args_start,
            args_len,
            meta: Metadata::from_atomic(meta),
            requires_span,
            children,
        }
    }
}

impl AstType for Defun {
    fn args_span(&self) -> Option<Span> {
        Some(Span {
            start_pos: self.args_start,
            end_or_len_and_meta: SpanUnion {
                len_and_meta: LenMeta {
                    len: self.args_len,
                    meta: self.meta,
                },
            },
        })
    }

    fn name_ident(&self) -> Token {
        Token {
            start: self.id_name,
            kind: crate::betac_tokenizer::token::TokenKind::Ident,
        }
    }

    fn type_ident(&self) -> Token {
        Token {
            start: self.id_ret,
            kind: crate::betac_tokenizer::token::TokenKind::Ident,
        }
    }

    fn kind(&self) -> AstKind {
        AstKind::Assign
    }

    fn is_dummy(&self) -> bool {
        self.as_inner() == DefunInner::dummy()
    }

    fn children_nodes(&self) -> Option<&AstList> {
        Some(&self.children)
    }

    fn has_children_nodes(&self) -> bool {
        true
    }
}
