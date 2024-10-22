use super::super::Lexer;
use super::context::{Context, SymbolKind};
use super::{Expr, Metadata, RawToken, Token};
use crate::betac_errors::{BetaError, Emitter, ErrorBuilder, Level};
use crate::betac_lexer::ast_types::{Ty, MUTABLE};
use crate::betac_util::Yarn;
use std::rc::Rc;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssignmentMeta(u8);

impl Metadata for AssignmentMeta {
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
    // let x: Int64 => 0;
    pub(in super::super) fn assignment(
        &mut self,
        mut meta: AssignmentMeta,
        ctx: &mut dyn Context<'_>,
    ) -> Option<Expr> {
        let mut current_tokens = vec![];
        let mut brace_count = 0;
        let mut brace_entered = false;
        let mut paren_count = 0;
        let mut paren_entered = false;
        // collect the expression, this is kind of why we're using `Rc` everywhere, for cheap cloning
        while *self.currently_evaluated_token != RawToken::Semi && brace_count == 0
            || brace_entered && paren_count == 0
            || paren_entered
        {
            match &self.currently_evaluated_token.inner {
                RawToken::Eof => break,
                RawToken::LeftBrace => {
                    if !brace_entered {
                        brace_entered = true;
                    }
                    brace_count += 1;
                }
                RawToken::RightBrace => {
                    brace_count -= 1;
                }
                RawToken::LeftParen => {
                    if !paren_entered {
                        paren_entered = true;
                    }
                    paren_count += 1;
                }
                RawToken::RightParen => {
                    brace_count -= 1;
                }
                //RawToken::Semi => {}
                _ => {}
            }
            current_tokens.push(self.currently_evaluated_token.clone());
            self.advance();
        }

        let current_tokens = current_tokens
            .into_iter()
            .filter(|token| !token.is_whitespace())
            .collect::<Vec<_>>();

        if !current_tokens[0].as_ident().is_some_and(|id| id == "let") {
            self.emit()
                .message(format!(
                    "expected keyword `let`, found: {:#?}",
                    current_tokens[0]
                ))
                .token(current_tokens[0].clone())
                .finish_and_report();
        }

        let mut idx_off = 0;
        if current_tokens[1].as_ident().is_some_and(|id| id == "mut") {
            meta = meta.add_flag(MUTABLE);
            idx_off = 1;
        }

        let ident = if idx_off == 0 {
            let current = current_tokens[1].clone();
            match current.as_ident() {
                Some(id) => id.clone(),
                None => {
                    println!("error_93");
                    self.emit()
                        .column(current.column as usize)
                        .line(current.line as usize)
                        .message(format!("expected `ident`, found {current:#?}"))
                        .token(current)
                        .finish_and_report();
                    return None;
                }
            }
        } else {
            let current = current_tokens[2].clone();
            println!("current: {current:#?}");
            match current.as_ident() {
                Some(id) => id.clone(),
                None => {
                    println!("error_107");
                    self.emit()
                        .column(current.column as usize)
                        .line(current.line as usize)
                        .message(format!(
                            "expected `ident`, found {current:#?}",
                            current = current.inner
                        ))
                        .token(current)
                        .finish_and_report();
                    return None;
                }
            }
        };

        if *current_tokens[2 + idx_off].as_raw() != RawToken::Colon {
            let current = current_tokens[2 + idx_off].clone();
            self.emit()
                .line(current.line as usize)
                .column(current.column as usize)
                .message(format!("expected colon, found: {:#?}", current.as_raw()))
                .token(current)
                .finish_and_report();
        }

        let ty = if let Some(id) = current_tokens[3 + idx_off].as_ident() {
            Ty::try_get(id.clone(), &self.session)?
        } else {
            let current = current_tokens[3 + idx_off].clone();
            println!("error_135");
            self.emit()
                .column(current.column as usize)
                .line(current.line as usize)
                .level(Level::HardError)
                .message(format!(
                    "expected `Ty`, found: {current:#?}",
                    current = current.inner
                ))
                .token(current)
                .finish_and_report();
            return None;
        };

        let value = if let Some(id) = current_tokens[5 + idx_off].as_number() {
            id.clone()
        } else {
            let current = current_tokens[5 + idx_off].clone();
            println!("error_152");
            self.emit()
                .column(current.column as usize)
                .line(current.line as usize)
                .level(Level::HardError)
                .message(format!(
                    "expected `Value`, found: {current:#?}",
                    current = current.inner
                ))
                .token(current)
                .finish_and_report();
            return None;
        };

        ctx.enter_symbol_into_scope(ident.clone(), SymbolKind::Assignment(ty, meta.to_vis()));

        Some(Expr::Assignment {
            ident,
            ty,
            value: Box::new(Expr::Literal(value)),
            meta,
        })
    }
}
