use mio::{Sender, Token};
use mogilefs_common::Backend;
use std::sync::Arc;
use super::notification::Notification;
use super::super::Tracker;
use threadpool::ThreadPool;

pub struct TrackerPool<B: Backend> {
    thread_pool: ThreadPool,
    tracker: Arc<Tracker<B>>,
}

impl<B: 'static + Backend> TrackerPool<B> {
    pub fn new(tracker: Tracker<B>, threads: usize) -> TrackerPool<B> {
        TrackerPool {
            thread_pool: ThreadPool::new(threads),
            tracker: Arc::new(tracker),
        }
    }

    pub fn handle(&self, request_line: Vec<u8>, token: Token, response_to: Sender<Notification>) {
        let tracker = self.tracker.clone();
        self.thread_pool.execute(move|| {
            let response = tracker.handle_bytes(request_line.as_ref());
            response_to.send(Notification::Response(token, response)).unwrap_or_else(|e| {
                error!("Error sending response to event loop connection {:?}: {:?}", token, e);
            });
        })
    }
}
