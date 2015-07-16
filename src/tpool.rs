use mio::{Sender, Token};
use std::fmt::Debug;
use std::sync::Arc;
use super::tracker::evented::Notification;
use super::tracker::{Tracker, ToMessage};
use threadpool::ThreadPool;

pub struct TrackerPool {
    thread_pool: ThreadPool,
    tracker: Arc<Tracker>,
}

impl TrackerPool {
    pub fn new(tracker: Tracker, threads: usize) -> TrackerPool {
        TrackerPool {
            thread_pool: ThreadPool::new(threads),
            tracker: Arc::new(tracker),
        }
    }

    pub fn handle<T>(&self, to_request: T, token: Token, response_to: Sender<Notification>)
        where T: ToMessage + Send + Debug
    {
        let tracker = self.tracker.clone();
        let request = match to_request.to_message() {
            Ok(msg) => msg,
            Err(e) => {
                error!("Error converting line to tracker message: {}", e);
                return;
            }
        };

        self.thread_pool.execute(move|| {
            let response = tracker.handle(request);
            response_to.send(Notification::Response(token, response)).unwrap_or_else(|e| {
                error!("Error sending response to event loop connection {:?}: {:?}", token, e);
            });
        })
    }
}
