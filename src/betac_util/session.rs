use std::{
    collections::HashMap,
    hash::BuildHasherDefault,
    hash::Hasher,
    ops::BitXor,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

use super::sso::OwnedYarn;
use crate::{
    betac_lexer::ast_types::{context::PackageContext, Ty},
    Globals,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Options(usize);

impl Options {
    pub const INCLUDE_WARNINGS_AS_ERRORS: Self = Self(1 << 0);
}

#[derive(Debug)]
pub struct Session {
    pub globals: crate::Globals,
    pub contents: OwnedYarn,
    pub out_dir: Option<PathBuf>,
    pub options: usize,
    pub type_ids: AtomicUsize,
    pub types_hashmap: HashMap<usize, OwnedYarn, BuildHasherDefault<FxHasher>>,
    pub package_context: PackageContext,
}

pub struct FxHasher {
    hash: usize,
}

#[cfg(target_pointer_width = "32")]
const K: usize = 0x9e3779b9;
#[cfg(target_pointer_width = "64")]
const K: usize = 0x517cc1b727220a95;

impl Default for FxHasher {
    #[inline]
    fn default() -> FxHasher {
        FxHasher { hash: 0 }
    }
}

impl FxHasher {
    #[inline]
    fn add_to_hash(&mut self, i: usize) {
        self.hash = self.hash.rotate_left(5).bitxor(i).wrapping_mul(K);
    }
}

impl Hasher for FxHasher {
    #[inline]
    fn write(&mut self, mut bytes: &[u8]) {
        #[cfg(target_pointer_width = "32")]
        let read_usize = |bytes: &[u8]| u32::from_ne_bytes(bytes[..4].try_into().unwrap());
        #[cfg(target_pointer_width = "64")]
        let read_usize = |bytes: &[u8]| u64::from_ne_bytes(bytes[..8].try_into().unwrap());

        let mut hash = FxHasher { hash: self.hash };
        assert!(size_of::<usize>() <= 8);
        while bytes.len() >= size_of::<usize>() {
            hash.add_to_hash(read_usize(bytes) as usize);
            bytes = &bytes[size_of::<usize>()..];
        }
        if (size_of::<usize>() > 4) && (bytes.len() >= 4) {
            hash.add_to_hash(u32::from_ne_bytes(bytes[..4].try_into().unwrap()) as usize);
            bytes = &bytes[4..];
        }
        if (size_of::<usize>() > 2) && bytes.len() >= 2 {
            hash.add_to_hash(u16::from_ne_bytes(bytes[..2].try_into().unwrap()) as usize);
            bytes = &bytes[2..];
        }
        if (size_of::<usize>() > 1) && bytes.len() >= 1 {
            hash.add_to_hash(bytes[0] as usize);
        }
        self.hash = hash.hash;
    }

    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.add_to_hash(i as usize);
    }

    #[inline]
    fn write_u16(&mut self, i: u16) {
        self.add_to_hash(i as usize);
    }

    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.add_to_hash(i as usize);
    }

    #[cfg(target_pointer_width = "32")]
    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.add_to_hash(i as usize);
        self.add_to_hash((i >> 32) as usize);
    }

    #[cfg(target_pointer_width = "64")]
    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.add_to_hash(i as usize);
    }

    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.add_to_hash(i);
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.hash as u64
    }
}

pub type BuildFxHasher = BuildHasherDefault<FxHasher>;

pub struct SessionBuilder {
    contents: Option<OwnedYarn>,
    in_file: Option<PathBuf>,
    out_file: Option<PathBuf>,
    options: usize,
}

impl SessionBuilder {
    pub fn input<P>(mut self, path: P) -> Self
    where
        P: AsRef<Path>,
    {
        self.in_file = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn output_to<P>(mut self, path: P) -> Self
    where
        P: AsRef<Path>,
    {
        self.out_file = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn add_option(mut self, opt: Options) -> Self {
        self.options |= opt.0;
        self
    }

    pub fn build(self) -> super::CompileResult<Session> {
        let in_path = self.in_file.unwrap();
        let contents: OwnedYarn = std::fs::read_to_string(in_path)?.into();
        Ok(Session {
            globals: Globals::new(),
            contents,
            out_dir: self.out_file,
            //span_interner: Mutex::new(SpanInterner {}),
            options: self.options,
            type_ids: AtomicUsize::new(Ty::OFFSET_FROM_BUILTIN),
            types_hashmap: HashMap::with_hasher(BuildHasherDefault::default()),
            package_context: PackageContext::init(),
        })
    }
}

impl Session {
    pub const fn builder() -> SessionBuilder {
        SessionBuilder {
            contents: None,
            out_file: None,
            in_file: None,
            options: 0,
        }
    }

    pub const fn has_flag_no_warnings_set(&self) -> bool {
        self.options & Options::INCLUDE_WARNINGS_AS_ERRORS.0 != 0
    }

    pub fn get_next_type_id(&self) -> usize {
        self.type_ids.fetch_add(1, Ordering::Acquire)
    }

    pub fn register(&mut self, k: usize, v: OwnedYarn) {
        let _ = self.types_hashmap.insert(k, v);
    }
}
