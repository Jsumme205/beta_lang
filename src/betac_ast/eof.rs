use super::AstKind;
use super::AstType;
use crate::betac_tokenizer::token::Token;
use std::fmt::Debug;

macro_rules! impls_for_dummy {
    ($ty:ident, $kind:expr) => {
        impl $ty {
            pub const fn new(inner: Token) -> Self {
                Self { inner }
            }
        }

        impl Debug for $ty {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                if self.inner == Token::DUMMMY {
                    f.write_str("<DUMMY>")
                } else {
                    f.debug_struct(stringify!($ty))
                        .field("inner", &self.inner)
                        .finish()
                }
            }
        }

        impl AstType for $ty {
            fn kind(&self) -> AstKind {
                $kind
            }

            fn is_dummy(&self) -> bool {
                self.inner == Token::DUMMMY
            }

            fn args_span(&self) -> Option<super::Span> {
                None
            }

            fn name_ident(&self) -> Token {
                self.inner
            }

            fn type_ident(&self) -> Token {
                self.inner
            }
        }
    };
}

#[derive(Clone, Copy)]
pub struct Newline {
    inner: Token,
}

#[derive(Clone, Copy)]
pub struct Whitespace {
    inner: Token,
}

#[derive(Clone, Copy)]
pub struct Eof {
    inner: Token,
}

impls_for_dummy!(Eof, AstKind::Eof);
impls_for_dummy!(Newline, AstKind::Newline);
impls_for_dummy!(Whitespace, AstKind::Whitespace);
