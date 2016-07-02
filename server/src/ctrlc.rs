// Adapted from https://github.com/Detegr/rust-ctrlc .

use libc::{signal, c_int, SIGINT};
use std::sync::{Condvar, Mutex};
use std::thread;

lazy_static!{
    static ref CVAR: Condvar = Condvar::new();
    static ref MUTEX: Mutex<bool> = Mutex::new(false);
}

fn handler(_: c_int) {
    CVAR.notify_all();
}

#[allow(missing_copy_implementations)]
pub struct CtrlC;

impl CtrlC {
    pub fn set_handler<F: Fn() -> () + 'static + Send>(user_handler: F) {
        // This seems to work, even though F is not the same type of
        // function as handler.
        let handler_ptr = handler as *const F;

        unsafe {
            signal(SIGINT, handler_ptr as usize);
        }

        thread::spawn(move || {
            loop {
                let _ = CVAR.wait(MUTEX.lock().unwrap());
                user_handler();
            }
        });
    }
}
