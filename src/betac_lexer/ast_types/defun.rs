use std::rc::Rc;

use crate::{
    betac_lexer::Lexer,
    betac_util::{VecExt, Yarn},
};

use super::{Expr, Metadata, RawToken, Token, Ty};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DefunMeta(u8);

pub type Argument<'a> = (Yarn<'a>, Ty);

impl Metadata for DefunMeta {
    fn init() -> Self {
        Self(0)
    }

    fn add_flag(mut self, flag: u8) -> Self {
        self.0 |= flag;
        self
    }

    fn flag_set(&self, flag: u8) -> bool {
        self.0 & flag != 0
    }
}

impl<'a> Lexer<'a> {
    pub(in super::super) fn defun(&mut self, meta: DefunMeta) -> Option<Expr> {
        let mut tokens = self
            .expr_loop(RawToken::RightBrace)
            .into_iter()
            .filter(|token| !token.is_whitespace())
            .collect::<Vec<_>>();
        println!("tokens_33: {tokens:#?}");

        let (ident, _, idx) = Self::parse_front_part_of_function(&mut tokens)?;

        None
    }

    /// when we actually start to implement the generic system, we need the `Vec<Ty>` but for now, its
    /// always `None`
    fn parse_front_part_of_function(
        tokens: &mut Vec<Rc<Token>>,
    ) -> Option<(Yarn<'static>, Option<Vec<Ty>>, usize)> {
        let ident = tokens.take(1);

        ident.as_ident().map(|id| (id.clone(), None, 2))
    }

    fn parse_arguments(
        &mut self,
        tokens: &mut Vec<Rc<Token>>,
        idx: usize,
    ) -> Option<(Vec<Argument<'static>>, Vec<Rc<Token>>)> {
        // get context, add this function to it
        // self.session.package_context.get_global_context(self.session.)

        None
    }
}
