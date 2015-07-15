use mio::{Sender, Token};
use std::sync::Arc;
use super::evserver::Notification;
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

    pub fn handle<T>(&self, request: T, token: Token, response_to: Sender<Notification>)
        where T: 'static + ToMessage + Send
    {
        let tracker = self.tracker.clone();
        self.thread_pool.execute(move|| {
            let response = tracker.handle(request);
            response_to.send(Notification::Response(token, response)).unwrap_or_else(|e| {
                error!("Error sending response to event loop connection {:?}: {:?}", token, e);
            });
        })
    }
}
