use crate::{
    betac_ast::{self, AstToken, AstType},
    betac_tokenizer::token::{Token, TokenKind},
    betac_util::{ptr::Ptr, small_vec::SmallVec},
};

pub struct Parser<'a, I> {
    tokens: I,
    input: &'a str,
    idx: u32,
    last_token: Token,
}

impl<'a, I> Parser<'a, I>
where
    I: Iterator<Item = Token> + Clone + 'a,
{
    pub fn new(input: &'a str, iter: I) -> Self {
        Self {
            tokens: iter,
            input,
            idx: 0,
            last_token: Token::DUMMMY,
        }
    }

    fn reconstruct_from_start_len(&self, start: u32, len: u32) -> &str {
        println!("{start}..{end}", end = start + len);
        let end = start + len;
        &self.input[start as usize..end as usize]
    }

    fn reconstruct_from_token_slice(&self, tokens: &[Token]) -> &str {
        let first = tokens.first().unwrap();
        let last = tokens.last().unwrap();
        self.reconstruct_from_start_len(first.start, *last - *first)
    }

    fn reconstruct_from_token_pair(&self, token1: Token, token2: Token) -> &str {
        self.reconstruct_from_start_len(token1.start, token2 - token1)
    }

    fn eat_tokens_while(&mut self, mut pred: impl FnMut(Token) -> bool) {
        while let Some(token) = self.tokens.next() {
            if !pred(token) {
                return;
            }
        }
    }

    fn peek(&self) -> Option<Token> {
        self.tokens.clone().next()
    }

    fn collect_until(&mut self, mut pred: impl FnMut(Token) -> bool) -> SmallVec<Token> {
        let mut buf = SmallVec::new();
        while let Some(token) = self.tokens.next() {
            buf.push(token);
            if !pred(token) {
                break;
            }
        }
        buf
    }

    fn bump_token(&mut self) -> Option<Token> {
        let token = self.tokens.next()?;
        self.last_token = token;
        Some(token)
    }

    pub fn parse_next_expr(&mut self) -> AstToken {
        let inner: Ptr<dyn AstType> = match self.bump_token().unwrap_or(self.last_token) {
            token @ Token {
                kind: TokenKind::Eof,
                ..
            } => {
                let eof = betac_ast::eof::Eof::new(token);
                betac_ast::eof(eof)
            }
            Token {
                start,
                kind: TokenKind::Ident,
            } => {
                let next = self.peek().unwrap();
                let substr = self.reconstruct_from_start_len(start, next.start - start);
                println!("substr_87: {substr}");
                todo!()
            }
            Token {
                start,
                kind: TokenKind::At,
            } if self.peek().unwrap().kind == TokenKind::Ident => {
                // this is most likely a preprocessor, so we need to check the next token
                // is a ident, then check if its a valid preprocessor statement
            }
            _ => todo!(),
        };
        todo!()
    }

    fn assignment(&mut self) {}

    fn preprocessor(&mut self) {}

    fn defun(&mut self) {}

    fn handle_rhs(&mut self) {}
}
