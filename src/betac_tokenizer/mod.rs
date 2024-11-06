use std::cell::RefCell;
use std::rc::Rc;
use token::{Token, TokenKind};

pub mod token;

pub struct Tokenizer<'a> {
    input: &'a [u8],
    idx: usize,
}

impl<'a> Tokenizer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            idx: 0,
        }
    }

    pub fn next(&self) -> char {
        self.input
            .get(self.idx + 1)
            .map(|c| *c as char)
            .unwrap_or('\0')
    }

    pub fn prev(&self) -> char {
        self.input[self.idx - 1] as char
    }

    pub fn nth_next(&self, pos: usize) -> char {
        self.input[self.idx + pos] as char
    }

    pub fn nth_prev(&self, pos: usize) -> char {
        self.input.get(self.idx + pos).map(|i| *i).unwrap_or(b'\0') as char
    }

    pub fn bump(&mut self) -> Option<char> {
        let c = self.input.get(self.idx).map(|i| *i as char)?;
        self.idx += 1;
        Some(c)
    }

    pub fn is_eof(&self) -> bool {
        self.input.get(self.idx).is_none()
    }

    pub fn eat_while(&mut self, mut pred: impl FnMut(char) -> bool) {
        while pred(self.next()) && !self.is_eof() {
            self.bump();
        }
    }

    pub fn as_str(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(self.input) }
    }
}

pub fn run_tokenizer(input: &str) -> impl Iterator<Item = Token> + Clone + '_ {
    // TODO: if we ever do multithreaded stuff, then we need to change to Arc<Mutex<>>
    let tokenizer = Rc::new(RefCell::new(Tokenizer::new(input)));
    crate::betac_util::from_fn(move || {
        let token = tokenizer.borrow_mut().advance_token();
        if token.kind() == TokenKind::Eof || token.kind() == TokenKind::Unknown {
            None
        } else {
            Some(token)
        }
    })
}
