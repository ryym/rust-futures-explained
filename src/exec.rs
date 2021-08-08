use crate::waker::MyWaker;
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    thread,
};

pub fn block_on<F: Future>(mut future: F) -> F::Output {
    let mywaker = Arc::new(MyWaker::default());
    let waker = MyWaker::into_waker(Arc::into_raw(mywaker));

    let mut cx = Context::from_waker(&waker);
    let mut future = unsafe { Pin::new_unchecked(&mut future) };

    loop {
        match Future::poll(future.as_mut(), &mut cx) {
            Poll::Ready(val) => break val,
            Poll::Pending => thread::park(),
        }
    }
}
