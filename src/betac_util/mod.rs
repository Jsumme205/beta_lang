pub mod alloc;
pub mod linked_list;

pub mod ptr;
pub mod small_vec;
pub mod sso;

use std::sync::Mutex;

static CRIT_SECTION: Mutex<()> = Mutex::new(());

pub fn enter_critical_section<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let _lock = CRIT_SECTION.lock().unwrap();
    let r = f();
    drop(_lock);
    r
}

pub fn from_fn<F, T>(f: F) -> FromFn<F>
where
    F: FnMut() -> Option<T> + Clone,
    T: Clone,
{
    FromFn(f)
}

pub struct FromFn<F>(F);

impl<T, F> Clone for FromFn<F>
where
    F: FnMut() -> Option<T> + Clone,
    T: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T, F> Iterator for FromFn<F>
where
    F: FnMut() -> Option<T> + Clone,
    T: Clone,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        (self.0)()
    }
}

pub struct TakeWhile<I, P> {
    iter: I,
    flag: bool,
    pred: P,
}

impl<I, P> Clone for TakeWhile<I, P>
where
    I: Clone,
    P: Clone,
{
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
            flag: self.flag,
            pred: self.pred.clone(),
        }
    }
}

impl<I, P> Iterator for TakeWhile<I, P>
where
    I: Iterator + Clone,
    P: FnMut(&I::Item) -> bool + Clone,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.flag {
            None
        } else {
            let x = self.iter.next()?;
            if (self.pred)(&x) {
                Some(x)
            } else {
                self.flag = true;
                None
            }
        }
    }
}

#[derive(Clone)]
pub struct Take<I> {
    iter: I,
    n: usize,
}

impl<I> Take<I> {
    pub fn into_inner(self) -> I {
        self.iter
    }
}

impl<I> Iterator for Take<I>
where
    I: Clone + Iterator,
{
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<<I as Iterator>::Item> {
        if self.n != 0 {
            self.n -= 1;
            self.iter.next()
        } else {
            None
        }
    }
}

pub trait IterExt: Iterator + Sized {
    fn clonable_take_while<P>(self, pred: P) -> self::TakeWhile<Self, P>
    where
        P: Clone + FnMut(&Self::Item) -> bool,
    {
        TakeWhile {
            iter: self,
            flag: false,
            pred,
        }
    }

    fn cloneable_take(self, n: usize) -> Take<Self> {
        Take { iter: self, n }
    }
}

impl<I> IterExt for I where I: Iterator + Clone {}

macro_rules! debug_dbg_impl {
    ($val:expr $(,)?) => {
        #[cfg(debug_assertions)]
        ::std::dbg!($val)
    };
}

pub(crate) use debug_dbg_impl as ddbg;

#[macro_export]
macro_rules! catch {
    (dbg $caught:ident) => {{
        println!("caught: {:?}", $caught);
        todo!()
    }};
    ($caught:ident) => {{
        println!("caught: {}", $caught);
        todo!()
    }};
    (tok $caught:ident, $this:expr) => {{
        println!("caught token: {:?}", $caught);
        if let Some(next) = $this.peek() {
            let substr = unsafe {
                $this
                    .source
                    .reconstruct_from_start_end_unchecked($caught.start, next.start)
            };
            println!("substring: {}", substr);
        }
        todo!()
    }};
}
