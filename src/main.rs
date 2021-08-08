mod exec;
mod reactor;
mod task;
mod waker;

use crate::{exec::block_on, reactor::Reactor, task::Task};
use std::time::Instant;

// #[tokio::main]
// async fn main() {
//     run_futures().await;
// }

fn main() {
    block_on(run_futures());
}

async fn run_futures() {
    let start = Instant::now();

    let reactor = Reactor::new();
    let future1 = Task::new(reactor.clone(), 1, 1);
    let future2 = Task::new(reactor.clone(), 2, 2);

    let fut1 = async {
        let val = future1.await;
        println!("Got {} at time: {:.2}", val, start.elapsed().as_secs_f32());
    };
    let fut2 = async {
        let val = future2.await;
        println!("Got {} at time: {:.2}", val, start.elapsed().as_secs_f32());
    };

    fut1.await;
    fut2.await;

    // Or we can run futures concurrently:
    // futures::future::join(fut1, fut2).await;
}
