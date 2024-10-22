use std::rc::Rc;

use crate::{
    betac_lexer::{ast_types::context::SymbolKind, Lexer},
    betac_util::{VecExt, Yarn},
};

use super::{context::Context, Expr, Metadata, RawToken, Token, Ty};

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
    pub(in super::super) fn defun(
        &mut self,
        meta: DefunMeta,
        ctx: &mut dyn Context<'_>,
    ) -> Option<Expr> {
        let mut tokens = self
            .expr_loop(RawToken::RightBrace)
            .into_iter()
            .filter(|token| !token.is_whitespace())
            .collect::<Vec<_>>();

        let (ident, _, idx) = Self::parse_front_part_of_function(&mut tokens)?;
        let (args, rest) = self.parse_arguments(tokens, idx)?;

        let start = rest
            .iter()
            .position(|token| *token.as_raw() == RawToken::Assign)?;
        let (ty, exprs) = self.parse_back_and_body(rest, start)?;

        Some(Expr::Defun {
            meta,
            args,
            expressions: exprs,
            return_ty: ty,
            ident,
        })
    }

    /// when we actually start to implement the generic system, we need the `Vec<Ty>` but for now, its
    /// always `None`
    fn parse_front_part_of_function(
        tokens: &mut Vec<Rc<Token>>,
    ) -> Option<(Yarn<'static>, Option<Vec<Ty>>, usize)> {
        let ident = tokens.take(1);

        ident.as_ident().map(|id| (id.clone(), None, 3))
    }

    fn parse_arguments(
        &mut self,
        mut tokens: Vec<Rc<Token>>,
        idx: usize,
    ) -> Option<(Vec<Argument<'static>>, Vec<Rc<Token>>)> {
        // get context, add this function to it
        // self.session.package_context.get_global_context(self.session.)

        let mut tokens = tokens
            .into_iter()
            .filter(|token| *token.as_raw() != RawToken::Comma)
            .collect::<Vec<_>>();

        let mut stop_idx = idx;
        let mut paren = 1;
        while paren != 0 && stop_idx < tokens.len() {
            match tokens[stop_idx].as_raw() {
                RawToken::RightParen => {
                    stop_idx += 1;
                    paren -= 1;
                    if paren == 0 {
                        break;
                    }
                }
                RawToken::LeftParen => {
                    stop_idx += 1;
                    paren += 1
                }
                RawToken::Eof => {
                    // emit here
                    return None;
                }
                _ => {
                    stop_idx += 1;
                    continue;
                }
            }
        }

        let args = tokens
            .drain(idx..stop_idx)
            .array_chunks::<3>()
            .map(|arg| {
                let ident = arg[0].as_ident().map(|id| id.clone()).unwrap_or_else(|| {
                    // emit
                    Yarn::empty()
                });
                if *arg[1].as_raw() != RawToken::Colon {
                    // emit
                }
                let ty = arg[2].as_ident().map(|id| id.clone()).unwrap_or_else(|| {
                    // emit
                    Yarn::empty()
                });
                let ty = Ty::try_get(ty, &self.session).unwrap_or_default();
                (ident, ty)
            })
            .collect::<Vec<_>>();

        Some((args, tokens))
    }

    fn parse_back_and_body(
        &mut self,
        mut rest: Vec<Rc<Token>>,
        start_idx: usize,
    ) -> Option<(Ty, Vec<Expr>)> {
    }
}
