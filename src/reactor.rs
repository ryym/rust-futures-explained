use crate::task::TaskId;
use std::{
    collections::HashMap,
    mem,
    sync::{
        mpsc::{channel, Sender},
        Arc, Mutex,
    },
    task::Waker,
    thread::{self, JoinHandle},
    time::Duration,
};

#[derive(Debug)]
pub enum TaskState {
    Ready,
    NotReady(Waker),
    Finished,
}

#[derive(Debug)]
enum Message {
    Close,
    Timeout(u64, TaskId),
}

#[derive(Debug)]
pub struct Reactor {
    dispatcher: Sender<Message>,
    handle: Option<JoinHandle<()>>,
    tasks: HashMap<TaskId, TaskState>,
}

impl Reactor {
    pub fn new() -> Arc<Mutex<Box<Self>>> {
        let (tx, rx) = channel::<Message>();
        let reactor = Arc::new(Mutex::new(Box::new(Reactor {
            dispatcher: tx,
            handle: None,
            tasks: HashMap::new(),
        })));

        // XXX: Weak を使うメリットってあるんだろうか？
        // msg_handle 内で upgrade が必要になるし、 Arc として clone する方が
        // 簡単なような。
        let reactor_clone = Arc::downgrade(&reactor);

        let handle = thread::spawn(move || {
            let mut handles = vec![];
            for msg in rx {
                println!("REACTOR: {:?}", msg);
                let reactor = reactor_clone.clone();
                match msg {
                    Message::Close => break,
                    Message::Timeout(duration, id) => {
                        let msg_handle = thread::spawn(move || {
                            thread::sleep(Duration::from_secs(duration));
                            let reactor = reactor.upgrade().unwrap();
                            reactor.lock().map(|mut r| r.wake(id)).unwrap();
                        });
                        handles.push(msg_handle);
                    }
                }
            }
        });

        reactor.lock().map(|mut r| r.handle = Some(handle)).unwrap();
        reactor
    }

    fn wake(&mut self, id: TaskId) {
        let state = self.tasks.get_mut(&id).unwrap();
        let prev_state = mem::replace(state, TaskState::Ready);
        match prev_state {
            TaskState::NotReady(waker) => waker.wake(),
            _ => panic!("Called 'wake' twice on task: {}", id),
        }
    }

    pub fn refresh_waker(&mut self, id: TaskId, waker: Waker) {
        let prev_state = self.set_waker(id, waker);
        if prev_state.is_none() {
            panic!("Tried to refresh not registered task with id: '{}'", id);
        }
    }

    pub fn set_timeout(&mut self, id: TaskId, waker: Waker, duration: u64) {
        let prev_state = self.set_waker(id, waker);
        if prev_state.is_some() {
            panic!("Tried to insert a task with id: '{}' twice!", id);
        }
        self.dispatcher
            .send(Message::Timeout(duration, id))
            .expect("dispatch Timeout message");
    }

    fn set_waker(&mut self, id: TaskId, waker: Waker) -> Option<TaskState> {
        self.tasks.insert(id, TaskState::NotReady(waker))
    }

    pub fn is_registered(&self, id: &TaskId) -> bool {
        self.tasks.contains_key(id)
    }

    pub fn is_ready(&self, id: TaskId) -> bool {
        let state = self.tasks.get(&id);
        state
            .map(|s| matches!(s, TaskState::Ready))
            .unwrap_or(false)
    }

    pub fn set_finished(&mut self, id: TaskId) {
        *self.tasks.get_mut(&id).unwrap() = TaskState::Finished;
    }
}

impl Drop for Reactor {
    fn drop(&mut self) {
        self.dispatcher.send(Message::Close).unwrap();
        self.handle.take().map(|h| h.join().unwrap()).unwrap();
    }
}
