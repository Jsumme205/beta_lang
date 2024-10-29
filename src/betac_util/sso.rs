use std::num::NonZero;

const SMALL: u8 = 1;
const STATIC: u8 = 2;
const HEAP: u8 = 3;

#[repr(C)]
#[derive(Clone, Copy)]
struct Small {
    data: [u8; 15],
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

#[test]
#[cfg(test)]
fn test_sso_has_niche() {
    assert!(std::mem::size_of::<RawSso>() == std::mem::size_of::<Option<RawSso>>())
}
