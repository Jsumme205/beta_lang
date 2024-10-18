use crate::betac_util::{sso::Bytes, Yarn};

use crate::betac_util::session::Session;

pub trait ByteChar {
    fn as_byte(&self) -> Option<u8>;

    fn cmp_byte(&self, byte: u8) -> bool {
        self.as_byte().is_some_and(|b| b == byte)
    }

    fn cmp_other<B>(&self, other: B) -> bool
    where
        B: ByteChar + Copy,
    {
        self.as_byte()
            .is_some_and(|b| other.as_byte().is_some_and(|o| o == b))
    }
}

impl ByteChar for char {
    fn as_byte(&self) -> Option<u8> {
        if self.is_ascii() {
            Some(*self as u8)
        } else {
            None
        }
    }
}

impl ByteChar for u8 {
    fn as_byte(&self) -> Option<u8> {
        Some(*self)
    }
}

pub trait CursorLike {
    type Element: Default + Copy + 'static;

    fn as_yarn(&self) -> Yarn<'_>;

    fn prev(&self) -> Self::Element;

    fn next(&self) -> Self::Element;

    fn second(&self) -> Self::Element;

    fn nth(&self, n: usize) -> Self::Element;

    fn range(&self, e: usize) -> Vec<Self::Element> {
        let mut buf = vec![];
        for i in 1..e {
            buf.push(self.nth(i))
        }
        buf
    }

    fn bump(&mut self) -> Option<Self::Element>;

    fn bump_while(&mut self, f: impl Fn(&Self::Element) -> bool) -> Vec<Self::Element> {
        let mut buf = vec![];
        while f(&self.second()) && !self.is_at_end() {
            buf.push(self.bump().unwrap_or(Default::default()));
        }
        buf
    }

    fn bump_while_with_ctx(
        &mut self,
        mut f: impl FnMut(&Self::Element, &mut Self) -> bool,
    ) -> Vec<Self::Element> {
        let mut buf = vec![];
        while f(&self.next(), self) && !self.is_at_end() {
            buf.push(self.bump().unwrap_or(Default::default()));
        }
        buf
    }

    fn bump_while_next(&mut self, f: impl Fn(&Self::Element) -> bool) -> Vec<Self::Element> {
        let mut buf = vec![];
        while f(&self.nth(2)) && !self.is_at_end() {
            buf.push(self.next());
        }
        buf
    }

    fn discard(&mut self, num: usize) {
        for _ in 0..num {
            let _ = self.bump();
        }
    }

    fn is_at_end(&self) -> bool;

    fn pos_within_token(&self) -> usize;

    fn reset_pos_within_token(&mut self);
}

pub struct Cursor<'a> {
    iter: Bytes<'a>,
    prev: u8,
    pos_within_token: usize,
    pub sess: &'a mut Session,
}

impl<'a> Cursor<'a> {
    pub fn init(input: Yarn<'a>, session: &'a mut Session) -> Self {
        Self {
            iter: unsafe { input.bytes_unchecked() },
            prev: 0,
            pos_within_token: 0,
            sess: session,
        }
    }
}

impl<'a> CursorLike for Cursor<'a> {
    type Element = u8;

    fn as_yarn(&self) -> Yarn<'_> {
        self.iter.as_yarn()
    }

    fn prev(&self) -> Self::Element {
        self.prev
    }

    fn next(&self) -> Self::Element {
        let mut iter = self.iter.clone();
        iter.next().unwrap_or(u8::default())
    }

    fn second(&self) -> Self::Element {
        let mut iter = self.iter.clone();
        iter.next();
        iter.next().unwrap_or(u8::default())
    }

    fn nth(&self, n: usize) -> Self::Element {
        let mut iter = self.iter.clone();
        for _ in 0..(n - 1) {
            let _ = iter.next();
        }
        iter.next().unwrap_or(u8::default())
    }

    fn bump(&mut self) -> Option<Self::Element> {
        let next = self.iter.next()?;
        self.prev = next;
        self.pos_within_token += 1;
        Some(next)
    }

    fn is_at_end(&self) -> bool {
        self.iter.is_end()
    }

    fn pos_within_token(&self) -> usize {
        self.pos_within_token
    }

    fn reset_pos_within_token(&mut self) {
        self.pos_within_token = 0;
    }
}
