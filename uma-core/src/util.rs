use std::ops::Range;

use derive_more::Constructor;

#[derive(Debug, Clone, PartialEq, Eq, Constructor)]
pub struct Spanned<T> {
    pub span: Range<usize>,
    pub val: T,
}

impl<T> Spanned<T> {
    pub fn map<F, O>(self, f: F) -> Spanned<O>
    where
        F: FnOnce(T) -> O,
    {
        Spanned::new(self.span, f(self.val))
    }
}

impl<T, E> Spanned<Result<T, E>> {
    pub fn transpose(self) -> Result<Spanned<T>, E> {
        self.val.map(|val| Spanned::new(self.span, val))
    }
}

pub trait Combine {
    fn combine(self, other: Self) -> Self;
}

impl<T: Ord> Combine for Range<T> {
    fn combine(self, other: Self) -> Self {
        let start = Ord::min(self.start, other.start);
        let end = Ord::max(self.end, other.end);
        start..end
    }
}
