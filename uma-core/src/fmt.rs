use std::fmt;

use derive_more::Constructor;

pub trait DisplayWithSrc {
    fn fmt_with_src(&self, f: &mut fmt::Formatter<'_>, src: &str) -> fmt::Result;
}

#[derive(Debug, Clone, Constructor)]
pub struct WithSrc<'a, T> {
    value: &'a T,
    src: &'a str,
}

impl<'a, T: DisplayWithSrc> fmt::Display for WithSrc<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt_with_src(f, self.src)
    }
}

pub trait DisplayWithSrcExt: DisplayWithSrc {
    fn with_src<'a>(&'a self, src: &'a str) -> WithSrc<'a, Self>
    where
        Self: Sized,
    {
        WithSrc::new(self, src)
    }
}

impl<T: DisplayWithSrc> DisplayWithSrcExt for T {}
