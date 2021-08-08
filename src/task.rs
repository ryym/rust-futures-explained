use crate::reactor::Reactor;
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

pub type TaskId = usize;

#[derive(Clone, Debug)]
pub struct Task {
    id: TaskId,
    reactor: Arc<Mutex<Box<Reactor>>>,
    data: u64,
}

impl Task {
    pub fn new(reactor: Arc<Mutex<Box<Reactor>>>, data: u64, id: TaskId) -> Self {
        Task { id, reactor, data }
    }
}

impl Future for Task {
    // XXX: この Output ってどういう風に使うんだろう？
    // 今回のサンプルだと特に使い道はなさそう。
    type Output = TaskId;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut r = self.reactor.lock().unwrap();
        if r.is_ready(self.id) {
            r.set_finished(self.id);
            Poll::Ready(self.id)
        } else if r.is_registered(&self.id) {
            r.refresh_waker(self.id, cx.waker().clone());
            Poll::Pending
        } else {
            r.set_timeout(self.id, cx.waker().clone(), self.data);
            Poll::Pending
        }
    }
}
