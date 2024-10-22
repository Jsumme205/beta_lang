pub mod ast_types;

use std::{
    rc::Rc,
    sync::{atomic::AtomicBool, RwLock, RwLockWriteGuard},
};

use ast_types::{
    context::{Context, FxHashMap, GlobalContext},
    AnyMetadata, Metadata, RawToken, Token, CONSTEXPR, CONSUMER, PUBLIC, STATIC,
};

use crate::{
    betac_errors::{LexerEntry, LexerError},
    betac_pp::cursor::CursorLike,
    betac_util::{self, session::Session, sso::Bytes, Yarn},
};

pub struct SourceCodeReader<'a> {
    input: Bytes<'a>,
    last: u8,
    next: u8,
    column: u32,
    pos: u32,
}

impl<'a> SourceCodeReader<'a> {
    pub fn init(input: &'a Yarn<'a>) -> Self {
        let iter = input.bytes();
        Self {
            input: iter,
            last: 0,
            next: input[1],
            column: 0,
            pos: 0,
        }
    }

    pub fn next_token(&mut self, nl_count: u32) -> Rc<ast_types::Token> {
        let start = self.pos;
        let (raw_token, pos) = match self.bump().unwrap_or('\0') {
            '\0' => (ast_types::RawToken::Eof, 1),
            '(' => (ast_types::RawToken::LeftParen, 1),
            '{' => (ast_types::RawToken::LeftBrace, 1),
            '[' => (ast_types::RawToken::LeftBracket, 1),
            ')' => (ast_types::RawToken::LeftParen, 1),
            '}' => (ast_types::RawToken::RightBrace, 1),
            ']' => (ast_types::RawToken::RightBracket, 1),
            ':' if CursorLike::next(self) == ':' => {
                self.bump();
                (ast_types::RawToken::Path, 2)
            }
            ':' => (ast_types::RawToken::Colon, 1),
            ';' => (ast_types::RawToken::Semi, 1),
            '=' if CursorLike::next(self) == '>' => {
                self.bump();
                (ast_types::RawToken::Assign, 2)
            }
            '\n' => {
                self.column = 0;
                (ast_types::RawToken::NewLine, 0)
            }
            ',' => (ast_types::RawToken::Comma, 1),
            '+' => (ast_types::RawToken::Plus, 1),
            c if c.is_numeric() => self.number(c),
            c if betac_util::is_whitespace(c as u8) => (ast_types::RawToken::Whitespace, 1),
            c if betac_util::is_id_start(c as u8) => self.ident(c),
            c => {
                println!("failed at: {c}");
                todo!()
            }
        };

        self.column += pos;
        Rc::new(Token::new(
            raw_token,
            start,
            start + pos,
            nl_count,
            self.column,
        ))
    }

    pub fn column(&self) -> u32 {
        self.column
    }

    fn number(&mut self, last: char) -> (ast_types::RawToken, u32) {
        let mut buf = vec![last as u8];
        let number: Yarn<'static> = unsafe {
            Yarn::from_utf8_unchecked_owned(
                self.bump_while(|c| {
                    let c = *c;
                    println!("current_88: {c}");
                    c.is_numeric() || c != 'x' || c != 'X' || c != 'b' || c != 'B'
                })
                .into_iter()
                .map(|c| c as u8)
                .collect_into(&mut buf)
                .drain(..),
            )
            .strip_back(1)
        };
        let len = number.len();
        (ast_types::RawToken::Number(number), len as u32)
    }

    fn ident(&mut self, c: char) -> (ast_types::RawToken, u32) {
        let mut buf = vec![c];
        let ident: Yarn<'static> = unsafe {
            Yarn::from_utf8_unchecked_owned(
                self.bump_while(|c| {
                    println!("c_69: {c}");
                    betac_util::is_id_continue(*c as u8)
                })
                .into_iter()
                .collect_into(&mut buf)
                .into_iter()
                .map(|c| *c as u8),
            )
        };
        let len = ident.len() as u32;
        (ast_types::RawToken::Ident(ident), len)
    }
}

impl<'a> Iterator for SourceCodeReader<'a> {
    type Item = Rc<ast_types::Token>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.next_token(0);
        if *next == ast_types::RawToken::Eof {
            None
        } else {
            Some(next)
        }
    }
}

impl<'a> CursorLike for SourceCodeReader<'a> {
    type Element = char;

    fn as_yarn(&self) -> Yarn<'_> {
        self.input.as_yarn()
    }

    fn prev(&self) -> Self::Element {
        self.last as char
    }

    fn next(&self) -> Self::Element {
        self.input.clone().next().unwrap_or(0) as char
    }

    fn second(&self) -> Self::Element {
        self.input.clone().next().unwrap_or(0) as char
    }

    fn nth(&self, n: usize) -> Self::Element {
        let mut iter = self.input.clone();
        for _ in 0..(n - 1) {
            iter.next();
        }
        iter.next().unwrap_or(0) as char
    }

    fn bump(&mut self) -> Option<Self::Element> {
        let c = self.input.next()?;
        self.last = c;
        self.pos += 1;
        Some(c as char)
    }

    fn is_at_end(&self) -> bool {
        self.input.is_end()
    }

    fn pos_within_token(&self) -> usize {
        unimplemented!()
    }

    fn reset_pos_within_token(&mut self) {
        unimplemented!()
    }
}

pub struct Lexer<'a> {
    token_reader: SourceCodeReader<'a>,
    last_significant_token: Rc<ast_types::Token>,
    currently_evaluated_token: Rc<ast_types::Token>,
    session: &'a mut Session,
    nl_count: usize,
    pub(crate) errors: RwLock<Vec<LexerEntry>>,
    pub(crate) guard: AtomicBool,
}

impl<'a> Lexer<'a> {
    pub fn init(input: &'a Yarn<'a>, session: &'a mut Session) -> Self {
        let mut token_reader = SourceCodeReader::init(input);
        Self {
            last_significant_token: Rc::default(),
            currently_evaluated_token: token_reader.next_token(0),
            token_reader,
            session,
            nl_count: 0,
            errors: RwLock::new(Vec::new()),
            guard: AtomicBool::new(false),
        }
    }

    pub fn nl_count(&self) -> usize {
        self.nl_count
    }

    pub fn column(&self) -> usize {
        self.token_reader.column as usize
    }

    pub(self) fn advance(&mut self) {
        if *self.currently_evaluated_token.as_raw() != ast_types::RawToken::Whitespace {
            self.last_significant_token = self.currently_evaluated_token.clone();
        }
        self.currently_evaluated_token = self.token_reader.next_token(self.nl_count as u32);
        if *self.currently_evaluated_token.as_raw() == ast_types::RawToken::NewLine {
            self.nl_count += 1;
        }
    }

    pub(crate) fn expr_loop(&mut self, delim: RawToken) -> Vec<Rc<Token>> {
        let mut current_tokens = vec![];
        let mut brace_count = 0;
        let mut brace_entered = false;
        let mut paren_count = 0;
        let mut paren_entered = false;
        // collect the expression, this is kind of why we're using `Rc` everywhere, for cheap cloning
        while *self.currently_evaluated_token != delim && brace_count == 0
            || brace_entered && paren_count == 0
            || paren_entered
        {
            match &self.currently_evaluated_token.as_raw() {
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
        current_tokens
    }

    pub fn parse_next_expr<C>(&mut self, fn_context: Option<C>) -> Option<ast_types::Expr>
    where
        C: for<'b> Context<'b>,
    {
        let mut meta = AnyMetadata::init();
        loop {
            println!("current_268: {:#?}", self.currently_evaluated_token);
            match &*self.currently_evaluated_token {
                // each coresponding function lives in its own file, with its dependencies
                // e.g imports.rs contains `Lexer::import()`, assignment.rs contains `Lexer::assignment()`
                expr_begin if self.currently_evaluated_token.ident_is_expr_start() => {
                    println!("got to 182");
                    match expr_begin.as_ident().unwrap().as_str() {
                        "let" => {
                            let g_ctx = self.session.get_current_context();
                            let g_ctx = g_ctx.write().unwrap();
                            println!("got to let");
                            let mut ctx = match fn_context {
                                Some(ctx) => FnCtxOrGlobalCtx::Fn(ctx),
                                None => FnCtxOrGlobalCtx::Global(g_ctx),
                            };
                            return self.assignment(meta.to_assignment(), &mut ctx);
                        }
                        "import" => {
                            println!("got to import");
                            return self.import(meta.to_import());
                        }
                        "defun" => {
                            println!("got to defun");
                            return self.defun(meta.to_defun());
                        }
                        _ => todo!(),
                    }
                }
                modifier if self.currently_evaluated_token.ident_is_modifier() => {
                    println!("got to 192");
                    match modifier.as_ident().unwrap().as_str() {
                        "pub" => meta = meta.add_flag(PUBLIC),
                        "constexpr" => meta = meta.add_flag(CONSTEXPR),
                        "static" => meta = meta.add_flag(STATIC),
                        "consumer" => meta = meta.add_flag(CONSUMER),
                        _ => unreachable!(),
                    }
                }
                ident if self.currently_evaluated_token.is_ident() => {
                    let ident = ident.as_raw();
                    todo!()
                }
                _ if self.currently_evaluated_token.is_whitespace() => {
                    self.advance();
                    continue;
                }
                _ => {
                    println!("failed at {:#?}", self.currently_evaluated_token);
                    todo!()
                }
            }
            self.advance();
        }
    }
}

pub enum FnCtxOrGlobalCtx<'a, C: Context<'a>> {
    Fn(C),
    Global(RwLockWriteGuard<'a, GlobalContext>),
}

macro_rules! delagate {
    ($handle:ident.$fn:ident()) => {
        match $handle {
            Self::Fn(c) => c.$fn(),
            Self::Global(c) => c.$fn(),
        }
    };
    ($handle:ident.$fn:ident($arg:expr)) => {
        match $handle {
            Self::Fn(c) => c.$fn($arg),
            Self::Global(c) => c.$fn($arg),
        }
    };
    ($handle:ident.$fn:ident($arg:expr, $arg2:expr)) => {
        match $handle {
            Self::Fn(c) => c.$fn($arg, $arg2),
            Self::Global(c) => c.$fn($arg, $arg2),
        }
    };
}

impl<'a, C: Context<'a>> Context<'a> for FnCtxOrGlobalCtx<'a, C> {
    fn symbol_is_in_scope(&self, sym: &Yarn<'_>) -> bool {
        delagate!(self.symbol_is_in_scope(sym))
    }

    fn kind(&self) -> ast_types::context::ContextKind {
        delagate!(self.kind())
    }

    fn kind_for_symbol(&self, sym: &Yarn<'_>) -> Option<ast_types::context::SymbolKind> {
        delagate!(self.kind_for_symbol(sym))
    }

    fn new_child_context<'child>(
        &'child mut self,
        kind: ast_types::context::ContextKind,
    ) -> Rc<RwLock<dyn Context<'child> + 'child>> {
        delagate!(self.new_child_context(kind))
    }

    fn symbols_in_context(&self) -> &FxHashMap<Yarn<'a>, ast_types::context::SymbolKind> {
        delagate!(self.symbols_in_context())
    }

    fn parent_context_kind(&self) -> ast_types::context::ContextKind {
        delagate!(self.parent_context_kind())
    }

    fn symbols_in_parent_ctx(&self) -> &FxHashMap<Yarn<'_>, ast_types::context::SymbolKind> {
        delagate!(self.symbols_in_parent_ctx())
    }

    fn symbols_in_context_mut(
        &mut self,
    ) -> &mut FxHashMap<Yarn<'a>, ast_types::context::SymbolKind> {
        unimplemented!()
    }

    fn enter_symbol_into_scope(
        &mut self,
        sym: Yarn<'a>,
        kind: ast_types::context::SymbolKind,
    ) -> bool {
        delagate!(self.enter_symbol_into_scope(sym.leak(), kind))
    }

    fn enter_symbol_into_parent_scope(
        &mut self,
        sym: Yarn<'static>,
        kind: ast_types::context::SymbolKind,
    ) -> bool {
        delagate!(self.enter_symbol_into_parent_scope(sym.leak(), kind))
    }
}
