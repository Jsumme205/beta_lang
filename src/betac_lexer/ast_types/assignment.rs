use super::super::Lexer;
use super::{Expr, Metadata, RawToken, Token};
use crate::betac_errors::{BetaError, Emitter, ErrorBuilder};
use crate::betac_lexer::ast_types::MUTABLE;
use crate::betac_util::Yarn;

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
    pub(in super::super) fn assignment(&mut self, mut meta: AssignmentMeta) -> Option<Expr<'a>> {
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
            println!("got to here_38");
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
            println!("current_60: {:#?}", self.currently_evaluated_token);
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
                .token(current_tokens[0])
                .finish_and_report();
        }

        let mut ident = Yarn::empty();
        if current_tokens[1].as_ident().is_some_and(|id| id == "mut") {
            meta = meta.add_flag(MUTABLE);
        } else {
            match current_tokens[1].as_ident() {
                None => {
                    self.emit()
                        .message(format!(
                            "exppected ident, found {:#?}",
                            current_tokens[1].as_raw()
                        ))
                        .token(current_tokens[1])
                        .finish_and_report();
                }
                Some(id) => ident = id.clone(),
            }
        }

        if *current_tokens[2].as_raw() != RawToken::Colon {
            let current = current_tokens[2].clone();
            self.emit()
                .line(current.line as usize)
                .column(current.column as usize)
                .message(format!("expected colon, found: {:#?}", current.as_raw()))
                .token(current)
                .finish_and_report();
        }

        println!("tokens_64_assignment.rs: {current_tokens:#?}");

        Some(Expr::Assignment {
            ident,
            ty: (),
            value: (),
            meta,
        })
    }
}
