use crate::{
    betac_backend::WriteIr,
    betac_util::{session::Session, sso::OwnedYarn, Yarn},
    yarn,
};
use std::{fmt::Debug, io::Write, os::unix::io};

pub mod assignment;
pub mod context;
pub mod defun;

pub use assignment::AssignmentMeta;
use context::ContextKind;
use defun::{Argument, DefunMeta};

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub enum Token<'src> {
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
    Semi,
    #[default]
    Eof,
    // {
    LeftBrace,
    // }
    RightBrace,
    // [
    LeftBracket,
    // ]
    RightBracket,
    LeftParen,
    RightParen,
    Comma,
}

pub enum IdentOrLit<'src> {
    Ident(Yarn<'src>),
    Number(Yarn<'src>),
    Str(Yarn<'src>),
}

impl<'src> Token<'src> {
    pub fn is_sep(&self) -> bool {
        use Token::*;
        match self {
            Whitespace | Colon | Semi | Comma | RightParen => true,
            _ => false,
        }
    }

    pub fn is_end(&self) -> bool {
        use Token::*;
        match self {
            Eof | RightBrace | RightBracket | Semi => true,
            _ => false,
        }
    }

    pub fn number_or_ident(&self) -> Option<IdentOrLit<'src>> {
        match self {
            Token::Ident(id) => Some(IdentOrLit::Ident(id.clone())),
            Token::LitStr(lit) => Some(IdentOrLit::Str(lit.clone())),
            Token::Number(num) => Some(IdentOrLit::Number(num.clone())),
            _ => None,
        }
    }

    pub fn is_operator(&self) -> bool {
        use Token::*;
        match self {
            Plus | Minus => true,
            _ => false,
        }
    }

    pub fn as_binop(&self) -> Option<BinOp> {
        match self {
            Self::Plus => Some(BinOp::Plus),
            Self::Minus => Some(BinOp::Minus),
            _ => None,
        }
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
