use derive_more::Constructor;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Constructor)]
pub struct Position {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Hash, Constructor)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

impl Span {
    pub fn contains(&self, pos: Position) -> bool {
        pos >= self.start && pos < self.end
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Constructor)]
pub struct Spanned<T> {
    pub span: Span,
    pub val: T,
}

impl<T> Spanned<T> {
    pub fn map<F, O>(self, f: F) -> Spanned<O>
    where
        F: FnOnce(T) -> O,
    {
        Spanned::new(self.span, f(self.val))
    }

    pub fn merge<T2, F, O>(self, s2: Spanned<T2>, f: F) -> Spanned<O>
    where
        F: FnOnce(Spanned<T>, Spanned<T2>) -> O,
    {
        let tot_span = self.span.combine(&s2.span);
        Spanned::new(tot_span, f(self, s2))
    }
}

impl<T, E> Spanned<Result<T, E>> {
    pub fn transpose(self) -> Result<Spanned<T>, E> {
        self.val.map(|val| Spanned::new(self.span, val))
    }
}

pub trait Combine {
    fn combine(&self, other: &Self) -> Self;
}

impl Combine for Span {
    fn combine(&self, other: &Self) -> Self {
        let start = Ord::min(self.start, other.start);
        let end = Ord::max(self.end, other.end);
        Self::new(start, end)
    }
}
