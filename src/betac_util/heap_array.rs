use std::{
    fmt::Debug,
    io,
    ops::{self, Index},
    ptr::NonNull,
    slice::SliceIndex,
    usize,
};

pub unsafe trait Array {
    type Output;
    const LEN: usize;

    fn bytes() -> usize {
        std::mem::size_of::<Self::Output>() * Self::LEN
    }
}

unsafe impl<T, const N: usize> Array for [T; N] {
    type Output = T;
    const LEN: usize = N;
}

unsafe impl<T> Array for [T] {
    type Output = T;

    const LEN: usize = 0;
}

pub enum TryPushError<T> {
    WouldOverflow(T),
    Other,
}

impl<T> Debug for TryPushError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Other => f.write_str("Other"),
            Self::WouldOverflow(_) => f.write_str("WouldOverflow"),
        }
    }
}

#[derive(Debug)]
pub enum HeapArrayError {
    LenIsUnexpectedlyZero,
    MmapFailure,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct LenCap {
    cap: u32,
    len: u32,
}

#[derive(Clone, Copy)]
union LenUnion {
    len: usize,
    len_cap: LenCap,
}

impl LenUnion {
    fn len<A: Array + ?Sized>(self) -> usize {
        if A::LEN != 0 {
            unsafe { self.len }
        } else {
            unsafe { self.len_cap.len as usize }
        }
    }

    fn inc_len<A: Array + ?Sized>(&mut self) {
        if A::LEN != 0 {
            unsafe { self.len += 1 }
        } else {
            unsafe { self.len_cap.len += 1 }
        }
    }

    fn dec_len<A: Array + ?Sized>(&mut self) {
        if A::LEN != 0 {
            unsafe { self.len -= 1 }
        } else {
            unsafe { self.len_cap.len -= 1 }
        }
    }

    fn cap_len<A: Array + ?Sized>(self) -> (usize, usize) {
        if A::LEN != 0 {
            (A::LEN, unsafe { self.len })
        } else {
            let this = unsafe { self.len_cap };
            (this.cap as usize, this.len as usize)
        }
    }
}

/// non-growable array type for allocation up front
/// this is great for
pub struct HeapArray<A: Array + ?Sized> {
    ptr: NonNull<A::Output>,
    len: LenUnion,
}

impl<A: Array> HeapArray<A> {
    pub fn new() -> Result<Self, HeapArrayError> {
        if A::LEN == 0 {
            return Err(HeapArrayError::LenIsUnexpectedlyZero);
        }
        let ptr = unsafe {
            match libc::mmap(
                std::ptr::null_mut(),
                A::bytes().next_power_of_two(),
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_ANON | libc::MAP_PRIVATE,
                -1,
                0,
            ) {
                libc::MAP_FAILED => return Err(HeapArrayError::MmapFailure),
                ptr => NonNull::new_unchecked(ptr as *mut A::Output),
            }
        };
        Ok(Self {
            ptr,
            len: LenUnion { len: 0 },
        })
    }
}

impl<A: Array + ?Sized> HeapArray<A> {
    pub fn with_cap(cap: u32) -> Result<Self, HeapArrayError> {
        let len = cap as usize * std::mem::size_of::<A::Output>();
        let ptr = unsafe {
            match libc::mmap(
                std::ptr::null_mut(),
                len.next_power_of_two(),
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_ANON | libc::MAP_PRIVATE,
                -1,
                0,
            ) {
                libc::MAP_FAILED => return Err(HeapArrayError::MmapFailure),
                ptr => NonNull::new_unchecked(ptr.cast::<A::Output>()),
            }
        };
        Ok(Self {
            ptr,
            len: LenUnion {
                len_cap: LenCap { cap, len: 0 },
            },
        })
    }
}

impl<A: Array + ?Sized> HeapArray<A> {
    pub fn try_push(&mut self, element: A::Output) -> Result<(), TryPushError<A::Output>> {
        let (len, offset) = self.len.cap_len::<A>();
        if len <= offset {
            return Err(TryPushError::WouldOverflow(element));
        }
        let offseted_ptr = unsafe { self.ptr.add(offset) };
        unsafe { offseted_ptr.write(element) };
        unsafe {
            if A::LEN != 0 {
                self.len.len += 1;
            } else {
                self.len.len_cap.len += 1
            }
        }
        Ok(())
    }

    pub fn push(&mut self, element: A::Output) {
        self.try_push(element).expect("cannot push");
    }

    pub fn len(&self) -> usize {
        if A::LEN != 0 {
            unsafe { self.len.len }
        } else {
            unsafe { self.len.len_cap.len as usize }
        }
    }

    pub fn pop(&mut self) -> Option<A::Output> {
        if self.len.len::<A>() == 0 {
            None
        } else {
            unsafe {
                self.len.dec_len::<A>();
                let ptr = self.ptr.as_ptr();
                Some(std::ptr::read(ptr.add(self.len())))
            }
        }
    }

    pub fn capacity(&self) -> usize {
        if A::LEN != 0 {
            A::LEN
        } else {
            unsafe { self.len.len_cap.cap as usize }
        }
    }
}

impl<A: Array + ?Sized> ops::Deref for HeapArray<A> {
    type Target = [A::Output];

    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len()) }
    }
}

impl<A: Array + ?Sized> ops::DerefMut for HeapArray<A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len()) }
    }
}

impl<A: Array + ?Sized> Drop for HeapArray<A> {
    fn drop(&mut self) {
        unsafe {
            std::ptr::drop_in_place(std::ptr::slice_from_raw_parts_mut(
                self.ptr.as_ptr(),
                self.len(),
            ));
            libc::munmap(
                self.ptr.as_ptr() as *mut _,
                (self.capacity() * std::mem::size_of::<A::Output>()).next_power_of_two(),
            );
        }
    }
}

impl<'a, A: Array + ?Sized> IntoIterator for &'a HeapArray<A> {
    type Item = &'a A::Output;
    type IntoIter = std::slice::Iter<'a, A::Output>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, A: Array + ?Sized> IntoIterator for &'a mut HeapArray<A> {
    type Item = &'a mut A::Output;
    type IntoIter = std::slice::IterMut<'a, A::Output>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<A: Array + ?Sized, I: SliceIndex<[A::Output]>> ops::Index<I> for HeapArray<A> {
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        ops::Index::index(&**self, index)
    }
}

impl<A: Array + ?Sized, I: SliceIndex<[A::Output]>> ops::IndexMut<I> for HeapArray<A> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        ops::IndexMut::index_mut(&mut **self, index)
    }
}

#[test]
#[cfg(test)]
fn test_has_niche() {
    assert!(
        std::mem::size_of::<HeapArray<[i32]>>() == std::mem::size_of::<Option<HeapArray<[i32]>>>()
    )
}
