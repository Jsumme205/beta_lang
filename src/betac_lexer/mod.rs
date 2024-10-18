use std::{
    rc::Rc,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use ast_types::{
    context::{Context, ContextKind, SubContext, SymbolKind},
    defun::{Argument, DefunMeta},
    IdentOrLit, Ty,
};

use crate::{
    betac_packer::pack::Vis,
    betac_pp::cursor::CursorLike,
    betac_util::{
        self,
        session::Session,
        sso::{Bytes, OwnedYarn},
        SplitVec, Yarn,
    },
    yarn,
};

pub mod ast_types;

pub type ContextHandle<'a> = Rc<RwLock<dyn Context<'a>>>;

pub struct Source<'sess, 'src> {
    session: &'sess mut Session,
    input: Bytes<'src>,
    current: u8,
}

impl<'sess, 'src> CursorLike for Source<'sess, 'src> {
    type Element = u8;

    fn as_yarn(&self) -> Yarn<'_> {
        self.input.as_yarn()
    }

    fn prev(&self) -> Self::Element {
        0
    }

    fn next(&self) -> Self::Element {
        self.first()
    }

    fn second(&self) -> Self::Element {
        let mut iter = self.input.clone();
        iter.next();
        iter.next().unwrap_or(Self::EOF)
    }

    fn nth(&self, n: usize) -> Self::Element {
        let mut iter = self.input.clone();
        for _ in 0..(n - 1) {
            let _ = iter.next();
        }
        iter.next().unwrap_or(u8::default())
    }

    fn bump(&mut self) -> Option<Self::Element> {
        self.input.next()
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

impl<'sess, 'src> Source<'sess, 'src> {
    const EOF: u8 = b'\0';

    pub fn init(input: &'src Yarn<'src>, session: &'sess mut Session) -> Self {
        println!("input: {input}");
        let mut input = input.bytes();
        Self {
            session,
            current: input.next().unwrap_or(Self::EOF),
            input,
        }
    }

    pub fn read_byte(&mut self) {
        self.current = self.input.next().unwrap_or(Self::EOF);
    }

    pub fn first(&self) -> u8 {
        let cchar = self.input.clone().next().unwrap_or(Self::EOF);
        if cchar == Self::EOF {
            println!("first: eof");
            println!("input: {}", self.input.as_yarn());
            cchar
        } else {
            cchar
        }
    }

    pub fn next_token(&mut self) -> Rc<ast_types::Token<'src>> {
        let token = match self.current as char {
            // `=`
            '=' if betac_util::is_whitespace(self.first()) => ast_types::Token::OneEq(self.current),
            // `=>`
            '=' if self.first() == b'>' => {
                let token = ast_types::Token::Assign;
                self.read_byte();
                token
            }
            c if betac_util::is_id_start(c as u8) => self.read_ident(c as u8),
            '+' => ast_types::Token::Plus,
            '-' => ast_types::Token::Minus,
            ':' => ast_types::Token::Colon,
            ';' => ast_types::Token::Semi,
            c if betac_util::is_whitespace(c as u8) => ast_types::Token::Whitespace,
            '(' => ast_types::Token::LeftParen,
            ')' => ast_types::Token::RightParen,
            ',' => ast_types::Token::Comma,
            '{' => ast_types::Token::LeftBrace,
            '}' => ast_types::Token::RightBrace,
            '[' => ast_types::Token::LeftBracket,
            ']' => ast_types::Token::RightBracket,
            c if c.is_ascii_digit() => self.read_number(c as u8),
            c => {
                if c as u8 == Self::EOF {
                    println!("input: {}", self.input.as_yarn());
                    println!("eof");
                    return Rc::new(ast_types::Token::Eof);
                }
                println!("current: {c}");
                todo!()
            }
        };
        self.read_byte();
        let token = Rc::new(token);
        token
    }

    fn read_ident(&mut self, last: u8) -> ast_types::Token<'src> {
        let mut v = vec![last];
        v.append(&mut self.bump_while_with_ctx(|_, this| betac_util::is_id_continue(this.first())));
        let yarn = unsafe { Yarn::from_utf8_unchecked_owned(v) };
        ast_types::Token::Ident(yarn)
    }

    fn read_number(&mut self, last: u8) -> ast_types::Token<'src> {
        let mut v = vec![last];
        let mut buf = self.bump_while_with_ctx(|_, this| {
            this.next().is_ascii_digit()
                || this.next() == b'x'
                || this.next() == b'b'
                || this.next() == b'X'
        });
        v.append(&mut buf);
        let yarn = unsafe { Yarn::from_utf8_unchecked_owned(v) };
        ast_types::Token::Number(yarn)
    }
}

/// TODO: context resolution struct
pub struct Lexer<'sess, 'src> {
    source: Source<'sess, 'src>,
    current_token: Rc<ast_types::Token<'src>>,
    last_significant_token: Rc<ast_types::Token<'src>>,
}

impl<'sess, 'src> Lexer<'sess, 'src> {
    pub fn init(input: &'src Yarn<'src>, sess: &'sess mut Session) -> Self {
        let mut source = Source::init(input, sess);
        let token = source.next_token();
        Self {
            source,
            current_token: token,
            last_significant_token: Rc::new(ast_types::Token::Eof),
        }
    }

    /// TODO: find someway to just advance this to the next signifcant token
    /// AKA: the next token not whitespace
    fn advance(&mut self) {
        if *self.current_token != ast_types::Token::Whitespace {
            self.last_significant_token = std::mem::take(&mut self.current_token);
        }

        self.current_token = self.source.next_token();
    }

    fn advance_until_next_significant_token(&mut self) {
        while *self.current_token == ast_types::Token::Whitespace {
            self.advance();
        }
    }

    fn skip_whitespace(&mut self) {
        loop {
            if *self.current_token != ast_types::Token::Whitespace {
                break;
            }
            self.advance();
        }
    }

    fn advance_until_semi_or_bracket_or_brace(&mut self) {
        self.last_significant_token = std::mem::take(&mut self.current_token);
        let next_token = loop {
            let next_token = self.source.next_token();
            if next_token.is_end() {
                break next_token;
            }
        };
        self.current_token = next_token;
    }

    fn parse_expressions_until_next_brace<'a>(
        &mut self,
        ctx: ContextHandle<'a>,
    ) -> Option<Vec<ast_types::Expr<'src>>> {
        let mut open = 1;
        let mut exprs = vec![];
        while *self.current_token != ast_types::Token::RightBrace
            && *self.current_token != ast_types::Token::Eof
        {
            if *self.current_token == ast_types::Token::LeftBrace {
                open += 1;
            }
            if open == 0 {
                self.advance();
                break;
            }
            // TODO: make it not recursice
            exprs.push(self.parse_interior_expression(ctx.clone())?);
        }
        Some(exprs)
    }

    fn parse_interior_expression(
        &mut self,
        ctx: ContextHandle<'_>,
    ) -> Option<ast_types::Expr<'src>> {
        todo!()
    }

    pub fn parse_next_expr(
        &mut self,
        global_context: &'static Rc<RwLock<dyn Context<'static>>>,
    ) -> Option<ast_types::Expr<'src>> {
        let mut last_tokens = vec![];
        loop {
            match &*self.current_token {
                ast_types::Token::Ident(id) => match id.as_str() {
                    "let" => {
                        let (ident, meta, ty, value) =
                            self.assignment(&mut last_tokens, &global_context)?;
                        let mut global_context = global_context.write().unwrap();
                        if !global_context.symbol_is_in_scope(&ident) {
                            global_context.enter_symbol_into_scope(
                                ident.clone().leak(),
                                SymbolKind::Assignment(ty, meta.to_vis()),
                            );
                        } else {
                            // return Err(())
                        }
                        last_tokens.clear();
                        return Some(ast_types::Expr::Assignment {
                            ident,
                            ty,
                            value: Box::new(value),
                            meta,
                        });
                    }
                    // we just kind of advance, this gets handled in `Self::assignment`
                    "constexpr" | "static" => {
                        last_tokens.push(self.current_token.clone());
                        self.advance();
                        self.advance();
                    }
                    "pub" | "consumer" | "unsafe" | "priv" => {
                        last_tokens.push(self.current_token.clone());
                        println!("made it here");
                        self.advance();
                        self.advance();
                    }
                    "defun" => {
                        let mut global_context = global_context.write().unwrap();
                        let mut ctx = global_context.new_child_context(ContextKind::Function);
                        let (ident, meta, ret, args, is_template) =
                            self.function(&ctx, &mut last_tokens).unwrap();

                        let expressions = if is_template {
                            Vec::new()
                        } else {
                            self.parse_expressions_until_next_brace(ctx)
                                .unwrap_or(Vec::new())
                        };
                        println!("expressions: {expressions:#?}");

                        last_tokens.clear();

                        return Some(ast_types::Expr::Defun {
                            meta,
                            args,
                            expressions,
                            return_ty: ret,
                            ident,
                        });
                    }
                    s => {
                        let mut token_buf = vec![];
                        while *self.current_token != ast_types::Token::Semi
                            || *self.current_token != ast_types::Token::RightBrace
                        {
                            if *self.current_token == ast_types::Token::Eof {
                                break;
                            }
                            token_buf.push(self.current_token.clone());
                            self.advance();
                        }
                        println!("token_buf: {token_buf:#?}");
                        let token_buf = token_buf
                            .into_iter()
                            .filter(|token| **token != ast_types::Token::Whitespace)
                            .collect::<Vec<_>>();
                        println!("token_buf_316: {token_buf:#?}");
                        return self.parse_binop(&global_context, token_buf);
                    }
                },
                ast_types::Token::Whitespace => self.advance(),
                ast_types::Token::LeftBrace | ast_types::Token::RightBrace => self.advance(),
                ast_types::Token::Semi => self.advance(),
                ast_types::Token::Eof => return Some(ast_types::Expr::Eof),
                _ => {
                    todo!()
                }
            }
        }
    }

    fn assignment(
        &mut self,
        last_tokens: &mut Vec<Rc<ast_types::Token<'src>>>,
        ctx: &ContextHandle<'_>,
    ) -> Option<(
        OwnedYarn,
        ast_types::AssignmentMeta,
        ast_types::Ty,
        ast_types::Expr<'src>,
    )> {
        let mut meta = if let ast_types::Token::Ident(id) = &*self.last_significant_token {
            match id.as_str() {
                "constexpr" => {
                    ast_types::AssignmentMeta::new().add(ast_types::AssignmentMeta::CONSTEXPR)
                }
                "static" => ast_types::AssignmentMeta::new().add(ast_types::AssignmentMeta::STATIC),
                _ => ast_types::AssignmentMeta::new(),
            }
        } else {
            ast_types::AssignmentMeta::new()
        };
        if last_tokens
            .iter()
            .any(|token| **token == ast_types::Token::Ident(yarn!("pub")))
        {
            meta = meta.add(ast_types::AssignmentMeta::PUBLIC);
        }

        self.advance();
        self.advance();
        println!("current_228: {:#?}", self.current_token);
        let ident: Yarn<'static> = if let ast_types::Token::Ident(id) = &*self.current_token {
            let id = id.clone();
            loop {
                match id.as_str() {
                    "mut" => {
                        meta = meta.add(ast_types::AssignmentMeta::MUTABLE);
                        self.advance();
                    }
                    id if betac_util::is_valid(&yarn!("{}", id)) => break yarn!("{}", id).leak(),
                    _ => return None,
                }
            }
        } else {
            self.advance_until_semi_or_bracket_or_brace();
            return None;
        };
        self.advance();
        self.advance();
        self.advance();
        let ty = if let ast_types::Token::Ident(id) = &*self.current_token {
            ast_types::Ty::try_get(id.clone().leak(), &self.source.session)?
        } else {
            self.advance_until_semi_or_bracket_or_brace();
            return None;
        };
        self.advance();
        self.advance();
        self.advance();
        self.advance();

        let res = Some((ident, meta, ty, self.parse_rhs(ctx)?));
        self.advance();
        res
    }

    fn function(
        &mut self,
        ctx: &Rc<RwLock<dyn Context<'_>>>,
        last_tokens: &mut Vec<Rc<ast_types::Token<'src>>>,
    ) -> Option<(
        OwnedYarn,
        ast_types::defun::DefunMeta,
        ast_types::Ty,
        Vec<ast_types::defun::Argument<'src>>,
        bool,
    )> {
        let meta = if !last_tokens.is_empty() {
            let mut m = ast_types::defun::DefunMeta::new();
            let ctx = ctx.read().unwrap();
            if *last_tokens[0] == ast_types::Token::Ident(yarn!("pub")) {
                m = m.add(DefunMeta::PUBLIC);
            }
            if let Some(ast_types::Token::Ident(id)) = last_tokens.get(1).map(|opt| opt.as_ref()) {
                match id.as_str() {
                    "constexpr" => m = m.add(DefunMeta::CONSTEXPR),
                    "unsafe" => m = m.add(DefunMeta::UNSAFE),
                    "consumer" if ctx.parent_context_kind() == ContextKind::Object => {
                        m = m.add(DefunMeta::CONSUMER)
                    }
                    "mut" if ctx.parent_context_kind() == ContextKind::Object => {
                        m = m.add(DefunMeta::MUTABLE)
                    }
                    _ => panic!("unregognized token in stream!"),
                }
            }

            if let Some(ast_types::Token::Ident(id)) = last_tokens.get(2).map(|opt| opt.as_ref()) {
                match id.as_str() {
                    "unsafe" => m = m.add(DefunMeta::UNSAFE),
                    "consumer" if ctx.parent_context_kind() == ContextKind::Object => {
                        m = m.add(DefunMeta::CONSUMER)
                    }
                    "mut" if ctx.parent_context_kind() == ContextKind::Object => {
                        m = m.add(DefunMeta::MUTABLE)
                    }
                    _ => panic!("unregognized token in stream!"),
                }
            }

            if let Some(ast_types::Token::Ident(id)) = last_tokens.get(3).map(|opt| opt.as_ref()) {
                match id.as_str() {
                    "consumer" if ctx.parent_context_kind() == ContextKind::Object => {
                        m = m.add(DefunMeta::CONSUMER)
                    }
                    "mut" if ctx.parent_context_kind() == ContextKind::Object => {
                        m = m.add(DefunMeta::MUTABLE)
                    }
                    _ => panic!("unregognized token in stream!"),
                }
            }
            m
        } else {
            ast_types::defun::DefunMeta::new()
        };

        self.advance();
        self.advance();
        let token_clone = self.current_token.clone();
        let ident = if let ast_types::Token::Ident(id) = token_clone.as_ref() {
            id
        } else {
            self.advance_until_semi_or_bracket_or_brace();
            return None;
        };
        self.advance();
        let args = self.parse_arguments(ctx)?;
        self.advance();
        self.advance();
        self.advance();
        self.advance();
        let ret_ty = if let ast_types::Token::Ident(id) = &*self.current_token {
            Ty::try_get(id.clone().leak(), &self.source.session)?
        } else {
            return None;
        };
        let mut ctx = ctx.write().unwrap();
        ctx.enter_symbol_into_parent_scope(
            ident.clone().leak(),
            SymbolKind::Function(ret_ty, SplitVec::split_off(&args), meta.to_vis()),
        );
        drop(ctx);
        self.advance();
        self.advance_until_next_significant_token();
        let is_template = *self.current_token == ast_types::Token::Semi;

        Some((ident.clone().leak(), meta, ret_ty, args, is_template))
    }

    fn parse_rhs(&mut self, ctx: &ContextHandle<'_>) -> Option<ast_types::Expr<'src>> {
        match &*self.current_token {
            ast_types::Token::Number(num) => self.literal_number(num.clone(), ctx),
            ast_types::Token::Ident(ident) => self.ident(ident.clone(), ctx),

            _ => todo!(),
        }
    }

    fn parse_arguments(
        &mut self,
        ctx: &Rc<RwLock<dyn Context<'_>>>,
    ) -> Option<Vec<Argument<'src>>> {
        let mut tokens = vec![];
        let mut count = 1;
        // this is to collect the tokens, we run a seperate loop to parse them
        while *self.current_token != ast_types::Token::RightParen && count != 0 {
            if *self.current_token == ast_types::Token::LeftParen {
                count += 1;
            }
            if *self.current_token == ast_types::Token::RightParen {
                count -= 1;
                if count == 0 {
                    break;
                }
            }
            if *self.current_token == ast_types::Token::Eof {
                break;
            }
            self.advance();
            tokens.push(self.current_token.clone());
        }

        let tokens = tokens
            .into_iter()
            .filter(|token| !token.is_sep())
            .collect::<Vec<_>>();

        println!("tokens_523: {tokens:#?}");
        let range = if tokens.len() == 2 {
            0..=1
        } else {
            0..=(tokens.len() / 2) + 1
        };
        let buf = range
            .step_by(2)
            .map(|i| {
                println!("len: {}", tokens.len() / 2);
                println!("i: {i}");
                let ident = tokens[i].clone();
                println!("ident_527: {ident:#?}");
                if let ast_types::Token::Ident(id) = &*ident
                    && let ast_types::Token::Ident(ty) = &*tokens[i + 1]
                {
                    let mut ctx = ctx.write().unwrap();
                    let ty = Ty::try_get(ty.clone().leak(), &self.source.session).unwrap();
                    ctx.enter_symbol_into_scope(
                        id.clone().leak(),
                        SymbolKind::Assignment(ty, Vis::Private),
                    );
                    Some((id.clone().leak(), ty))
                } else {
                    None
                }
            })
            .try_collect::<Vec<_>>();

        buf
    }

    fn literal_number(
        &mut self,
        _num: Yarn<'src>,
        ctx: &ContextHandle<'_>,
    ) -> Option<ast_types::Expr<'src>> {
        let mut token_buf = vec![];
        while *self.current_token != ast_types::Token::Semi
            || *self.current_token != ast_types::Token::RightBrace
        {
            if *self.current_token == ast_types::Token::Eof {
                break;
            }
            token_buf.push(self.current_token.clone());
            self.advance();
        }
        println!("token_buf_534: {token_buf:#?}");

        let token_buf = token_buf
            .into_iter()
            .filter(|token| **token != ast_types::Token::Whitespace)
            .collect::<Vec<_>>();

        if token_buf.len() == 1
            && let ast_types::Token::Number(id) = &*token_buf[0]
        {
            return Some(ast_types::Expr::Literal(id.clone()));
        } else {
            return self.parse_binop(ctx, token_buf);
        }
    }

    fn parse_binop(
        &mut self,
        ctx: &ContextHandle<'_>,
        args: Vec<Rc<ast_types::Token<'src>>>,
    ) -> Option<ast_types::Expr<'src>> {
        if let Some(lhs) = args[0].number_or_ident()
            && args[1].is_operator()
            && let Some(rhs) = args[2].number_or_ident()
        {
            let ctx = ctx.read().unwrap();
            match (lhs, rhs) {
                (IdentOrLit::Ident(lhs), IdentOrLit::Number(rhs)) => {
                    let ty = ctx.kind_for_symbol(&lhs)?;
                    if let SymbolKind::Assignment(ty, _) = ty {
                        if !ty.is_number() {
                            return None;
                        }
                        return Some(ast_types::Expr::Binary {
                            lhs: Box::new(ast_types::Expr::LitOrIdent(lhs, ty)),
                            rhs: Box::new(ast_types::Expr::LitOrIdent(rhs, ty)),
                            op: args[1].as_binop().unwrap(),
                            ty,
                            context_kind: ctx.kind(),
                        });
                    } else {
                        return None;
                    }
                }
                (IdentOrLit::Number(lhs), IdentOrLit::Number(rhs)) => {
                    return Some(ast_types::Expr::Binary {
                        lhs: Box::new(ast_types::Expr::Literal(lhs)),
                        rhs: Box::new(ast_types::Expr::Literal(rhs)),
                        op: args[1].as_binop().unwrap(),
                        ty: Ty::TY_UINT32,
                        context_kind: ctx.kind(),
                    });
                }
                (IdentOrLit::Ident(lhs), IdentOrLit::Ident(rhs)) => {
                    let lhs_ty = ctx.kind_for_symbol(&lhs)?;
                    let rhs_ty = ctx.kind_for_symbol(&rhs)?;
                    if let SymbolKind::Assignment(lhs_ty, _) = lhs_ty
                        && let SymbolKind::Assignment(rhs_ty, _) = rhs_ty
                    {
                        if rhs_ty == lhs_ty
                            || rhs_ty.can_be_implicitly_converted(lhs_ty)
                            || lhs_ty.can_be_implicitly_converted(rhs_ty)
                        {
                            return Some(ast_types::Expr::Binary {
                                lhs: Box::new(ast_types::Expr::LitOrIdent(lhs, lhs_ty)),
                                rhs: Box::new(ast_types::Expr::LitOrIdent(rhs, rhs_ty)),
                                op: args[1].as_binop().unwrap(),
                                ty: lhs_ty,
                                context_kind: ctx.kind(),
                            });
                        } else {
                            return None;
                        }
                    }
                }
                _ => todo!(),
            }
        }
        None
    }

    fn ident(
        &mut self,
        ident: Yarn<'src>,
        ctx: &ContextHandle<'_>,
    ) -> Option<ast_types::Expr<'src>> {
        //if ctx.symbol_is_in_scope(&ident) {
        //    return Some(ast_types::Expr::Copy(ident));
        //}

        self.advance();
        println!("current_538: {:#?}", self.current_token);
        if *self.current_token == ast_types::Token::LeftParen {
            // we reached a function call, check if it is valid
            let ctx = ctx.read().unwrap();
            println!("ident_541:{ident}");
            println!("symbols_542: {:#?}", ctx.symbols_in_parent_ctx());
            if matches!(
                ctx.kind_for_symbol(&ident).unwrap(),
                SymbolKind::Function(_, _, _)
            ) {
                println!("current_547: {:#?}", self.current_token);
                let mut count = 1;
                let mut args = vec![];
                while *self.current_token != ast_types::Token::RightParen && count != 0 {
                    if *self.current_token == ast_types::Token::LeftParen {
                        count += 1;
                    }
                    if *self.current_token == ast_types::Token::RightParen {
                        count -= 1;
                        if count == 0 {
                            break;
                        }
                    }
                    if *self.current_token == ast_types::Token::Eof {
                        break;
                    }
                    self.advance();
                    if !self.current_token.is_sep()
                        && let ast_types::Token::Ident(id) = &*self.current_token
                    {
                        args.push(id.clone());
                    }
                }
                let ty = match ctx.kind_for_symbol(&ident) {
                    Some(v) => match v {
                        SymbolKind::Function(ty, _, _) => ty,
                        _ => return None,
                    },
                    None => return None,
                };
                return Some(ast_types::Expr::Call {
                    ident,
                    args,
                    ret_ty: ty,
                });
            }
            return None;
        }

        if ctx.read().unwrap().symbol_is_in_scope(&ident) {
            return Some(ast_types::Expr::Copy(ident));
        }

        None
    }
}
