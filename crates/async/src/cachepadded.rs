use {
    core::{
        fmt,
        ops::{
            Deref,
            DerefMut,
        },
    },
};

#[cfg_attr(any(target_arch="x86_64",target_arch="aarch64"),repr(align(128)))]
#[cfg_attr(not(any(target_arch="x86_64",target_arch="aarch64")),repr(align(64)))]

#[derive(Clone,Copy,Default,Hash,PartialEq,Eq)]
pub struct CachePadded<T>(T);

impl<T> CachePadded<T> {
    pub const fn new(t: T) -> CachePadded<T> {
        CachePadded(t)
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for CachePadded<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> DerefMut for CachePadded<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T: fmt::Debug> fmt::Debug for CachePadded<T> {
    fn fmt(&self,f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CachePadded").field(&self.0).finish()
    }
}

impl<T> From<T> for CachePadded<T> {
    fn from(t: T) -> Self {
        CachePadded::new(t)
    }
}
