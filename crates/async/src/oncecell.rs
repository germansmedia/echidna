pub mod unsync {

    use core::{
        cell::{
            Cell,
            UnsafeCell,
        },
        fmt,
        mem,
        ops::{
            Deref,
            DerefMut,
        },
    };

    use std::panic::{
        RefUnwindSafe,
        UnwindSafe,
    };

    pub struct OnceCell<T> {
        inner: UnsafeCell<Option<T>>,
    }

    impl<T: RefUnwindSafe + UnwindSafe> RefUnwindSafe for OnceCell<T> {}

    impl<T: UnwindSafe> UnwindSafe for OnceCell<T> {}

    impl<T> Default for OnceCell<T> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<T: fmt::Debug> fmt::Debug for OnceCell<T> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self.get() {
                Some(v) => f.debug_tuple("OnceCell").field(v).finish(),
                None => f.write_str("OnceCell(Uninit)"),
            }
        }
    }

    impl<T: Clone> Clone for OnceCell<T> {
        fn clone(&self) -> OnceCell<T> {
            let res = OnceCell::new();
            if let Some(value) = self.get() {
                match res.set(value.clone()) {
                    Ok(()) => (),
                    Err(_) => unreachable!(),
                }
            }
            res
        }
    }

    impl<T: PartialEq> PartialEq for OnceCell<T> {
        fn eq(&self, other: &Self) -> bool {
            self.get() == other.get()
        }
    }

    impl<T: Eq> Eq for OnceCell<T> { }

    impl<T> From<T> for OnceCell<T> {
        fn from(value: T) -> Self {
            OnceCell { inner: UnsafeCell::new(Some(value)) }
        }
    }

    impl<T> OnceCell<T> {
        pub const fn new() -> OnceCell<T> {
            OnceCell { inner: UnsafeCell::new(None) }
        }

        pub fn get(&self) -> Option<&T> {
            unsafe { &*self.inner.get() }.as_ref()
        }

        pub fn get_mut(&mut self) -> Option<&mut T> {
            unsafe { &mut *self.inner.get() }.as_mut()
        }

        pub fn set(&self, value: T) -> Result<(), T> {
            let slot = unsafe { &*self.inner.get() };
            if slot.is_some() {
                return Err(value);
            }
            let slot = unsafe { &mut *self.inner.get() };
            *slot = Some(value);
            Ok(())
        }

        pub fn get_or_init<F>(&self, f: F) -> &T where F: FnOnce() -> T,
        {
            enum Void {}
            match self.get_or_try_init(|| Ok::<T, Void>(f())) {
                Ok(val) => val,
                Err(void) => match void {},
            }
        }

        pub fn get_or_try_init<F, E>(&self, f: F) -> Result<&T, E> where F: FnOnce() -> Result<T, E>,
        {
            if let Some(val) = self.get() {
                return Ok(val);
            }
            let val = f()?;
            assert!(self.set(val).is_ok(), "reentrant init");
            Ok(self.get().unwrap())
        }

        pub fn take(&mut self) -> Option<T> {
            mem::replace(self, Self::default()).into_inner()
        }

        pub fn into_inner(self) -> Option<T> {
            self.inner.into_inner()
        }
    }

    pub struct Lazy<T,F = fn() -> T> {
        cell: OnceCell<T>,
        init: Cell<Option<F>>,
    }

    impl<T,F: RefUnwindSafe> RefUnwindSafe for Lazy<T,F> where OnceCell<T>: RefUnwindSafe {}

    impl<T:fmt::Debug, F> fmt::Debug for Lazy<T,F> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.debug_struct("Lazy").field("cell", &self.cell).field("init", &"..").finish()
        }
    }

    impl<T,F> Lazy<T,F> {
        pub const fn new(init: F) -> Lazy<T,F> {
            Lazy { cell: OnceCell::new(), init: Cell::new(Some(init)) }
        }

        pub fn into_value(this: Lazy<T,F>) -> Result<T,F> {
            let cell = this.cell;
            let init = this.init;
            cell.into_inner().ok_or_else(|| {
                init.take().unwrap_or_else(|| panic!("Lazy instance has previously been poisoned"))
            })
        }
    }

    impl<T,F: FnOnce() -> T> Lazy<T,F> {
        pub fn force(this: &Lazy<T,F>) -> &T {
            this.cell.get_or_init(|| match this.init.take() {
                Some(f) => f(),
                None => panic!("Lazy instance has previously been poisoned"),
            })
        }
    }

    impl<T,F: FnOnce() -> T> Deref for Lazy<T,F> {
        type Target = T;
        fn deref(&self) -> &T {
            Lazy::force(self)
        }
    }

    impl<T,F: FnOnce() -> T> DerefMut for Lazy<T,F> {
        fn deref_mut(&mut self) -> &mut T {
            Lazy::force(self);
            self.cell.get_mut().unwrap_or_else(|| unreachable!())
        }
    }

    impl<T: Default> Default for Lazy<T> {
        fn default() -> Lazy<T> {
            Lazy::new(T::default)
        }
    }
}

pub mod sync {
    use std::{
        cell::Cell,
        fmt,
        mem,
        ops::{
            Deref,
            DerefMut,
        },
        panic::RefUnwindSafe,
    };
    use crate::imp::OnceCell as Imp;

    pub struct OnceCell<T>(Imp<T>);

    impl<T> Default for OnceCell<T> {
        fn default() -> OnceCell<T> {
            OnceCell::new()
        }
    }

    impl<T: fmt::Debug> fmt::Debug for OnceCell<T> {
        fn fmt(&self,f: &mut fmt::Formatter) -> fmt::Result {
            match self.get() {
                Some(v) => f.debug_tuple("OnceCell").field(v).finish(),
                None => f.write_str("OnceCell(Uninit)"),
            }
        }
    }

    impl<T: Clone> Clone for OnceCell<T> {
        fn clone(&self) -> OnceCell<T> {
            let res = OnceCell::new();
            if let Some(value) = self.get() {
                match res.set(value.clone()) {
                    Ok(()) => (),
                    Err(_) => unreachable!(),
                }
            }
            res
        }
    }

    impl<T> From<T> for OnceCell<T> {
        fn from(value: T) -> Self {
            let cell = Self::new();
            cell.get_or_init(|| value);
            cell
        }
    }

    impl<T: PartialEq> PartialEq for OnceCell<T> {
        fn eq(&self, other: &OnceCell<T>) -> bool {
            self.get() == other.get()
        }
    }

    impl<T: Eq> Eq for OnceCell<T> {}

    impl<T> OnceCell<T> {
        pub const fn new() -> OnceCell<T> {
            OnceCell(Imp::new())
        }

        pub fn get(&self) -> Option<&T> {
            if self.0.is_initialized() {
                Some(unsafe { self.get_unchecked() })
            } else {
                None
            }
        }

        pub fn get_mut(&mut self) -> Option<&mut T> {
            self.0.get_mut()
        }

        pub unsafe fn get_unchecked(&self) -> &T {
            self.0.get_unchecked()
        }

        pub fn set(&self,value: T) -> Result<(), T> {
            let mut value = Some(value);
            self.get_or_init(|| value.take().unwrap());
            match value {
                None => Ok(()),
                Some(value) => Err(value),
            }
        }

        pub fn get_or_init<F>(&self,f: F) -> &T where F: FnOnce() -> T {
            enum Void {}
            match self.get_or_try_init(|| Ok::<T, Void>(f())) {
                Ok(val) => val,
                Err(void) => match void {},
            }
        }

        pub fn get_or_try_init<F,E>(&self,f: F) -> Result<&T,E> where F: FnOnce() -> Result<T,E> {
            if let Some(value) = self.get() {
                return Ok(value);
            }
            self.0.initialize(f)?;
            debug_assert!(self.0.is_initialized());
            Ok(unsafe { self.get_unchecked() })
        }

        pub fn take(&mut self) -> Option<T> {
            mem::replace(self, Self::default()).into_inner()
        }

        pub fn into_inner(self) -> Option<T> {
            self.0.into_inner()
        }
    }

    pub struct Lazy<T,F = fn() -> T> {
        cell: OnceCell<T>,
        init: Cell<Option<F>>,
    }

    impl<T: fmt::Debug,F> fmt::Debug for Lazy<T,F> {
        fn fmt(&self,f: &mut fmt::Formatter) -> fmt::Result {
            f.debug_struct("Lazy").field("cell", &self.cell).field("init", &"..").finish()
        }
    }

    unsafe impl<T,F: Send> Sync for Lazy<T,F> where OnceCell<T>: Sync { }

    impl<T,F: RefUnwindSafe> RefUnwindSafe for Lazy<T,F> where OnceCell<T>: RefUnwindSafe { }

    impl<T,F> Lazy<T,F> {
        pub const fn new(f: F) -> Lazy<T,F> {
            Lazy {
                cell: OnceCell::new(),
                init: Cell::new(Some(f)),
            }
        }

        pub fn into_value(this: Lazy<T,F>) -> Result<T,F> {
            let cell = this.cell;
            let init = this.init;
            cell.into_inner().ok_or_else(|| {
                init.take().unwrap_or_else(|| panic!("Lazy instance has previously been poisoned"))
            })
        }
    }

    impl<T,F: FnOnce() -> T> Lazy<T,F> {
        pub fn force(this: &Lazy<T,F>) -> &T {
            this.cell.get_or_init(|| match this.init.take() {
                Some(f) => f(),
                None => panic!("Lazy instance has previously been poisoned"),
            })
        }
    }

    impl<T,F: FnOnce() -> T> Deref for Lazy<T,F> {
        type Target = T;
        fn deref(&self) -> &T {
            Lazy::force(self)
        }
    }

    impl<T,F: FnOnce() -> T> DerefMut for Lazy<T,F> {
        fn deref_mut(&mut self) -> &mut T {
            Lazy::force(self);
            self.cell.get_mut().unwrap_or_else(|| unreachable!())
        }
    }

    impl<T: Default> Default for Lazy<T> {
        fn default() -> Lazy<T> {
            Lazy::new(T::default)
        }
    }

    fn _dummy() {
    }
}

unsafe fn take_unchecked<T>(val: &mut Option<T>) -> T {
    match val.take() {
        Some(it) => it,
        None => {
            debug_assert!(false);
            std::hint::unreachable_unchecked()
        }
    }
}