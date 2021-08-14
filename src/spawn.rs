// https://web.archive.org/web/20200207092849/https://stjepang.github.io/2020/01/31/build-your-own-executor.html

use crossbeam::channel;
use futures::{channel::oneshot, task::ArcWake};
use once_cell::sync::Lazy;
use std::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
    sync::{Arc, Mutex},
    task::{Context},
    thread,
};

static QUEUE: Lazy<channel::Sender<Arc<Work>>> = Lazy::new(|| {
    let (sender, receiver) = channel::unbounded::<Arc<Work>>();
    for _ in 0..num_cpus::get().max(1) {
        let rc = receiver.clone();
        thread::spawn(move || {
            for work in rc {
                work.run();
            }
        });
    }
    sender
});

pub type JoinHandle<R> = Pin<Box<dyn Future<Output = R> + Send>>;

pub fn spawn<F, R>(future: F) -> JoinHandle<R>
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    // future を実行して結果を send するだけの future.
    let (sender, receiver) = oneshot::channel();
    let future = async move {
        let _ = sender.send(future.await);
    };

    let work = Arc::new(Work {
        state: AtomicUsize::new(PENDING),
        future: Mutex::new(Box::pin(future)),
    });
    QUEUE.send(work).unwrap();

    Box::pin(async { receiver.await.unwrap() })
}

const PENDING: usize = 0b00;
const WOKEN: usize = 0b01;
const RUNNING: usize = 0b10;

pub struct Work {
    state: AtomicUsize,
    future: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>,
}

impl Work {
    fn run(self: Arc<Self>) {
        let waker = futures::task::waker(Arc::new(WorkWaker { work: self.clone() }));

        self.state.store(RUNNING, Ordering::SeqCst);

        let cx = &mut Context::from_waker(&waker);
        let poll = self.future.try_lock().unwrap().as_mut().poll(cx);

        if poll.is_pending() {
            let prev_state = self.state.fetch_and(!RUNNING, Ordering::SeqCst);
            // 再度 poll するために enqueue する。
            if prev_state == WOKEN | RUNNING {
                QUEUE.send(self).unwrap();
            }
            // prev_state == WOKEN -> Work.wake() により enqueue されてるはず。
            // prev_state == PENDING, RUNNING -> まだ wake() が呼ばれてない。
        }
    }

    fn wake(self: &Arc<Self>) {
        let prev_state = self.state.fetch_or(WOKEN, Ordering::SeqCst);
        if prev_state == PENDING {
            // 再度 poll するために enqueue する。
            QUEUE.send(self.clone()).unwrap();
        }
    }
}

struct WorkWaker {
    work: Arc<Work>,
}

impl ArcWake for WorkWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.work.wake();
    }

    fn wake(self: Arc<Self>) {
        self.work.wake();
    }
}
