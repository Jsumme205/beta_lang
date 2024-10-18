use std::io::Write;

pub mod cursor;

use crate::betac_util::{
    self, node::OpKind, session::Session, sso::OwnedYarn, CharVec, IntoYarn, OperatorNode, VecExt,
    Yarn,
};

use self::cursor::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    AtStart,
    AtMacro,
    AtExternal,
    AtEval,
    AtDef,
    AtFor,
    Unknown,
}

impl Kind {
    fn is_not_unknown(self) -> bool {
        match self {
            Kind::Unknown => false,
            _ => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Evaluation {
    AtStart,
    AtDef,
    AtEval {
        contents: OwnedYarn,
        should_be_included: bool,
    },
    AtFor {
        start: usize,
        end: usize,
        contents: OwnedYarn,
        name: OwnedYarn,
        offset_from_start: usize,
    },
}

impl Evaluation {
    pub fn finish(self, buf: &mut Vec<u8>) -> Result<(), betac_util::CompileError> {
        match self {
            Self::AtDef | Self::AtStart => Ok(()),
            Self::AtFor {
                start,
                end,
                contents,
                name,
                offset_from_start,
            } => {
                buf.reserve((end - start) * contents.len());
                for i in start..end {
                    println!("i is: {i}");
                    let contents = contents.replace(name.as_str(), i.to_string().into());
                    buf.write("{\n".as_bytes())?;
                    buf.write(contents.strip_back("@end;".len()).as_bytes())?;
                    buf.write("\n}\n".as_bytes())?;
                    println!("offset is: {offset_from_start}");
                }
                Ok(())
            }
            Self::AtEval {
                contents,
                should_be_included,
            } => {
                buf.reserve(contents.len());
                if should_be_included {
                    buf.write(contents.strip_back("@end;".len()).as_bytes())?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EvaluationError {
    InvalidTyForDef,
    StartIdentAlreadyFound,
    WarningDefAlreadyWritten,
    NoConstexprInAtForLoop,
    InvalidIdent,
    InvalidOp,
    NotEnoughArgs,
    InvalidNumbers,
    Dummy,
}

pub type EvalResult = Result<Evaluation, EvaluationError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Token {
    kind: Kind,
    len: usize,
}

#[derive(Debug)]
pub struct TokenCtx<'sess> {
    inner: Token,
    span: OwnedYarn,
    sess: &'sess mut Session,
}

impl TokenCtx<'_> {
    pub fn evaluate(self) -> EvalResult {
        self.inner.evaluate(self.span, self.sess)
    }
}

impl Token {
    const fn new(kind: Kind, len: usize) -> Self {
        Self { kind, len }
    }

    pub fn evaluate(self, span: OwnedYarn, sess: &mut Session) -> EvalResult {
        match self.kind {
            Kind::AtStart => {
                if sess.globals.id_start.is_none() {
                    sess.globals.id_start = Some(span);
                    Ok(Evaluation::AtStart)
                } else {
                    Err(EvaluationError::StartIdentAlreadyFound)
                }
            }
            Kind::AtDef => {
                let (key, value, valid) = Self::parse_at_def(span);
                if !valid {
                    return Err(EvaluationError::InvalidTyForDef);
                }
                if sess.globals.defined.insert(key, value).is_some() {
                    Err(EvaluationError::WarningDefAlreadyWritten)
                } else {
                    Ok(Evaluation::AtDef)
                }
            }
            Kind::AtEval => Self::parse_at_eval(span, sess),
            Kind::AtFor => Self::parse_at_for(span),
            _ => todo!(),
        }
    }

    fn parse_at_def(span: OwnedYarn) -> (Yarn<'static>, Yarn<'static>, bool) {
        let mut splits = span.split(' ').collect::<Vec<_>>();
        let name = splits.take(0);
        let name = if name.ends_with(':') {
            name.strip_back(1)
        } else {
            name
        };
        let ty = splits.take(1);
        let eq = splits.take(2);
        let (eq, n_idx) = if eq == ":" {
            (splits.take(3), 4)
        } else {
            (eq, 3)
        };
        assert!(eq.contains("=>"));
        let value = splits.take(n_idx);
        (name.leak(), value.leak(), Self::is_valid_ty(&ty))
    }

    fn parse_at_eval(span: OwnedYarn, sess: &mut Session) -> Result<Evaluation, EvaluationError> {
        let mut splits = span.split(':').collect::<Vec<_>>();
        let (stmt, contents) = (splits.take(0).strip_back(1), splits.take(1));
        println!("contents: {contents}");
        let stmt = stmt
            .split(' ')
            .into_iter()
            .map(|y| y.into())
            .collect::<Vec<String>>();
        let result = OperatorNode::new()
            .add_op(OpKind::Bin, &stmt)
            .comptime_evaluate(sess);
        Ok(Evaluation::AtEval {
            contents: contents.leak(),
            should_be_included: result,
        })
    }

    fn parse_at_for(span: OwnedYarn) -> EvalResult {
        let mut splits = span.split("):").collect::<Vec<_>>();
        let (expr, span) = (splits.take(0), splits.take(1));

        let mut expr = expr.split(' ').collect::<Vec<_>>();

        if expr.len() < 5 {
            return Err(EvaluationError::NotEnoughArgs);
        }
        if !expr.take(0).contains("constexpr") {
            return Err(EvaluationError::NoConstexprInAtForLoop);
        }
        let name = expr.take(1);
        if !name.starts_with('@') {
            return Err(EvaluationError::InvalidIdent);
        }
        let name = if name.ends_with(':') {
            name.strip_back(1)
        } else {
            name
        };
        let op = expr.take(3);
        println!("op: {op}");
        if !op.contains("=>") {
            return Err(EvaluationError::InvalidOp);
        }
        let ty = expr.take(2);
        if !Self::is_valid_ty(&ty) {
            return Err(EvaluationError::InvalidTyForDef);
        }
        let (start, end) = Self::handle_range(&expr[4..])?;
        Ok(Evaluation::AtFor {
            start,
            end,
            contents: span.leak(),
            name: name.leak(),
            offset_from_start: 0,
        })
    }

    fn is_valid_ty(ty: &Yarn<'_>) -> bool {
        match ty.as_str() {
            "Uint64" => true,
            _ => false,
        }
    }

    fn handle_range(s: &[Yarn<'_>]) -> Result<(usize, usize), EvaluationError> {
        if s.len() == 1 {
            let nums = s[0].split("..").collect::<Vec<_>>();
            let (start, end) = (
                nums[0]
                    .parse::<usize>()
                    .map_err(|_| EvaluationError::InvalidNumbers),
                nums[1]
                    .parse::<usize>()
                    .map_err(|_| EvaluationError::InvalidNumbers),
            );
            Ok((start?, end?))
        } else {
            Err(EvaluationError::Dummy)
        }
    }
}

impl<'a> Cursor<'a> {
    const EXTERNAL_LEN: usize = "external".len() + 1;
    const EVAL_LEN: usize = "eval".len() + 1;
    const DEF_LEN: usize = "def".len() + 1;
    const START_LEN: usize = "start".len() + 1;
    const MACRO_LEN: usize = "macro".len() + 1;
    const END_LEN: usize = "@end;".len();
    const FOR_LEN: usize = "for".len() + 1;

    pub fn advance_token(&mut self, buf: &mut Vec<u8>) -> (Token, Yarn<'static>) {
        //let mut chunk = self.bump_while_next(|c|);
        //buf.append(&mut chunk);
        let (kind, contents) = match self.bump() {
            None => (Kind::Unknown, Yarn::empty()),
            Some(c) => {
                println!("char is: {c}", c = (c as char));
                match c as char {
                    '@' if 's'.cmp_byte(self.next()) => {
                        let result = self.try_handle_start();
                        if result.is_some() {
                            (Kind::AtStart, result.unwrap())
                        } else {
                            (Kind::Unknown, Yarn::<'static>::empty())
                        }
                    }
                    '@' if 'm'.cmp_byte(self.next()) => {
                        let result = self.try_handle_macro();
                        if result.is_some() {
                            (Kind::AtMacro, result.unwrap())
                        } else {
                            (Kind::Unknown, Yarn::empty())
                        }
                    }
                    '@' if self.next().cmp_other('e') && self.second().cmp_other('x') => {
                        let result = self.try_handle_external();
                        if result.is_some() {
                            (Kind::AtExternal, result.unwrap())
                        } else {
                            (Kind::Unknown, Yarn::empty())
                        }
                    }
                    '@' if self.next().cmp_other('e') && self.second().cmp_other('v') => {
                        let result = self.try_handle_eval();
                        if result.is_some() {
                            (Kind::AtEval, result.unwrap())
                        } else {
                            (Kind::Unknown, Yarn::empty())
                        }
                    }
                    '@' if self.next().cmp_other('d') => {
                        let result = self.try_handle_def();
                        if result.is_some() {
                            (Kind::AtDef, result.unwrap())
                        } else {
                            (Kind::Unknown, Yarn::empty())
                        }
                    }
                    '@' if self.next().cmp_other('f') => {
                        let result = self.try_handle_for();
                        if result.is_some() {
                            (Kind::AtFor, result.unwrap())
                        } else {
                            (Kind::Unknown, Yarn::empty())
                        }
                    }
                    _ => (Kind::Unknown, Yarn::empty()),
                }
            }
        };

        let token = Token::new(kind, self.pos_within_token());
        self.reset_pos_within_token();
        let mut chunk = self.bump_while(|c| (*c as char) != '@' || (*c as char).is_whitespace());
        self.bump();
        buf.append(&mut chunk);
        (token, contents)
    }

    pub fn advance_to_ctx(&mut self, buf: &mut Vec<u8>) -> TokenCtx {
        let (inner, span) = self.advance_token(buf);
        TokenCtx {
            inner,
            span,
            sess: self.sess,
        }
    }

    fn try_handle_start(&mut self) -> Option<OwnedYarn> {
        let start = self.range(Self::START_LEN).collect();
        if start == "start" {
            self.discard(Self::START_LEN);
            self.bump_while(|c| (*c as char).is_whitespace());
            if self.next().cmp_other(' ') {
                self.bump();
            }

            let rest = self
                .bump_while_with_ctx(|_, this| !this.next().cmp_other(';'))
                .collect();
            Some(rest)
        } else {
            None
        }
    }

    fn try_handle_macro(&mut self) -> Option<OwnedYarn> {
        let mac = self.range(Self::MACRO_LEN).collect();
        if mac == "macro" {
            // just doing this to make sure the next is actually viable
            self.discard(Self::MACRO_LEN);
            self.bump_while(|c| c.cmp_other(' '));
            if self.next().cmp_other(' ') {
                self.bump();
            }

            let rest = self
                .bump_while_with_ctx(|_, this| {
                    this.range(Self::END_LEN).collect_string() != "@end;"
                })
                .collect();
            Some(rest)
        } else {
            None
        }
    }

    fn try_handle_external(&mut self) -> Option<OwnedYarn> {
        let external = self
            .range(Self::EXTERNAL_LEN)
            .into_iter()
            .collect::<OwnedYarn>();
        if external == "external" {
            self.discard(Self::EXTERNAL_LEN);
            self.bump_while(|c| c.cmp_other(' '));
            if self.next().cmp_other(' ') {
                self.bump();
            }
            let rest = self
                .bump_while_with_ctx(|_, this| {
                    this.range(Self::END_LEN).collect_string() != "@end;"
                })
                .collect();
            Some(rest)
        } else {
            None
        }
    }

    fn try_handle_eval(&mut self) -> Option<OwnedYarn> {
        let eval = self.range(Self::EVAL_LEN).collect();
        if eval == "eval" {
            self.discard(Self::EVAL_LEN);
            self.bump_while(|c| c.cmp_other(' '));
            if self.next().cmp_other(' ') {
                self.bump();
            }
            let rest = self
                .bump_while_with_ctx(|_, this| this.range(Self::END_LEN).collect_string() != "@end")
                .collect();
            Some(rest)
        } else {
            None
        }
    }

    fn try_handle_def(&mut self) -> Option<OwnedYarn> {
        let def = self.range(Self::DEF_LEN).collect();
        if def == "def" {
            self.discard(Self::DEF_LEN);
            self.bump_while(|c| c.cmp_other(' '));
            if self.next().cmp_other(' ') {
                self.bump();
            }
            Some(
                self.bump_while_with_ctx(|_, this| !this.next().cmp_other(';'))
                    .collect(),
            )
        } else {
            None
        }
    }

    fn try_handle_for(&mut self) -> Option<OwnedYarn> {
        let range = self.range(Self::FOR_LEN).collect();
        if range == "for" {
            self.discard(Self::FOR_LEN);
            self.bump_while(|c| c.cmp_other(' '));
            if self.next().cmp_other(' ') {
                self.bump();
            }
            Some(
                self.bump_while_with_ctx(|_, this| {
                    this.range(Self::END_LEN).collect_string() != "@end;"
                })
                .collect(),
            )
        } else {
            None
        }
    }
}

pub fn run_pproc(input: Yarn<'_>, session: &mut Session) -> betac_util::CompileResult<Vec<u8>> {
    let mut cursor = Cursor::init(input, session);
    let mut buf = Vec::<u8>::new();
    let mut token = cursor.advance_to_ctx(&mut buf);
    while token.inner.kind.is_not_unknown() {
        token.evaluate()?.finish(&mut buf)?;
        //let _string = unsafe { std::str::from_utf8_unchecked(&buf) };
        token = cursor.advance_to_ctx(&mut buf);
    }
    Ok(buf)
}
