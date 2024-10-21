use crate::{
    betac_backend::WriteIr,
    betac_packer::pack::Vis,
    betac_util::{session::Session, sso::OwnedYarn, Yarn},
    yarn,
};
use std::{
    fmt::Debug,
    io::Write,
    os::{fd::RawFd, unix::io},
};

pub mod assignment;
pub mod context;
pub mod defun;
pub mod imports;

pub use assignment::AssignmentMeta;
use context::ContextKind;
use defun::{Argument, DefunMeta};
use imports::ImportMeta;

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub enum RawToken<'src> {
    Plus,
    Minus,
    Assign,
    Ident(Yarn<'src>),
    Number(Yarn<'src>),
    LitStr(Yarn<'src>),
    OneEq(u8),
    EqEq([u8; 2]),
    Whitespace,
    Colon,
    Path,
    Semi,
    #[default]
    Eof,
    /// {
    LeftBrace,
    /// }
    RightBrace,
    /// [
    LeftBracket,
    /// ]
    RightBracket,
    LeftParen,
    RightParen,
    Comma,
    NewLine,
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct Token<'src> {
    start: u32,
    end: u32,
    inner: RawToken<'src>,
}

impl<'src> Token<'src> {
    pub fn new(raw: RawToken<'src>, start: u32, end: u32) -> Self {
        Self {
            start,
            end,
            inner: raw,
        }
    }

    pub fn as_raw(&self) -> &RawToken<'src> {
        &self.inner
    }

    pub fn is_ident(&self) -> bool {
        match &self.inner {
            RawToken::Ident(_) => true,
            _ => false,
        }
    }

    pub fn is_whitespace(&self) -> bool {
        match &self.inner {
            RawToken::Whitespace => true,
            _ => false,
        }
    }

    pub fn as_span(&self) -> (usize, usize) {
        (self.start as usize, self.end as usize)
    }
}

pub enum IdentOrLit<'src> {
    Ident(Yarn<'src>),
    Number(Yarn<'src>),
    Str(Yarn<'src>),
}

impl<'src> Token<'src> {
    pub fn is_sep(&self) -> bool {
        use RawToken::*;
        match self.inner {
            Whitespace | Colon | Semi | Comma | RightParen => true,
            _ => false,
        }
    }

    pub fn is_end(&self) -> bool {
        use RawToken::*;
        match self.inner {
            Eof | RightBrace | RightBracket | Semi => true,
            _ => false,
        }
    }

    pub fn number_or_ident(&self) -> Option<IdentOrLit<'src>> {
        match &self.inner {
            RawToken::Ident(id) => Some(IdentOrLit::Ident(id.clone())),
            RawToken::LitStr(lit) => Some(IdentOrLit::Str(lit.clone())),
            RawToken::Number(num) => Some(IdentOrLit::Number(num.clone())),
            _ => None,
        }
    }

    pub fn is_operator(&self) -> bool {
        use RawToken::*;
        match self.inner {
            Plus | Minus => true,
            _ => false,
        }
    }

    pub fn as_ident(&self) -> Option<&Yarn<'src>> {
        match &self.inner {
            RawToken::Ident(id) => Some(id),
            _ => None,
        }
    }

    pub fn as_binop(&self) -> Option<BinOp> {
        match &self.inner {
            RawToken::Plus => Some(BinOp::Plus),
            RawToken::Minus => Some(BinOp::Minus),
            _ => None,
        }
    }

    pub fn ident_is_expr_start(&self) -> bool {
        match &self.inner {
            RawToken::Ident(id) => match id.as_str() {
                "import" | "let" | "obj" | "comp" | "defun" | "pack" | "constexpr" => true,
                _ => false,
            },
            _ => false,
        }
    }

    pub fn ident_is_modifier(&self) -> bool {
        match &self.inner {
            RawToken::Ident(id) => match id.as_str() {
                "constexpr" | "mut" | "static" | "pub" => true,
                _ => false,
            },
            _ => false,
        }
    }
    pub fn ident_is_keyword(&self) -> bool {
        match &self.inner {
            RawToken::Ident(id) => match id.as_str() {
                "import" | "let" | "obj" | "comp" | "defun" | "pub" => true,
                _ => false,
            },
            _ => false,
        }
    }
}

impl PartialEq<RawToken<'_>> for Token<'_> {
    fn eq(&self, other: &RawToken) -> bool {
        self.inner == *other
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ty(usize);

impl Ty {
    pub const OFFSET_FROM_BUILTIN: usize = 10;

    pub const TY_INT8: Self = Self(0);
    pub const TY_INT16: Self = Self(1);
    pub const TY_INT32: Self = Self(2);
    pub const TY_INT64: Self = Self(3);
    pub const TY_UINT8: Self = Self(4);
    pub const TY_UINT16: Self = Self(5);
    pub const TY_UINT32: Self = Self(6);
    pub const TY_UINT64: Self = Self(7);
    pub const TY_STR: Self = Self(8);
    pub const TY_BOOL: Self = Self(9);

    pub fn gen_from_session(session: &Session) -> Self {
        Self(session.get_next_type_id())
    }

    pub fn get(self, session: &Session) -> &OwnedYarn {
        session
            .types_hashmap
            .get(&self.0)
            .expect("type not found in scope")
    }

    pub fn register(self, yarn: OwnedYarn, session: &mut Session) -> Self {
        if !session.types_hashmap.contains_key(&self.0) {
            session.register(self.0, yarn);
            return self;
        }
        panic!("type already found");
    }

    pub fn try_get(yarn: OwnedYarn, _session: &Session) -> Option<Self> {
        match yarn.as_str() {
            "Int8" => Some(Ty::TY_INT8),
            "Int16" => Some(Ty::TY_INT16),
            "Int32" => Some(Ty::TY_INT32),
            "Int64" => Some(Ty::TY_INT64),
            "Uint8" => Some(Ty::TY_UINT8),
            "Uint16" => Some(Ty::TY_UINT16),
            "Uint32" => Some(Ty::TY_UINT32),
            "Uint64" => Some(Ty::TY_UINT64),
            "Str" => Some(Ty::TY_STR),
            "Bool" => Some(Ty::TY_BOOL),
            _ => None,
        }
    }

    pub fn to_llvm_name(self) -> &'static str {
        match self {
            Ty::TY_BOOL => "i1 ",
            Ty::TY_INT16 => "i16 ",
            Ty::TY_INT32 => "i32 ",
            Ty::TY_INT64 => "i64 ",
            Ty::TY_UINT32 => "u32 ",
            _ => todo!(),
        }
    }

    pub fn can_be_implicitly_converted(&self, other: Self) -> bool {
        if *self == other {
            return true;
        }
        match (*self, other) {
            (Ty::TY_INT16, Ty::TY_INT32)
            | (Ty::TY_INT32, Ty::TY_INT64)
            | (Ty::TY_UINT16, Ty::TY_UINT32)
            | (Ty::TY_UINT32, Ty::TY_UINT64) => true,
            _ => false,
        }
    }

    pub fn is_number(&self) -> bool {
        match *self {
            Ty::TY_INT8
            | Ty::TY_INT16
            | Ty::TY_INT32
            | Ty::TY_INT64
            | Ty::TY_UINT8
            | Ty::TY_UINT16
            | Ty::TY_UINT32
            | Ty::TY_UINT64 => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
pub enum Expr<'src> {
    Assignment {
        ident: Yarn<'src>,
        ty: Ty,
        value: Box<Self>,
        meta: AssignmentMeta,
    },
    Binary {
        lhs: Box<Self>,
        op: BinOp,
        rhs: Box<Self>,
        ty: Ty,
        context_kind: ContextKind,
    },
    Literal(Yarn<'src>),
    LitOrIdent(Yarn<'src>, Ty),
    Copy(Yarn<'src>),
    Call {
        ident: Yarn<'src>,
        args: Vec<Yarn<'src>>,
        ret_ty: Ty,
    },
    Defun {
        meta: DefunMeta,
        args: Vec<Argument<'src>>,
        expressions: Vec<Expr<'src>>,
        return_ty: Ty,
        ident: OwnedYarn,
    },
    Import {
        meta: ImportMeta,
        root: Yarn<'src>,
        rest: Vec<Yarn<'src>>,
    },
    Eof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BinOp {
    Plus,
    Minus,
    Mul,
    Div,
    EqEq,
    Ne,
    Or,
    And,
}

impl WriteIr for BinOp {
    fn lower(
        self,
        writer: &mut crate::betac_backend::IrCodegen,
    ) -> Result<(), crate::betac_backend::BackendError> {
        match self {
            Self::Plus => writer.write_str("add ")?,
            Self::Minus => writer.write_str("sub ")?,
            _ => todo!(),
        };
        Ok(())
    }
}

pub const STATIC: u8 = 1 << 1;
pub const CONSTEXPR: u8 = 1 << 2;
pub const MUTABLE: u8 = 1 << 3;
pub const PUBLIC: u8 = 1 << 4;

pub trait Metadata: Copy {
    fn init() -> Self;
    fn add_flag(self, flag: u8) -> Self;

    fn is_public(&self) -> bool {
        self.flag_set(PUBLIC)
    }

    fn is_constexpr(&self) -> bool {
        self.flag_set(CONSTEXPR)
    }

    fn is_mutable(&self) -> bool {
        self.flag_set(MUTABLE)
    }

    fn is_static(&self) -> bool {
        self.flag_set(STATIC)
    }

    fn flag_set(&self, flag: u8) -> bool;

    fn to_vis(self) -> Vis {
        if self.is_public() {
            Vis::Public
        } else {
            Vis::Private
        }
    }
}

#[derive(Clone, Copy)]
pub union AnyMetadata {
    defun: DefunMeta,
    assignment: AssignmentMeta,
    import: ImportMeta,
}

impl AnyMetadata {
    pub fn to_defun(self) -> DefunMeta {
        unsafe { self.defun }
    }

    pub fn to_assignment(self) -> AssignmentMeta {
        unsafe { self.assignment }
    }

    pub fn to_import(self) -> ImportMeta {
        unsafe { self.import }
    }
}

impl Metadata for AnyMetadata {
    fn init() -> Self {
        Self {
            defun: DefunMeta::init(),
        }
    }

    fn add_flag(self, flag: u8) -> Self {
        let this = unsafe { self.defun };
        Self {
            defun: this.add_flag(flag),
        }
    }

    fn flag_set(&self, flag: u8) -> bool {
        let this = unsafe { self.defun };
        this.flag_set(flag)
    }
}
