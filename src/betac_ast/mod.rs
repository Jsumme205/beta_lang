use crate::{
    betac_runner::Session,
    betac_tokenizer::token::Token,
    betac_util::{cell::RcCell, ptr::Ptr},
};
use core::fmt;
use std::{
    alloc::{self, Layout},
    fmt::Debug,
    sync::atomic::{AtomicU8, Ordering},
};

pub mod assign;
pub mod defun;
pub mod eof;
pub mod preproc;

use assign::Assign;
use defun::Defun;
use eof::Eof;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Metadata(u8);

#[derive(Debug, Clone, Copy)]
pub enum Vis {
    Priv,
    Pub,
    PubPack,
}

impl Metadata {
    pub const PUBLIC: u8 = 1 << 0;
    pub const STATIC: u8 = 1 << 1;
    pub const CONSTEXPR: u8 = 1 << 2;
    pub const CONSUMER: u8 = 1 << 3;
    pub const MUTABLE: u8 = 1 << 4;
    pub const PRIVATE: u8 = 1 << 5;

    pub const DUMMY: Self = Self(0);

    pub const fn new() -> Self {
        Self(0)
    }

    pub const fn has_flag_set(&self, flag: u8) -> bool {
        self.0 & flag != 0
    }

    pub fn from_atomic(v: &AtomicU8) -> Self {
        Self(v.load(Ordering::SeqCst))
    }

    pub const fn add(self, flag: u8) -> Self {
        Self(self.0 | flag)
    }

    pub const fn is_pack_public(&self) -> bool {
        self.0 & Self::PUBLIC != 0 && self.0 & Self::PRIVATE != 0
    }

    pub const fn is_public(&self) -> bool {
        self.0 & Self::PUBLIC != 0 && self.0 & Self::PRIVATE == 0
    }

    pub const fn is_private(&self) -> bool {
        self.0 & Self::PRIVATE != 0 && !self.is_public()
    }

    pub fn as_vis(&self) -> Vis {
        if self.is_public() {
            Vis::Pub
        } else if self.is_private() {
            Vis::Priv
        } else {
            Vis::PubPack
        }
    }
}

impl Debug for Metadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Metadata")
            .field("vis", &self.as_vis())
            .field("static", &self.has_flag_set(Self::STATIC))
            .field("constexpr", &self.has_flag_set(Self::CONSTEXPR))
            .field("consumer", &self.has_flag_set(Self::CONSUMER))
            .field("mutable", &self.has_flag_set(Self::MUTABLE))
            .finish()
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct Span {
    pub start_pos: u16,
    pub end_or_len_and_meta: SpanUnion,
}

impl Span {
    pub const DUMMY: Span = Span {
        start_pos: 0,
        end_or_len_and_meta: SpanUnion { end_pos: 0 },
    };

    pub fn from_token_slice(slice: &[Token]) -> Self {
        let first = slice.first().unwrap();
        let last = slice.last().unwrap();
        let mut me = Self::DUMMY;
        me.start_pos = first.start;
        me.end_or_len_and_meta.end_pos = last.start + last.len().unwrap_or(0);
        me
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Span")
            .field("start_pos", &self.start_pos)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy)]
pub union SpanUnion {
    pub end_pos: u16,
    pub len_and_meta: LenMeta,
}

impl PartialEq for SpanUnion {
    fn eq(&self, other: &Self) -> bool {
        unsafe { self.end_pos == other.end_pos }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LenMeta {
    pub len: u8,
    pub meta: Metadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AstKind {
    Assign,
    Defun,
    Eof,
    Pproc(preproc::PprocType),
    Whitespace,
    Newline,
}

pub trait AstType: fmt::Debug {
    fn name_ident(&self) -> Token;

    fn type_ident(&self) -> Token;

    fn args_span(&self) -> Option<Span>;

    fn kind(&self) -> AstKind;

    fn is_dummy(&self) -> bool;

    fn children_nodes(&self) -> Option<&AstList> {
        None
    }

    fn has_children_nodes(&self) -> bool {
        false
    }
}

impl dyn AstType {
    pub fn downcast_to_assign(&self) -> Assign {
        assert!(self.kind() == AstKind::Assign);
        unsafe { (self as *const _ as *const Assign).read() }
    }

    pub fn downcast_to_defun(&self) -> Defun {
        assert!(self.kind() == AstKind::Assign);
        unsafe { (self as *const _ as *const Defun).read() }
    }

    pub fn downcast_to_eof(&self) -> Eof {
        assert!(self.kind() == AstKind::Eof);
        unsafe { (self as *const _ as *const Eof).read() }
    }

    pub fn downcast_to_pproc(&self) -> preproc::PreprocessorStmt {
        assert!(matches!(self.kind(), AstKind::Pproc(_)));
        unsafe { (self as *const _ as *const preproc::PreprocessorStmt).read() }
    }

    pub fn downcast_to_whitespace(&self) -> eof::Whitespace {
        assert!(self.kind() == AstKind::Whitespace);
        unsafe { (self as *const _ as *const eof::Whitespace).read() }
    }
}

pub fn allocate_and_write<T>(value: T) -> *mut T {
    unsafe {
        let ptr = alloc::alloc(Layout::new::<T>()) as *mut T;
        if ptr.is_null() {
            alloc::handle_alloc_error(Layout::new::<T>());
        }
        ptr.write(value);
        ptr
    }
}

pub fn dummy() -> Ptr<dyn AstType> {
    unsafe { Ptr::from_raw(allocate_and_write(Assign::dummy())) }
}

macro_rules! ast_function {
    ($f:ident => $ty:ty) => {
        pub fn $f(x: $ty) -> $crate::betac_util::ptr::Ptr<dyn AstType> {
            unsafe { $crate::betac_util::ptr::Ptr::from_raw(allocate_and_write(x)) }
        }
    };
}

ast_function!(assign => Assign);
ast_function!(eof => Eof);
ast_function!(preprocessor => preproc::PreprocessorStmt);
ast_function!(defun => Defun);
ast_function!(whitespace => eof::Whitespace);
ast_function!(new_line => eof::Newline);

pub struct AstToken {
    prev: Option<RcCell<Self>>,
    next: Option<RcCell<Self>>,
    meta: Ptr<dyn AstType>,
}

impl Debug for AstToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        struct FullTree<'a>(&'a AstToken);
        struct NoTree<'a>(&'a AstToken);

        impl fmt::Debug for FullTree<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let mut builder = f.debug_struct("AstToken");
                builder.field("meta", &self.0.meta);
                if self.0.meta.has_children_nodes() {
                    builder
                        .field("children", self.0.meta.children_nodes().unwrap())
                        .finish_non_exhaustive()
                } else {
                    builder.finish_non_exhaustive()
                }
            }
        }

        impl fmt::Debug for NoTree<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct("AstToken")
                    .field("meta", &self.0.meta)
                    .finish_non_exhaustive()
            }
        }

        if Session::has_full_tree_backtrace_set() {
            FullTree(self).fmt(f)
        } else {
            NoTree(self).fmt(f)
        }
    }
}

impl AstToken {
    pub const fn new(meta: Ptr<dyn AstType>) -> Self {
        Self {
            prev: None,
            next: None,
            meta,
        }
    }

    pub fn dummy() -> Self {
        Self::new(dummy())
    }

    #[inline]
    pub fn set_next(&mut self, next: RcCell<Self>) {
        self.next = Some(next);
    }

    #[inline]
    pub fn set_prev(&mut self, prev: RcCell<Self>) {
        self.prev = Some(prev);
    }

    #[inline]
    pub fn get_next(&self) -> Option<RcCell<Self>> {
        self.next.clone()
    }

    #[inline]
    pub fn get_prev(&self) -> Option<RcCell<Self>> {
        self.prev.clone()
    }
}

pub struct NodeVisitor {
    current: RcCell<AstToken>,
}

impl Iterator for NodeVisitor {
    type Item = RcCell<AstToken>;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.current.read().get_next()?;
        self.current = node.clone();
        Some(node)
    }
}

// constantly, the tail's next will point to the head
// which means we only need to store the tail to implicitly store the head
pub struct AstList {
    tail: RcCell<AstToken>,
}

impl Debug for AstList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(self.visit()).finish()
    }
}

impl AstList {
    pub fn new() -> Self {
        let head = RcCell::new(AstToken::dummy());
        let tail = RcCell::new(AstToken::dummy());
        let mut head_b = head.write();
        let mut tail_b = tail.write();
        tail_b.set_next(head.clone());
        head_b.set_prev(tail.clone());
        drop(tail_b);
        Self { tail }
    }

    pub fn push(&mut self, node: AstToken) {
        let node = RcCell::new(node);
        let prev = self.tail.read().prev.clone();
        match prev {
            Some(prev) => {
                prev.write().set_next(node.clone());
                node.write().set_prev(prev);
                node.write().set_next(self.tail.clone());
            }
            None => {
                self.tail.write().set_prev(node.clone());
                node.write().set_next(self.tail.clone());
            }
        }
    }

    pub fn visit(&self) -> NodeVisitor {
        NodeVisitor {
            current: self.tail.read().get_next().unwrap(),
        }
    }
}
