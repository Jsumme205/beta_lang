use std::sync::atomic::{AtomicU8, Ordering};

use crate::{
    betac_ast::{
        self, preproc::PreprocessorStmt, AstKind, AstList, AstToken, AstType, Metadata, Span,
    },
    betac_errors::{self, Reportable, SpanKind},
    betac_tokenizer::token::{Token, TokenKind},
    betac_util::{ptr::Ptr, small_vec::svec, small_vec::SmallVec},
};

pub mod assign;
pub mod defun;
pub mod preprocessor;

pub struct ParseInner<'a, I> {
    tokens: Option<I>,
    input: &'a str,
    last_token: Token,
}

impl<'a, I> ParseInner<'a, I>
where
    I: Iterator<Item = Token> + Clone + 'a,
{
    pub fn new(input: &'a str, iter: I) -> Self {
        Self {
            tokens: Some(iter),
            input,
            last_token: Token::DUMMMY,
        }
    }

    fn reconstruct_from_start_len(&self, start: u16, len: u8) -> &str {
        let end = start + len as u16;
        &self.input[start as usize..end as usize]
    }

    fn reconstruct_from_token_slice(&self, tokens: &[Token]) -> &str {
        let first = tokens.first().unwrap();
        let last = tokens.last().unwrap();
        self.reconstruct_from_start_len(first.start, (*last - *first) as u8)
    }

    fn reconstruct_from_token_pair(&self, token1: Token, token2: Token) -> &str {
        self.reconstruct_from_start_len(token1.start, (token2 - token1) as u8)
    }

    fn reconstruct_from_last_token_and_peek(&self, last: Token) -> &str {
        self.reconstruct_from_start_len(last.start, (self.peek().unwrap() - last) as u8)
    }

    fn reconstruct_from_span(&self, span: Span, kind: SpanKind) -> &str {
        match kind {
            SpanKind::Meta => self.reconstruct_from_start_len(span.start_pos, unsafe {
                span.end_or_len_and_meta.len_and_meta.len as u8
            }),
            SpanKind::NoMeta => self.reconstruct_from_start_len(span.start_pos, unsafe {
                (span.end_or_len_and_meta.end_pos - span.start_pos) as u8
            }),
            _ => panic!(),
        }
    }

    fn eat_tokens_while(&mut self, mut pred: impl FnMut(Token) -> bool) {
        while let Some(token) = self.bump_token() {
            if !pred(token) {
                return;
            }
        }
    }

    fn peek(&self) -> Option<Token> {
        self.tokens.clone().as_mut().unwrap().next()
    }

    fn collect_until(&mut self, mut pred: impl FnMut(Token) -> bool) -> SmallVec<Token> {
        let mut buf = SmallVec::new();
        while let Some(token) = self.bump_token() {
            buf.push(token);
            if !pred(token) {
                break;
            }
        }
        buf
    }

    fn bump_until_next(&mut self, mut pred: impl FnMut(Token) -> bool) {
        while let Some(token) = self.peek() {
            if !pred(token) {
                break;
            }
            self.bump_token();
        }
    }

    #[must_use = "if you don't need the result use `Parser::bump_until_next`"]
    fn collect_until_next(&mut self, mut pred: impl FnMut(Token) -> bool) -> SmallVec<Token> {
        let mut buf = svec![];
        while let Some(token) = self.peek() {
            buf.push(token);
            if !pred(token) {
                break;
            }
            self.bump_token();
        }
        buf
    }

    fn bump_token(&mut self) -> Option<Token> {
        let token = self.tokens.as_mut().unwrap().next()?;
        self.last_token = token;
        Some(token)
    }

    fn bump_whitespace(&mut self) {
        self.eat_tokens_while(|token| token.kind == TokenKind::Whitespace);
    }

    /// returns whether this was the last token or not. \n
    /// `true` indictates that this is the last token \n
    /// `false` indicates that there are more tokens to parse \n
    /// TODO: set up some sort of arena to allocate short-lived objects \n
    /// (AKA Whitespace tokens, Newline tokens, etc.)
    fn parse_next_expr(&mut self, list: &mut AstList) -> bool {
        let (inner, finished): (Ptr<dyn AstType>, bool) =
            match self.bump_token().unwrap_or(self.last_token) {
                token @ Token {
                    kind: TokenKind::Eof,
                    ..
                } => {
                    let eof = betac_ast::eof::Eof::new(token);
                    (betac_ast::eof(eof), true)
                }
                Token {
                    start,
                    kind: TokenKind::Ident,
                } => {
                    // this runs a loop sort of like this:
                    // get token
                    // get ascociated substring
                    // see if it matches a list of possible strings
                    // if not, break with an error
                    let info = self.run_ident_loop(start);
                    (info, false)
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
                        self.bump_token().unwrap();
                        (
                            betac_ast::preprocessor(pproc),
                            self.peek().unwrap_or(Token::DUMMMY).kind == TokenKind::Eof,
                        )
                    } else {
                        (betac_ast::dummy(), true)
                    }
                }
                token @ Token {
                    kind: TokenKind::Whitespace,
                    ..
                } => (
                    betac_ast::whitespace(betac_ast::eof::Whitespace::new(token)),
                    false,
                ),
                token @ Token {
                    kind: TokenKind::NewLine,
                    ..
                } => (
                    betac_ast::new_line(betac_ast::eof::Newline::new(token)),
                    false,
                ),
                token => {
                    println!(
                        "caught token: {token:#?}, {}",
                        self.reconstruct_from_token_pair(token, self.peek().unwrap())
                    );

                    todo!()
                }
            };

        match inner.kind() {
            // whitespaces are parsed just to not have to loop, because looping
            // causes preformance issues and easy infinite loops.
            AstKind::Whitespace | AstKind::Newline => {}
            _ => {
                let token = AstToken::new(inner);
                list.push(token);
            }
        }
        finished
    }

    fn parse_vis(&mut self, meta: &AtomicU8, public: bool) {
        self.bump_token();
        match self.last_token.kind {
            TokenKind::LeftParen => {
                meta.fetch_or(Metadata::PRIVATE | Metadata::PUBLIC, Ordering::SeqCst);
                self.bump_token(); // ident
                self.bump_token(); // right_paren
            }
            TokenKind::Whitespace if public => {
                meta.fetch_or(Metadata::PUBLIC, Ordering::SeqCst);
                self.bump_token();
            }
            TokenKind::Whitespace => {
                meta.fetch_or(Metadata::PRIVATE, Ordering::SeqCst);
                self.bump_token();
            }
            _ => {
                println!("current: {:?}", self.last_token.kind);
                todo!("add proper error handling on `parse_vis`")
            }
        };
    }

    fn preprocessor(&mut self, start: u16, next_start: u16) -> PreprocessorStmt {
        let substr = self.reconstruct_from_start_len(start, (next_start - start) as u8);
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

    fn handle_rhs(&mut self) {}

    fn run_ident_loop(&mut self, mut start: u16) -> Ptr<dyn AstType> {
        static METADATA: AtomicU8 = AtomicU8::new(0);
        loop {
            let next = self.peek().unwrap();
            let substr = self.reconstruct_from_start_len(start, (next.start - start) as u8);
            match substr {
                "static" => {
                    METADATA.fetch_or(Metadata::STATIC, Ordering::SeqCst);
                }
                "constexpr" => {
                    METADATA.fetch_or(Metadata::CONSTEXPR, Ordering::SeqCst);
                }
                "consumer" => {
                    METADATA.fetch_or(Metadata::CONSUMER, Ordering::SeqCst);
                }
                "mut" => {
                    METADATA.fetch_or(Metadata::MUTABLE, Ordering::SeqCst);
                }
                "let" if METADATA.load(Ordering::SeqCst) & Metadata::STATIC != 0 => {
                    let a = self.handle_global_assignment(&METADATA);
                    break betac_ast::assign(a);
                }
                "pub" => {
                    //self.parse_vis(&METADATA, true);
                    METADATA.fetch_or(Metadata::PUBLIC, Ordering::SeqCst);
                }
                "priv" => {
                    self.parse_vis(&METADATA, false);
                }
                "defun" => {
                    let d = self.handle_defun(&METADATA);
                    break betac_ast::defun(d);
                }
                other => {
                    println!("caught substring: {other}");
                    todo!()
                }
            };

            start = self.bump_token().unwrap().start;
        }
    }

    pub fn complete(self) -> I {
        self.tokens.unwrap()
    }
}

pub struct Parser<'a, I> {
    inner: ParseInner<'a, I>,
    list: AstList,
}

pub trait Parse<'a> {
    type Iter;

    fn parse_next_expr(&mut self) -> bool;

    fn complete(self) -> (AstList, Self::Iter);
}

impl<'a, I> Parser<'a, I>
where
    I: Iterator<Item = Token> + Clone + 'a,
{
    pub fn new(input: &'a str, iter: I) -> Self {
        Self {
            inner: ParseInner::new(input, iter),
            list: AstList::new(),
        }
    }
}

impl<'a, I> Parse<'a> for Parser<'a, I>
where
    I: Iterator<Item = Token> + Clone + 'a,
{
    type Iter = I;

    fn parse_next_expr(&mut self) -> bool {
        self.inner.parse_next_expr(&mut self.list)
    }

    fn complete(self) -> (AstList, I) {
        (self.list, self.inner.complete())
    }
}
