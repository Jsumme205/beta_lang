use std::{
    alloc,
    fmt::{self, Write},
    hash::Hash,
    marker::PhantomData,
    mem,
    num::NonZero,
    ops::Index,
    path::Path,
    ptr::NonNull,
    str::{
        pattern::{Pattern, ReverseSearcher},
        FromStr,
    },
    u128,
};

use crate::betac_pp::cursor::ByteChar;

pub const SSO_LEN: usize = std::mem::size_of::<usize>() * 2 - 1;

pub type OwnedYarn = Yarn<'static>;

const SMALL_MASK: usize = (1 << 1) << (usize::BITS - 2);
const SMALL: u8 = 1 << 1; // 2
const HEAP: u8 = 1 << 2; // 4
const STATIC: u8 = 0;
const BORROWED: u8 = 1;

#[repr(C)]
#[derive(Clone, Copy)]
struct Small {
    data: [u8; SSO_LEN],
    meta: NonZero<u8>,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Slice {
    data: *const u8,
    meta: NonZero<usize>,
}

#[derive(Clone, Copy)]
union RawYarn {
    small_str: Small,
    heap_str: Slice,
}

impl RawYarn {
    const fn is_small(self) -> bool {
        // SAFETY: We are reading the top byte only
        // the top byte is always guaranteed to be the metadata
        // here, we are interpreting `self` as an heap_string so we can
        // use the same mask
        let meta = unsafe { self.heap_str.meta.get() };
        meta & SMALL_MASK != 0
    }

    /// assumes that `RawYarn` is a Small string
    ///
    /// SAFETY: the caller must insure that the `RawYarn`
    /// is actually a small string
    const unsafe fn assume_small(self) -> Small {
        self.small_str
    }

    const unsafe fn assume_small_ref(&self) -> &Small {
        &self.small_str
    }

    /// assumes that `RawYarn` is a Heap string
    ///
    /// SAFETY: the caller must insure that the `RawYarn`
    /// is actually a heap string
    const unsafe fn assume_slice(self) -> Slice {
        self.heap_str
    }

    /// gets the length of the `RawYarn`
    const fn len(self) -> usize {
        let (meta, adjust) = if self.is_small() {
            (
                unsafe { self.assume_small().meta.get() as usize },
                usize::BITS - 8,
            )
        } else {
            (unsafe { self.assume_slice().meta.get() }, 0)
        };
        (meta << (2 + adjust)) >> (2 + adjust)
    }

    const fn kind(self) -> u8 {
        let meta = unsafe { self.heap_str.meta.get() };
        (meta >> (usize::BITS - 2)) as u8
    }

    const unsafe fn from_raw_parts(ptr: *const u8, len: usize, kind: u8) -> Self {
        if len <= SSO_LEN {
            let small = Self::small(ptr, len);
            Self { small_str: small }
        } else {
            let slice = Self::slice(ptr, len, kind);
            Self { heap_str: slice }
        }
    }

    const unsafe fn small(ptr: *const u8, len: usize) -> Small {
        if len > SSO_LEN {
            std::hint::unreachable_unchecked()
        }
        let tagged_len = SMALL << (u8::BITS - 2) | (len as u8);

        if mem::size_of::<Self>() == 16 {
            // this does some really weird stuff
            // (I stole it)
            // docs.rs/byteyarn/latest/src/byteyarn/raw.rs.html#335
            let register = if len > 8 {
                // so if the length of the string is greater than 8,
                // we have to seperate it into 2 seperate (64-bit) registers,
                // then OR them together (with some bitshift)
                let x0 = ptr.cast::<u64>().read_unaligned() as u128;
                let x1 = ptr.add(len - 8).cast::<u64>().read_unaligned() as u128;
                x0 | (x1 << ((len - 8) * 8))
            } else if len > 3 {
                let x0 = ptr.cast::<u32>().read_unaligned() as u128;
                let x1 = ptr.add(len - 4).cast::<u32>().read_unaligned() as u128;
                x0 | (x1 << ((len - 4) * 8))
            } else if len > 0 {
                let x0 = ptr.read() as u128;
                let x1 = ptr.add(len / 2).read() as u128;
                let x2 = ptr.add(len - 1).read() as u128;
                x0 | x1 << (len / 2 * 8) | x2 << ((len - 1) * 8)
            } else {
                0
            };

            let mut small = (&register as *const u128).cast::<Small>().read();
            small.meta = NonZero::new_unchecked(tagged_len);
            small
        } else {
            let mut small = Small {
                data: [0; SSO_LEN],
                meta: NonZero::new_unchecked(tagged_len),
            };
            let mut i = 0;
            while i < len {
                small.data[i] = *ptr.add(i);
                i += 1;
            }
            small
        }
    }

    const unsafe fn slice(ptr: *const u8, len: usize, kind: u8) -> Slice {
        assert!(len <= usize::MAX / 4);
        Slice {
            data: ptr,
            meta: NonZero::new_unchecked((kind as usize & 0b11) << (usize::BITS - 2) | len),
        }
    }

    const fn as_ptr(&self) -> *const u8 {
        unsafe {
            match self.is_small() {
                true => self.assume_small().data.as_ptr(),
                false => self.assume_slice().data,
            }
        }
    }

    /// gets the `RawYarn` as a byte slice
    unsafe fn as_slice(&self) -> &[u8] {
        match self.is_small() {
            true => &self.assume_small_ref().data[..self.len()],
            false => std::slice::from_raw_parts(self.assume_slice().data, self.len()),
        }
    }

    unsafe fn as_mut_slice(&mut self) -> &mut [u8] {
        std::slice::from_raw_parts_mut(self.as_ptr().cast_mut(), self.len())
    }

    const fn from_small_slice(layout: alloc::Layout, ptr: *const u8) -> Option<Self> {
        assert!(
            layout.align() <= mem::align_of::<Self>(),
            "cannot store types with alignment greater than a pointer in a Yarn"
        );
        if layout.size() > SSO_LEN {
            return None;
        }
        unsafe {
            let small = Self::small(ptr, layout.size());
            Some(Self { small_str: small })
        }
    }
}

/// a viewable, ASCII character sequence.
pub struct Yarn<'a> {
    raw: RawYarn,
    _ph: PhantomData<&'a [u8]>,
}

impl<'a> Yarn<'a> {
    const fn sso_qualified(len: usize) -> bool {
        len <= SSO_LEN
    }

    pub fn borrowed(data: &'a str) -> Self {
        let ptr = data.as_ptr();
        let len = data.len();
        let kind = if Self::sso_qualified(len) {
            SMALL
        } else {
            BORROWED
        };

        unsafe {
            let raw = RawYarn::from_raw_parts(ptr, len, kind);
            Self {
                raw,
                _ph: PhantomData,
            }
        }
    }

    /// creates a `Yarn` from a string constant
    pub const fn constant(data: &'static str) -> Yarn<'static> {
        let ptr = data.as_ptr();
        let len = data.len();
        let kind = if Self::sso_qualified(len) {
            SMALL
        } else {
            STATIC
        };

        unsafe {
            let raw = RawYarn::from_raw_parts(ptr, len, kind);
            Yarn::<'static> {
                raw,
                _ph: PhantomData,
            }
        }
    }

    pub const fn empty() -> Self {
        Self::constant("")
    }

    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub const unsafe fn from_utf8_unchecked(data: &'a [u8]) -> Self {
        let ptr = data.as_ptr();
        let len = data.len();
        let kind = if Self::sso_qualified(len) {
            SMALL
        } else {
            BORROWED
        };
        unsafe {
            let raw = RawYarn::from_raw_parts(ptr, len, kind);
            Self {
                raw,
                _ph: PhantomData,
            }
        }
    }

    pub fn from_bytes(data: &'a [u8]) -> Option<Self> {
        if data.iter().any(|b| b.as_byte().is_none()) {
            None
        } else {
            unsafe { Some(Self::from_utf8_unchecked(data)) }
        }
    }

    pub unsafe fn from_utf8_unchecked_owned<T>(data: T) -> Self
    where
        T: IntoIterator<Item = u8>,
    {
        let vec = data.into_iter().collect::<Vec<u8>>();
        let ptr = vec.as_ptr();
        let len = vec.len();
        mem::forget(vec);
        let kind = if Self::sso_qualified(len) {
            SMALL
        } else {
            HEAP
        };
        unsafe {
            Self {
                raw: RawYarn::from_raw_parts(ptr, len, kind),
                _ph: PhantomData,
            }
        }
    }

    /// The generic parameter is here so we can use
    /// `String` and `Box<str>`
    pub fn owned<T>(data: T) -> Self
    where
        T: Into<Box<str>>,
    {
        Self::owned_inner(data.into())
    }

    const fn owned_inner(data: Box<str>) -> Self {
        let len = data.len();
        let ptr = data.as_ptr();
        std::mem::forget(data);
        let kind = if Self::sso_qualified(len) {
            SMALL
        } else {
            HEAP
        };

        unsafe {
            let raw = RawYarn::from_raw_parts(ptr, len, kind);
            Self {
                raw,
                _ph: PhantomData,
            }
        }
    }

    pub fn from_fmt_args(args: fmt::Arguments<'_>) -> Self {
        if let Some(constant) = args.as_str() {
            return Self::constant(constant);
        }

        let mut writer = Buf::Small(0, [0; SSO_LEN]);
        let _ = writer.write_fmt(args);
        match writer {
            Buf::Slice(buf) => Self::owned(String::from_utf8(buf).unwrap()),
            Buf::Small(len, bytes) => {
                let chunk = &bytes[..len];
                let raw =
                    RawYarn::from_small_slice(alloc::Layout::for_value(chunk), chunk.as_ptr())
                        .unwrap();
                Self {
                    raw,
                    _ph: PhantomData,
                }
            }
        }
    }

    pub fn as_str(&'a self) -> &'a str {
        unsafe { std::str::from_utf8_unchecked(self.raw.as_slice()) }
    }

    pub fn leak<'b>(mut self) -> Yarn<'b> {
        if self.raw.kind() == BORROWED {
            let copy: Box<str> = self.as_str().into();
            self = Yarn::owned_inner(copy);
        }

        let raw = self.raw;
        mem::forget(self);
        Yarn::<'b> {
            raw,
            _ph: PhantomData,
        }
    }

    pub const fn len(&self) -> usize {
        self.raw.len()
    }

    pub const fn is_small(&self) -> bool {
        self.raw.is_small()
    }

    pub fn bytes<'b>(&'b self) -> Bytes<'b> {
        let (first_ref, last_ref) = unsafe {
            (
                &self.raw.as_slice()[0],
                &self.raw.as_slice()[self.raw.len() - 1],
            )
        };
        unsafe {
            Bytes {
                start: NonNull::new_unchecked(first_ref as *const _ as *mut u8),
                end: (last_ref as *const u8).add(1),
                _ph: PhantomData,
            }
        }
    }

    pub unsafe fn bytes_unchecked(&self) -> Bytes<'a> {
        let (first_ref, last_ref) = unsafe {
            (
                &self.raw.as_slice()[0],
                &self.raw.as_slice()[self.raw.len() - 1],
            )
        };

        unsafe {
            Bytes {
                start: NonNull::new_unchecked(first_ref as *const _ as *mut u8),
                end: (last_ref as *const u8).add(1),
                _ph: PhantomData,
            }
        }
    }

    pub fn borrow<'b>(&'b self) -> Yarn<'b>
    where
        'b: 'a,
    {
        Yarn::borrowed(self.as_str())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self[..]
    }

    pub fn split<'b, P: Pattern<'b>>(&'b self, pat: P) -> impl Iterator<Item = Yarn<'b>> {
        self.as_str().split(pat).map(|s| Yarn::borrowed(s))
    }

    pub fn contains<'b, P: Pattern<'b>>(&'b self, pat: P) -> bool {
        self.as_str().contains(pat)
    }

    pub fn strip_back<'b>(self, num: usize) -> Yarn<'b> {
        unsafe { Self::from_utf8_unchecked_owned(self[0..(self.len() - num)].to_vec()).leak() }
    }

    pub fn ends_with<'b, P: Pattern<'b>>(&'b self, pat: P) -> bool
    where
        <P as Pattern<'b>>::Searcher: ReverseSearcher<'b>,
    {
        self.as_str().ends_with(pat)
    }

    pub fn starts_with<'b, P: Pattern<'b>>(&'b self, pat: P) -> bool {
        self.as_str().starts_with(pat)
    }

    pub fn parse<T>(&self) -> Result<T, <T as FromStr>::Err>
    where
        T: FromStr,
    {
        self.as_str().parse()
    }

    pub fn replace<'b, P: Pattern<'b>>(&'b self, pat: P, s: Yarn<'a>) -> Yarn<'static> {
        self.as_str().replace(pat, s.as_str()).into()
    }
}

impl Into<String> for Yarn<'_> {
    fn into(self) -> String {
        unsafe { String::from_utf8_unchecked(self.as_str().as_bytes().to_vec()) }
    }
}

impl From<String> for Yarn<'_> {
    fn from(value: String) -> Self {
        Yarn::owned(value)
    }
}

impl Default for Yarn<'_> {
    fn default() -> Self {
        Self::empty()
    }
}

impl Into<Box<str>> for Yarn<'_> {
    fn into(self) -> Box<str> {
        Into::<String>::into(self).into()
    }
}

impl Into<Vec<u8>> for OwnedYarn {
    fn into(self) -> Vec<u8> {
        Into::<String>::into(self).into_bytes()
    }
}

impl Drop for Yarn<'_> {
    fn drop(&mut self) {
        if self.raw.kind() == HEAP {
            let _ = unsafe { Box::from_raw(self.raw.as_mut_slice()) };
        }
    }
}

impl<'a> Clone for Yarn<'a> {
    fn clone(&self) -> Self {
        let copied: Box<str> = self.as_str().into();
        Self::owned(copied)
    }
}

impl<'a> fmt::Display for Yarn<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl<'a> fmt::Debug for Yarn<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Yarn<'_>")
            .field("value", &self.as_str())
            .field("len", &self.len())
            .finish()
    }
}

impl FromIterator<u8> for Yarn<'static> {
    fn from_iter<T: IntoIterator<Item = u8>>(iter: T) -> Self {
        unsafe { Self::from_utf8_unchecked_owned(iter) }
    }
}

impl<'a, 'b> PartialEq<&'b str> for Yarn<'a> {
    fn eq(&self, other: &&'b str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<str> for Yarn<'_> {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl<'a> Eq for Yarn<'a> {}

impl<'a, 'b> PartialEq<Yarn<'b>> for Yarn<'a> {
    fn eq(&self, other: &Yarn<'b>) -> bool {
        self.as_str() == other.as_str()
    }
}

impl<'a> Hash for Yarn<'a> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::hash::Hash::hash(self.as_str(), state);
    }
}

impl<'a> AsRef<Path> for Yarn<'a> {
    fn as_ref(&self) -> &Path {
        self.as_str().as_ref()
    }
}

#[macro_export]
macro_rules! yarn {
    ($($args:tt)*) => {
        $crate::betac_util::sso::Yarn::from_fmt_args(::std::format_args!($($args)*))
    };
}

enum Buf {
    Small(usize, [u8; SSO_LEN]),
    Slice(Vec<u8>),
}

impl fmt::Write for Buf {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        match self {
            Self::Small(len, bytes) => {
                let new_len = *len + s.len();
                if new_len > SSO_LEN {
                    let mut vec = Vec::from(&bytes[..*len]);
                    vec.extend_from_slice(s.as_bytes());
                    *self = Self::Slice(vec);
                } else {
                    let _ = &bytes[*len..new_len].copy_from_slice(s.as_bytes());
                    *len = new_len;
                }
            }
            Self::Slice(buf) => {
                buf.extend_from_slice(s.as_bytes());
            }
        }
        Ok(())
    }
}

pub struct Bytes<'a>
where
    u8: 'a,
{
    start: NonNull<u8>,
    end: *const u8,
    _ph: PhantomData<&'a [u8]>,
}

impl<'a> Bytes<'a> {
    unsafe fn post_inc_start(&mut self, off: usize) -> NonNull<u8> {
        let old = self.start;
        self.start = self.start.add(off);
        old
    }

    fn len_impl(&self) -> usize {
        let start_addr = self.start.as_ptr() as usize;
        let end_addr = self.end as usize;
        end_addr.abs_diff(start_addr)
    }

    unsafe fn next_impl(&mut self) -> Option<u8> {
        if self.is_end() {
            None
        } else {
            Some(self.post_inc_start(1).read())
        }
    }

    pub fn is_end(&self) -> bool {
        self.start.as_ptr().cast_const() == self.end
    }

    pub fn as_bytes(&self) -> &'a [u8] {
        unsafe { std::slice::from_raw_parts(self.start.as_ptr(), self.len_impl()) }
    }

    pub fn as_yarn(&self) -> Yarn<'a> {
        unsafe { Yarn::from_utf8_unchecked(self.as_bytes()) }
    }
}

impl<I> Index<I> for Yarn<'_>
where
    [u8]: Index<I>,
{
    type Output = <[u8] as Index<I>>::Output;

    fn index(&self, index: I) -> &Self::Output {
        unsafe { &self.raw.as_slice()[index] }
    }
}

impl<'a> Iterator for Bytes<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe { self.next_impl() }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len_impl(), Some(self.len_impl()))
    }
}

impl<'a> ExactSizeIterator for Bytes<'a> {
    fn len(&self) -> usize {
        self.len_impl()
    }
}

impl Clone for Bytes<'_> {
    fn clone(&self) -> Self {
        Self {
            start: self.start,
            end: self.end,
            _ph: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assert_empty_str_not_panic() {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| Yarn::empty())).unwrap();
    }

    #[test]
    fn test_format() {
        let x = 2;
        assert_eq!(crate::yarn!("x: {x}"), "x: 2");
    }
}
