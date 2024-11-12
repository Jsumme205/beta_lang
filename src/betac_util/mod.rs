pub mod alloc;
pub mod cell;
pub mod ptr;
pub mod small_vec;
pub mod sso;

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
