use std::{
    sync::Arc,
    task::{RawWaker, RawWakerVTable, Waker},
    thread,
};

#[derive(Clone, Debug)]
pub struct MyWaker {
    pub thread: thread::Thread,
}

impl MyWaker {
    pub fn into_waker(s: *const MyWaker) -> Waker {
        let raw_waker = RawWaker::new(s as *const (), &VTABLE);
        unsafe { Waker::from_raw(raw_waker) }
    }

    fn vtable_clone(s: *const MyWaker) -> RawWaker {
        let arc = unsafe { Arc::from_raw(s) };
        // Increase ref count. Decreasing is done at drop of VTABLE.
        std::mem::forget(arc.clone());
        RawWaker::new(Arc::into_raw(arc) as *const (), &VTABLE)
    }

    fn vtable_wake(s: *const MyWaker) {
        let waker_arc = unsafe { Arc::from_raw(s) };
        waker_arc.thread.unpark();
    }

    unsafe fn vtable_wake_by_ref(s: *const MyWaker) {
        (*s).thread.unpark();
    }

    unsafe fn vtable_drop(s: *const MyWaker) {
        drop(Arc::from_raw(s))
    }
}

// https://github.com/rust-lang/rfcs/blob/master/text/2592-futures.md#waking-up
const VTABLE: RawWakerVTable = unsafe {
    RawWakerVTable::new(
        |s| MyWaker::vtable_clone(s as *const MyWaker),
        |s| MyWaker::vtable_wake(s as *const MyWaker),
        |s| MyWaker::vtable_wake_by_ref(s as *const MyWaker),
        |s| MyWaker::vtable_drop(s as *const MyWaker),
    )
};

impl Default for MyWaker {
    fn default() -> Self {
        MyWaker {
            thread: thread::current(),
        }
    }
}
