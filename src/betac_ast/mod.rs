use pproc::{Tag, Tags};

use crate::{
    betac_runner::fx_hasher::FxHashMap,
    betac_util::{
        linked_list::{Link, LinkedList, Pointers},
        ptr::Ptr,
        small_vec::SmallVec,
    },
};

use core::fmt;
use std::{
    fmt::Debug,
    pin::Pin,
    ptr::NonNull,
    rc::Rc,
    sync::{
        atomic::{AtomicU16, AtomicU8, Ordering},
        LazyLock, Mutex,
    },
};

pub mod assignment;
pub mod pproc;

static SYNTAX_TREE_LISTS: LazyLock<Mutex<FxHashMap<u16, SyntaxTree>>> =
    LazyLock::new(|| Mutex::new(FxHashMap::default()));

static TAG_LISTS: LazyLock<Mutex<FxHashMap<u16, Tags>>> =
    LazyLock::new(|| Mutex::new(FxHashMap::default()));

static TREE_COUNT: AtomicU16 = AtomicU16::new(1);
static TAG_COUNT: AtomicU16 = AtomicU16::new(1);

pub(crate) fn register_new_list() -> u16 {
    let count = TREE_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    let mut lock = SYNTAX_TREE_LISTS.lock().unwrap();
    let _ = lock.insert(count, SyntaxTree::new());
    count
}

pub(crate) fn register_new_tag() -> u16 {
    let count = TAG_COUNT.fetch_add(1, Ordering::SeqCst);
    let mut lock = TAG_LISTS.lock().unwrap();
    let _ = lock.insert(count, SmallVec::new());
    count
}

pub(crate) fn with_syntax_list<F, R>(key: u16, f: F) -> Option<R>
where
    F: FnOnce(&mut SyntaxTree) -> R,
{
    let mut lock = SYNTAX_TREE_LISTS.lock().ok()?;
    let list = lock.get_mut(&key)?;
    let r = f(list);
    drop(lock);
    Some(r)
}

pub(crate) fn with_tags<F, R>(key: u16, f: F) -> Option<R>
where
    F: FnOnce(&mut Tags) -> R,
{
    let mut lock = TAG_LISTS.lock().ok()?;
    let list = lock.get_mut(&key)?;
    let r = f(list);
    drop(lock);
    Some(r)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Metadata(u8);

impl Metadata {
    pub const STATIC: u8 = 1 << 0;
}

pub type SyntaxTree = LinkedList<AstToken, <AstToken as Link>::Target>;

pub struct AstToken {
    data: Ptr<dyn AstNode>,
    pointers: Pointers<AstToken>,
}

unsafe impl Link for AstToken {
    type Handle = Pin<Rc<AstToken>>;
    type Target = AstToken;

    unsafe fn from_raw(target: std::ptr::NonNull<Self::Target>) -> Self::Handle {
        Pin::new_unchecked(Rc::from_raw(target.as_ptr()))
    }

    unsafe fn pointers(target: std::ptr::NonNull<Self::Target>) -> Pointers<Self::Target> {
        (*target.as_ptr()).pointers
    }

    fn as_raw(handle: &Self::Handle) -> std::ptr::NonNull<Self::Target> {
        let handle = Pin::clone(handle);
        let ptr = Rc::into_raw(unsafe { Pin::into_inner_unchecked(handle) });
        unsafe { NonNull::new_unchecked(ptr as *mut _) }
    }
}

impl AstToken {
    pub fn new(metadata: Ptr<dyn AstNode>) -> Pin<Rc<Self>> {
        unsafe {
            Pin::new_unchecked(Rc::new(Self {
                data: metadata,
                pointers: Pointers::new(),
            }))
        }
    }
}

pub trait AstNode: fmt::Debug + Send {
    fn start_pos(&self) -> u16 {
        self.span().start_pos
    }

    fn span(&self) -> Span;

    fn metadata(&self) -> Option<Metadata>;

    fn has_child_nodes(&self) -> bool {
        false
    }

    fn node_key(&self) -> Option<u16> {
        None
    }

    fn is_dummy(&self) -> bool {
        false
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    pub start_pos: u16,
    pub len: u8,
    pub meta: Metadata,
}

impl Span {
    pub const DUMMY: Self = Self {
        start_pos: 0,
        len: 0,
        meta: Metadata(0),
    };
}

impl Debug for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if *self == Span::DUMMY {
            f.write_str("<dummy>")
        } else {
            f.debug_struct("Span")
                .field("start_pos", &self.start_pos)
                .field("len", &self.len)
                .field("meta", &self.meta)
                .finish()
        }
    }
}

pub struct NoOp;

impl fmt::Debug for NoOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<no-op>")
    }
}

impl AstNode for NoOp {
    fn span(&self) -> Span {
        Span::DUMMY
    }

    fn metadata(&self) -> Option<Metadata> {
        None
    }

    fn is_dummy(&self) -> bool {
        true
    }
}

pub struct AtomicMetadata(AtomicU8);

impl AtomicMetadata {
    pub fn get() -> &'static Self {
        static ATOMIC: AtomicMetadata = AtomicMetadata(AtomicU8::new(0));
        &ATOMIC
    }

    pub fn add_flag(&'static self, flag: u8) {
        self.0.fetch_or(flag, Ordering::SeqCst);
    }

    pub fn to_metadata(&'static self) -> Metadata {
        Metadata(self.0.load(Ordering::Acquire))
    }
}
