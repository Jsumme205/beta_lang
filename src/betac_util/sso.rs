use std::{alloc::Layout, fmt::Debug, marker::PhantomData, num::NonZero};

use crate::betac_parser::traits::Source;

const SMALL: u8 = 1;
const STATIC: u8 = 2;
const HEAP: u8 = 3;
const BORROWED: u8 = 0;

#[repr(C)]
#[derive(Clone, Copy)]
struct Small {
    data: [u8; RawSso::SSO_LEN],
    meta: NonZero<u8>,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Heap {
    data: *const u8,
    meta: NonZero<usize>,
}

#[derive(Clone, Copy)]
union RawSso {
    small: Small,
    heap: Heap,
}

impl RawSso {
    const SSO_LEN: usize = std::mem::size_of::<usize>() * 2 - 1;

    const fn kind(&self) -> u8 {
        let meta = unsafe { self.small.meta.get() };
        meta >> u8::BITS - 2
    }

    const unsafe fn assume_small(self) -> Small {
        self.small
    }

    const unsafe fn assume_small_ref(&self) -> &Small {
        &self.small
    }

    unsafe fn assume_small_mut(&mut self) -> &mut Small {
        &mut self.small
    }

    const unsafe fn assume_heap(self) -> Heap {
        self.heap
    }

    const unsafe fn from_small_slice_unchecked(ptr: *const u8, len: usize) -> Self {
        if len > Self::SSO_LEN {
            std::hint::unreachable_unchecked();
        }

        let tagged = (len as u8) | SMALL << u8::BITS - 2;

        if std::mem::size_of::<Self>() == 16 {
            let register = if len > 8 {
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
            small.meta = NonZero::new_unchecked(tagged);
            Self { small }
        } else {
            let mut small = Small {
                data: [0; Self::SSO_LEN],
                meta: NonZero::new_unchecked(tagged),
            };

            let mut i = 0;
            while i < len {
                small.data[i] = *ptr.add(i);
                i += 1;
            }

            Self { small }
        }
    }

    const unsafe fn from_raw_parts(ptr: *const u8, len: usize, kind: u8) -> Self {
        if len <= Self::SSO_LEN {
            Self::from_small_slice_unchecked(ptr, len)
        } else {
            Self {
                heap: Heap {
                    data: ptr,
                    meta: NonZero::new_unchecked((kind as usize & 0b11) << usize::BITS - 1 | len),
                },
            }
        }
    }

    const unsafe fn len(self) -> usize {
        if self.kind() == SMALL {
            let meta = self.assume_small().meta.get();
            ((meta << 2) >> 2) as usize
        } else {
            let meta = self.assume_heap().meta.get();
            (meta << 2) >> 2
        }
    }

    const unsafe fn as_slice(&self) -> &[u8] {
        if self.kind() == SMALL {
            &self.assume_small_ref().data
        } else {
            let Heap { data, .. } = self.assume_heap();
            std::slice::from_raw_parts(data, self.len())
        }
    }

    unsafe fn as_mut_slice(&mut self) -> &mut [u8] {
        if self.kind() == SMALL {
            &mut self.assume_small_mut().data
        } else {
            let Heap { data, .. } = self.assume_heap();
            std::slice::from_raw_parts_mut(data.cast_mut(), self.len())
        }
    }
}

pub struct Sso<'a> {
    raw: RawSso,
    _ph: PhantomData<&'a [u8]>,
}

impl<'a> Sso<'a> {
    pub const fn borrowed(data: &'a str) -> Self {
        let len = data.len();
        let ptr = data.as_ptr();
        let raw = unsafe { RawSso::from_raw_parts(ptr, len, BORROWED) };
        Self {
            raw,
            _ph: PhantomData,
        }
    }

    const fn owned_inner(data: Box<str>) -> Sso<'static> {
        let len = data.len();
        let ptr = data.as_ptr();
        std::mem::forget(data);
        let raw = unsafe { RawSso::from_raw_parts(ptr, len, HEAP) };
        Sso {
            raw,
            _ph: PhantomData,
        }
    }

    pub const fn from_boxed_str(data: Box<str>) -> Sso<'static> {
        Self::owned_inner(data)
    }

    pub fn from_string(data: String) -> Sso<'static> {
        Self::owned_inner(data.into_boxed_str())
    }

    pub const fn as_bytes(&self) -> &[u8] {
        unsafe { self.raw.as_slice() }
    }

    pub const fn len(&self) -> usize {
        unsafe { self.raw.len() }
    }

    pub const fn borrow<'b>(&'b self) -> Sso<'b>
    where
        'a: 'b,
    {
        unsafe { Sso::from_ascii_unchecked(self.as_bytes()) }
    }

    pub const unsafe fn from_ascii_unchecked(data: &'a [u8]) -> Self {
        let ptr = data.as_ptr();
        let len = data.len();
        let raw = RawSso::from_raw_parts(ptr, len, BORROWED);
        Self {
            raw,
            _ph: PhantomData,
        }
    }

    pub const fn as_str(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(self.as_bytes()) }
    }
}

impl<'a> Source for Sso<'a> {
    #[cfg(debug_assertions)]
    fn length(&self) -> usize {
        self.len()
    }

    unsafe fn reconstruct_from_start_end_unchecked(&self, start: u16, end: u16) -> &str {
        std::str::from_utf8_unchecked(self.as_bytes().get_unchecked(start as usize..end as usize))
    }
}

impl<'a> Debug for Sso<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            f.write_str(self.as_str())
        } else {
            f.debug_struct("Sso<'_>")
                .field("value", &self.as_str())
                .field("len", &self.len())
                .finish()
        }
    }
}

impl Drop for Sso<'_> {
    fn drop(&mut self) {
        unsafe {
            if self.raw.kind() == HEAP {
                let layout = Layout::array::<u8>(self.len()).unwrap();
                std::ptr::drop_in_place(self.raw.as_mut_slice());
                std::alloc::dealloc(self.raw.assume_heap().data as *mut u8, layout);
            }
        }
    }
}

#[test]
#[cfg(test)]
fn test_sso_has_niche() {
    assert!(std::mem::size_of::<RawSso>() == std::mem::size_of::<Option<RawSso>>())
}
