#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_debug_implementations)]

use {
    crate::*,
    core::{
        future::Future,
        fmt,
        sync::atomic::Ordering,
        task::{
            Context,
            Poll,
        },
        mem,
        pin::Pin,
        ptr::NonNull,
        marker::{
            PhantomData,
            Unpin,
            Send,
            Sync,
        },
    },
    std::{
        panic::{
            UnwindSafe,
            RefUnwindSafe,
        },
        alloc::Layout,
    },
};

pub(crate) const SCHEDULED: usize = 1 << 0;
pub(crate) const RUNNING: usize = 1 << 1;
pub(crate) const COMPLETED: usize = 1 << 2;
pub(crate) const CLOSED: usize = 1 << 3;
pub(crate) const TASK: usize = 1 << 4;
pub(crate) const AWAITER: usize = 1 << 5;
pub(crate) const REGISTERING: usize = 1 << 6;
pub(crate) const NOTIFYING: usize = 1 << 7;
pub(crate) const REFERENCE: usize = 1 << 8;

pub struct Task<T> {
    pub(crate) ptr: NonNull<()>,
    pub(crate) _marker: PhantomData<T>,
}

unsafe impl<T: Send> Send for Task<T> { }

unsafe impl<T> Sync for Task<T> { }

impl<T> Unpin for Task<T> { }

impl<T> UnwindSafe for Task<T> { }

impl<T> RefUnwindSafe for Task<T> { }

impl<T> Task<T> {
    pub fn detach(self) {
        let mut this = self;
        let _out = this.set_detached();
        mem::forget(this);
    }

    pub async fn cancel(self) -> Option<T> {
        let mut this = self;
        this.set_canceled();
        struct Fut<T>(Task<T>);
        impl<T> Future for Fut<T> {
            type Output = Option<T>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                self.0.poll_task(cx)
            }
        }
        Fut(this).await
    }

    fn set_canceled(&mut self) {
        let ptr = self.ptr.as_ptr();
        let header = ptr as *const Header;
        unsafe {
            let mut state = (*header).state.load(Ordering::Acquire);
            loop {
                if state & (COMPLETED | CLOSED) != 0 {
                    break;
                }
                let new = if state & (SCHEDULED | RUNNING) == 0 {
                    (state | SCHEDULED | CLOSED) + REFERENCE
                } else {
                    state | CLOSED
                };
                match (*header).state.compare_exchange_weak(
                    state,
                    new,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(_) => {
                        if state & (SCHEDULED | RUNNING) == 0 {
                            ((*header).vtable.schedule)(ptr);
                        }
                        if state & AWAITER != 0 {
                            (*header).notify(None);
                        }
                        break;
                    }
                    Err(s) => state = s,
                }
            }
        }
    }

    fn set_detached(&mut self) -> Option<T> {
        let ptr = self.ptr.as_ptr();
        let header = ptr as *const Header;
        unsafe {
            let mut output = None;
            if let Err(mut state) = (*header).state.compare_exchange_weak(SCHEDULED | TASK | REFERENCE,SCHEDULED | REFERENCE,Ordering::AcqRel,Ordering::Acquire) {
                loop {
                    if ((state & COMPLETED) != 0) && ((state & CLOSED) == 0) {
                        match (*header).state.compare_exchange_weak(state,state | CLOSED,Ordering::AcqRel,Ordering::Acquire) {
                            Ok(_) => {
                                output = Some((((*header).vtable.get_output)(ptr) as *mut T).read());
                                state |= CLOSED;
                            }
                            Err(s) => state = s,
                        }
                    } else {
                        let new = if state & (!(REFERENCE - 1) | CLOSED) == 0 {
                            SCHEDULED | CLOSED | REFERENCE
                        } else {
                            state & !TASK
                        };
                        match (*header).state.compare_exchange_weak(state,new,Ordering::AcqRel,Ordering::Acquire) {
                            Ok(_) => {
                                if state & !(REFERENCE - 1) == 0 {
                                    if state & CLOSED == 0 {
                                        ((*header).vtable.schedule)(ptr);
                                    } else {
                                        ((*header).vtable.destroy)(ptr);
                                    }
                                }
                                break;
                            }
                            Err(s) => state = s,
                        }
                    }
                }
            }
            output
        }
    }

    fn poll_task(&mut self, cx: &mut Context<'_>) -> Poll<Option<T>> {
        let ptr = self.ptr.as_ptr();
        let header = ptr as *const Header;
        unsafe {
            let mut state = (*header).state.load(Ordering::Acquire);
            loop {
                if state & CLOSED != 0 {
                    if state & (SCHEDULED | RUNNING) != 0 {
                        (*header).register(cx.waker());
                        state = (*header).state.load(Ordering::Acquire);
                        if state & (SCHEDULED | RUNNING) != 0 {
                            return Poll::Pending;
                        }
                    }
                    (*header).notify(Some(cx.waker()));
                    return Poll::Ready(None);
                }
                if state & COMPLETED == 0 {
                    (*header).register(cx.waker());
                    state = (*header).state.load(Ordering::Acquire);
                    if state & CLOSED != 0 {
                        continue;
                    }
                    if state & COMPLETED == 0 {
                        return Poll::Pending;
                    }
                }
                match (*header).state.compare_exchange(state,state | CLOSED,Ordering::AcqRel,Ordering::Acquire) {
                    Ok(_) => {
                        if state & AWAITER != 0 {
                            (*header).notify(Some(cx.waker()));
                        }
                        let output = ((*header).vtable.get_output)(ptr) as *mut T;
                        return Poll::Ready(Some(output.read()));
                    }
                    Err(s) => state = s,
                }
            }
        }
    }
}

impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        self.set_canceled();
        self.set_detached();
    }
}

impl<T> Future for Task<T> {
    type Output = T;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.poll_task(cx) {
            Poll::Ready(t) => Poll::Ready(t.expect("task has failed")),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T> fmt::Debug for Task<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ptr = self.ptr.as_ptr();
        let header = ptr as *const Header;
        f.debug_struct("Task").field("header", unsafe { &(*header) }).finish()
    }
}

pub(crate) fn abort() -> ! {
    struct Panic;
    impl Drop for Panic {
        fn drop(&mut self) {
            panic!("aborting the process");
        }
    }
    let _panic = Panic;
    panic!("aborting the process");
}

#[inline]
pub(crate) fn abort_on_panic<T>(f: impl FnOnce() -> T) -> T {
    struct Bomb;
    impl Drop for Bomb {
        fn drop(&mut self) {
            abort();
        }
    }
    let bomb = Bomb;
    let t = f();
    mem::forget(bomb);
    t
}

#[inline]
pub(crate) fn extend(a: Layout,b: Layout) -> (Layout,usize) {
    let new_align = a.align().max(b.align());
    let pad = padding_needed_for(a, b.align());
    let offset = a.size().checked_add(pad).unwrap();
    let new_size = offset.checked_add(b.size()).unwrap();
    let layout = Layout::from_size_align(new_size, new_align).unwrap();
    (layout, offset)
}

#[inline]
pub(crate) fn padding_needed_for(layout: Layout,align: usize) -> usize {
    let len = layout.size();
    let len_rounded_up = len.wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
    len_rounded_up.wrapping_sub(len)
}
