use super::super::Lexer;
use super::{Expr, Metadata, Token};

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
    pub(in super::super) fn assignment(&mut self, meta: AssignmentMeta) -> Option<Expr<'a>> {
        let mut current_tokens = vec![];
        let mut brace_count = 0;
        let mut brace_entered = false;
        let mut paren_count = 0;
        let mut paren_entered = false;
        // collect the expression, this is kind of why we're using `Rc` everywhere, for cheap cloning
        while *self.currently_evaluated_token != Token::Semi
            && brace_count == 0
            && brace_entered
            && paren_count == 0
            && paren_entered
        {
            match &*self.currently_evaluated_token {
                Token::Eof => break,
                Token::LeftBrace => {
                    if !brace_entered {
                        brace_entered = true;
                    }
                    brace_count += 1;
                }
                Token::RightBrace => {
                    brace_count -= 1;
                }
                Token::LeftParen => {
                    if !paren_entered {
                        paren_entered = true;
                    }
                    paren_count += 1;
                }
                Token::RightParen => {
                    brace_count -= 1;
                }
                _ => {}
            }
            current_tokens.push(self.currently_evaluated_token.clone());
            self.advance();
        }

        println!("tokens_64_assignment.rs: {current_tokens:#?}");

        None
    }
}
