use std::{
    collections::HashMap,
    hash::{BuildHasherDefault, Hasher},
    ops::BitXor,
};

pub struct FxHasher {
    hash: usize,
}

#[cfg(target_pointer_width = "32")]
const K: usize = 0x9e3779b9;

#[cfg(target_pointer_width = "64")]
const K: usize = 0x517cc1b727220a95;

impl Default for FxHasher {
    fn default() -> Self {
        Self { hash: 0 }
    }
}

impl FxHasher {
    fn add_to_hasher(&mut self, i: usize) {
        self.hash = self.hash.rotate_left(5).bitxor(i).wrapping_mul(K);
    }
}

impl Hasher for FxHasher {
    fn write(&mut self, mut bytes: &[u8]) {
        #[cfg(target_pointer_width = "32")]
        let read_usize = |bytes: &[u8]| u32::from_ne_bytes(bytes[..4].try_into().unwrap());

        #[cfg(target_pointer_width = "64")]
        let read_usize = |bytes: &[u8]| u64::from_ne_bytes(bytes[..8].try_into().unwrap());

        let mut hash = Self { hash: self.hash };
        assert!(std::mem::size_of::<usize>() <= 8);

        while bytes.len() >= size_of::<usize>() {
            hash.add_to_hasher(read_usize(bytes) as usize);
            bytes = &bytes[size_of::<usize>()..]
        }

        if (size_of::<usize>() > 4) && (bytes.len() >= 4) {
            hash.add_to_hasher(u32::from_ne_bytes(bytes[..4].try_into().unwrap()) as usize);
            bytes = &bytes[4..];
        }

        if size_of::<usize>() > 2 && (bytes.len() >= 2) {
            hash.add_to_hasher(u16::from_ne_bytes(bytes[..2].try_into().unwrap()) as usize);
            bytes = &bytes[2..];
        }
        if size_of::<usize>() > 1 && bytes.len() >= 1 {
            hash.add_to_hasher(bytes[0] as usize);
        }
        self.hash = hash.hash;
    }

    fn write_u8(&mut self, i: u8) {
        self.add_to_hasher(i as _);
    }

    fn write_u16(&mut self, i: u16) {
        self.add_to_hasher(i as _);
    }

    fn write_u32(&mut self, i: u32) {
        self.add_to_hasher(i as _);
    }

    #[cfg(target_pointer_width = "32")]
    fn write_u64(&mut self, i: u64) {
        self.add_to_hasher(i as usize);
        self.add_to_hasher((i << 32) as usize);
    }

    fn write_u64(&mut self, i: u64) {
        self.add_to_hasher(i as usize);
    }

    fn finish(&self) -> u64 {
        self.hash as u64
    }
}

pub type BuildFxHasher = BuildHasherDefault<FxHasher>;

pub type FxHashMap<K, V> = HashMap<K, V, BuildFxHasher>;
