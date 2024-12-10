pub mod assign;
pub mod pproc;
pub mod traits;

use crate::{
    betac_ast::{AstNode, AstToken, AtomicMetadata, Metadata, NoOp, SyntaxTree},
    betac_errors::option::ResultExtension,
    betac_tokenizer::token::{Token, TokenKind},
    betac_util::{linked_list::LinkedList, ptr::Ptr, small_vec::SmallVec},
};

use traits::{Context, ContextKind, Parse, Source};

struct ParseInner<Iter> {
    iterator: Iter,
    source: Ptr<dyn Source>,
}

impl<Iter> ParseInner<Iter>
where
    Iter: Iterator<Item = Token> + Clone,
{
    pub(crate) fn new(iter: Iter, source: Ptr<dyn Source>) -> Self {
        Self {
            iterator: iter,
            source,
        }
    }

    #[inline(always)]
    pub(crate) fn peek(&self) -> Option<Token> {
        self.iterator.clone().next()
    }

    pub(crate) fn take_until(&mut self, mut f: impl FnMut(Token) -> bool) -> SmallVec<Token> {
        let mut vec = crate::svec![];
        while let Some(token) = self.iterator.next()
            && !f(token)
        {
            vec.push(token);
        }
        vec
    }

    /// the main function for parsing
    /// this returns a boolean, representing whether the iterator was terminated or not\n
    /// `false` represents that the iterator is done\n
    /// `true` represents that there are more tokens to parse\n
    pub(crate) fn parse_expression(&mut self, ctx: &mut dyn Context) -> bool {
        let Some(next_token) = self.iterator.next() else {
            return false;
        };

        println!("next_47: {next_token:?}");
        let (metadata, result): (Ptr<dyn AstNode>, bool) = match next_token {
            // preprocessor
            Token {
                kind: TokenKind::At,
                start,
            } => {
                println!("entered pproc");

                if self.peek().is_some_and(|t| t.kind == TokenKind::LeftBrace) {
                    println!("found a tag!")
                }

                let token = match unsafe {
                    self.source
                        .reconstruct_from_start_end_unchecked(start + 1, self.peek().unwrap().start)
                        .trim()
                } {
                    "start" => {
                        let tokens = self.take_until(|token| token.kind != TokenKind::Semi);
                        self::pproc::parse_at_start(tokens)
                    }
                    caught => super::catch!(caught),
                };
                (Ptr::new(token), true)
            }
            Token {
                kind: TokenKind::Ident,
                start,
            } => {
                let next = self.peek().unwrap_or_emit();
                println!("start: {start}");
                println!("next: {next:?}");
                println!("len: {}", self.source.length());
                match unsafe {
                    self.source
                        .reconstruct_from_start_end_unchecked(start, next.start)
                } {
                    "let" => {
                        match ctx.context_kind() {
                            ContextKind::Function => {}
                            ContextKind::Global => {
                                let tokens = self.take_until(|token| token.kind != TokenKind::Semi);
                                println!("tokens: {tokens:?}");
                                self::assign::parse_global_assignment(
                                    tokens,
                                    AtomicMetadata::get().to_metadata(),
                                );
                            }
                            ContextKind::Object => {}
                        }

                        todo!()
                    }
                    "static" => {
                        AtomicMetadata::get().add_flag(Metadata::STATIC);
                        (Ptr::new(NoOp), true)
                    }
                    caught => super::catch!(caught),
                }
            }
            Token {
                kind: TokenKind::NewLine,
                ..
            } => (Ptr::new(NoOp), true),
            caught => super::catch!(tok caught, self),
        };

        // if it is a dummy, we don't push
        if metadata.is_dummy() {
            result
        } else {
            // it's not a dummy, so we push
            let token = AstToken::new(metadata);
            ctx.current_syntax_tree().push(&token);
            result
        }
    }
}

pub struct GlobalContext {
    tree: SyntaxTree,
    //source: Sso<'static>,
}

impl GlobalContext {
    pub const fn new() -> Self {
        Self {
            tree: LinkedList::new(),
        }
    }
}

impl Context for GlobalContext {
    #[inline(always)]
    fn context_kind(&self) -> traits::ContextKind {
        ContextKind::Global
    }

    fn symbol_is_in_scope(&self, token: Token) -> bool {
        true
    }

    #[inline(always)]
    fn current_syntax_tree(&mut self) -> &mut SyntaxTree {
        &mut self.tree
    }

    #[inline(always)]
    fn complete(self: Box<Self>) -> SyntaxTree {
        self.tree
    }
}

pub struct GlobalParser<Iter> {
    inner: ParseInner<Iter>,
    ctx: Box<dyn Context>,
}

impl<Iter> GlobalParser<Iter>
where
    Iter: Iterator<Item = Token> + Clone,
{
    pub fn new(source: String, iter: Iter) -> Self {
        let source: Ptr<dyn Source> = Ptr::new(source);
        Self {
            inner: ParseInner::new(iter, source),
            ctx: Box::new(GlobalContext::new()),
        }
    }
}

impl<Iter> Parse for GlobalParser<Iter>
where
    Iter: Iterator<Item = Token> + Clone,
{
    #[inline(always)]
    fn finish(self) -> SyntaxTree {
        self.ctx.complete()
    }

    #[inline(always)]
    fn next_expression(&mut self) -> bool {
        self.inner.parse_expression(&mut *self.ctx)
    }
}
