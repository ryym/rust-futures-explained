use crate::{parker::Parker, waker::MyWaker};
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

pub fn block_on<F: Future>(mut future: F) -> F::Output {
    let parker = Arc::new(Parker::default());
    let mywaker = MyWaker::new(parker.clone());
    let waker = mywaker.into_waker();

    let mut cx = Context::from_waker(&waker);
    let mut future = unsafe { Pin::new_unchecked(&mut future) };

    loop {
        match Future::poll(future.as_mut(), &mut cx) {
            Poll::Ready(val) => break val,
            Poll::Pending => parker.park(),
        }
    }
}
