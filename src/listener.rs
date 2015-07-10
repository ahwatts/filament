// Adapted from hyper's listener module.

use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, mpsc};
use std::thread;

pub struct ListenerPool {
    acceptor: TcpListener,
}

impl ListenerPool {
    pub fn new(acceptor: TcpListener) -> ListenerPool {
        ListenerPool {
            acceptor: acceptor,
        }
    }

    pub fn accept<F>(self, work: F, threads: usize)
        where F: Fn(TcpStream) + Send + Sync + 'static
    {
        assert!(threads != 0, "Can't accept 0 threads.");
        let work = Arc::new(work);
        let (super_tx, super_rx) = mpsc::channel();

        for _ in 0..threads {
            spawn_with(super_tx.clone(), work.clone(), &self.acceptor);
        }

        for _ in super_rx.iter() {
            spawn_with(super_tx.clone(), work.clone(), &self.acceptor);
        }
    }
}

fn spawn_with<F>(supervisor: mpsc::Sender<()>, work: Arc<F>, acceptor: &TcpListener)
    where F: Fn(TcpStream) + Send + Sync + 'static
{
    match acceptor.try_clone() {
        Ok(local_acceptor) => {
            thread::spawn(move|| {
                let _sentinel = Sentinel::new(supervisor, ());
                for stream_result in local_acceptor.incoming() {
                    match stream_result {
                        Ok(stream) => work(stream),
                        Err(e) => println!("Connection failed: {}", e),
                    }
                }
            });
        },
        Err(e) => {
            println!("Error cloning TCP Listener: {}", e);
        }
    }
}

struct Sentinel<T: Send + 'static> {
    value: Option<T>,
    supervisor: mpsc::Sender<T>,
}

impl<T: Send + 'static> Sentinel<T> {
    fn new(channel: mpsc::Sender<T>, data: T) -> Sentinel<T> {
        Sentinel {
            value: Some(data),
            supervisor: channel,
        }
    }
}

impl <T: Send + 'static> Drop for Sentinel<T> {
    fn drop(&mut self) {
        match self.supervisor.send(self.value.take().unwrap()) {
            Ok(_) => {},
            Err(e) => {
                println!("Error reporting dead thread to supervisor: {}", e);
            }
        }
    }
}
