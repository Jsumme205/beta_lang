use std::{
    fmt::{self, Debug},
    io,
    ops::Range,
};

//pub mod lock;
pub mod node;

pub mod session;
pub mod sso;
pub mod thin_vec;

pub use node::{BinOp, OperatorNode};
pub(crate) use sso::Yarn;

use crate::{
    betac_lexer::ast_types::{defun::Argument, Ty},
    betac_pp,
};

pub fn str_contains_any<const N: usize>(s: &str, p: [&str; N]) -> bool {
    for pat in p {
        if s.contains(pat) {
            return true;
        }
    }
    false
}

pub fn is_valid_ident(s: &str) -> bool {
    match s.chars().next() {
        Some(c) => c.is_alphabetic(),
        None => false,
    }
}

pub fn is_valid(y: &Yarn<'_>) -> bool {
    is_id_start(y[0]) && y.bytes().all(is_id_continue)
}

pub fn is_whitespace(c: u8) -> bool {
    matches!(
        c as char,
        '\u{0009}'
            | '\u{000A}'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{000D}'
            | '\u{0020}'
            | '\u{0085}'
            | '\u{200E}'
            | '\u{200F}'
            | '\u{2028}'
            | '\u{2029}'
    )
}

pub fn is_id_start(c: u8) -> bool {
    let c = c as char;
    c == '_' || unicode_xid::UnicodeXID::is_xid_start(c)
}

pub fn is_id_continue(c: u8) -> bool {
    let c = c as char;
    unicode_xid::UnicodeXID::is_xid_continue(c)
}

pub fn strip_one(s: &str) -> &str {
    &s[0..s.len() - 1]
}

pub trait OptionExt {
    type Element;
    fn try_catch<U>(self, t: impl FnOnce(Self::Element) -> U, c: impl FnOnce() -> U) -> U;
}

pub trait VecExt<T> {
    fn take(&mut self, index: usize) -> T;

    fn take_many(&mut self, index: Range<usize>) -> Vec<T>;
}

impl<T> VecExt<T> for Vec<T>
where
    T: Default,
{
    fn take(&mut self, index: usize) -> T {
        std::mem::take(&mut self[index])
    }

    fn take_many(&mut self, index: Range<usize>) -> Vec<T> {
        let mut buf = vec![];
        for i in index {
            buf.push(std::mem::take(&mut self[i]));
        }
        buf
    }
}

pub trait SplitVec {
    type Out;

    fn split_off(&self) -> Vec<Self::Out>;
}

impl<'a> SplitVec for Vec<Argument<'a>> {
    type Out = Ty;

    fn split_off(&self) -> Vec<Self::Out> {
        self.iter().map(|(_, ty)| *ty).collect::<Vec<_>>()
    }
}

pub trait CharVec {
    fn collect_string(self) -> String;
}

pub trait IntoYarn {
    fn collect(self) -> Yarn<'static>;
}

impl CharVec for Vec<char> {
    fn collect_string(self) -> String {
        self.into_iter().collect()
    }
}

impl CharVec for Vec<u8> {
    fn collect_string(self) -> String {
        unsafe { String::from_utf8_unchecked(self) }
    }
}

impl IntoYarn for Vec<u8> {
    fn collect(self) -> Yarn<'static> {
        self.into_iter().collect::<Yarn<'static>>()
    }
}

impl<T> OptionExt for Option<T> {
    type Element = T;

    fn try_catch<U>(self, t: impl FnOnce(Self::Element) -> U, c: impl FnOnce() -> U) -> U {
        match self {
            Some(v) => t(v),
            None => c(),
        }
    }
}

#[derive(Debug)]
pub enum CompileError {
    Eval(betac_pp::EvaluationError),
    Io(io::Error),

    Other,
}

impl From<io::Error> for CompileError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<betac_pp::EvaluationError> for CompileError {
    fn from(value: betac_pp::EvaluationError) -> Self {
        Self::Eval(value)
    }
}

pub type CompileResult<T> = Result<T, CompileError>;

pub fn debug_vec<T: fmt::Debug>(vec: &Vec<T>) {
    let enumerated = vec.iter().enumerate().collect::<Vec<_>>();
    println!("{enumerated:#?}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ignore = "not applicable for right now"]
    #[test]
    fn test_strip() {
        let s = "KEY:";
        assert!(strip_one(s) == "KEY")
    }

    #[test]
    fn test_yarn() {
        let y = crate::yarn!("hello, {name}", name = "jack");
        assert!(y == "hello, jack");
    }
}
