use crate::{
    betac_ast::{
        self,
        preproc::{PreprocKind, PreprocessorStmt},
        AstList, AstToken, AstType, Span,
    },
    betac_errors::{self, Reportable, SpanKind},
    betac_tokenizer::token::{Token, TokenKind},
    betac_util::{cell::RcCell, ptr::Ptr, small_vec::SmallVec},
};

pub mod preprocessor;

pub struct Parser<'a, I> {
    tokens: I,
    input: &'a str,
    last_token: Token,
    list: AstList,
}

impl<'a, I> Parser<'a, I>
where
    I: Iterator<Item = Token> + Clone + 'a,
{
    pub fn new(input: &'a str, iter: I) -> Self {
        Self {
            tokens: iter,
            input,
            last_token: Token::DUMMMY,
            list: AstList::new(),
        }
    }

    fn reconstruct_from_start_len(&self, start: u32, len: u32) -> &str {
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
                kind: TokenKind::At,
                ..
            } => {
                let next = self.peek().unwrap();
                // this is most likely a preprocessor, so we need to check the next token
                // is a ident, then check if its a valid preprocessor statement
                if next.kind == TokenKind::Ident {
                    let start = next.start;
                    let next = self.bump_token().unwrap();
                    let pproc = self.preprocessor(start, next.start);
                    betac_ast::preprocessor(pproc)
                } else {
                    betac_ast::dummy()
                }
            }
            _ => todo!(),
        };

        let token = RcCell::new(AstToken::new(inner));

        todo!()
    }

    fn assignment(&mut self) -> betac_ast::assign::Assign {
        todo!()
    }

    fn preprocessor(&mut self, start: u32, next_start: u32) -> PreprocessorStmt {
        let substr = self.reconstruct_from_start_len(start, next_start - start);
        println!("substr: {substr}");
        match substr {
            "start" => self.handle_start(),
            _ => {
                betac_errors::preproc_errors::UnrecognizedPreprocMacro::builder()
                    .message(format!("unrecognized preprocessor in input: {substr}"))
                    .span(Span::DUMMY, SpanKind::NoMeta)
                    .report();
                PreprocessorStmt::dummy()
            }
        }
    }

    fn defun(&mut self) {}

    fn handle_rhs(&mut self) {}

    pub fn complete(self) -> AstList {
        todo!()
    }
}
