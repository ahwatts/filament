// Adapted from https://github.com/Detegr/rust-ctrlc .

use libc::funcs::posix01::signal::signal;
use libc::{SIGINT, c_int};
use std::mem;
use std::sync::{Condvar, Mutex};
use std::thread;

lazy_static!{
    static ref CVAR: Condvar = Condvar::new();
    static ref MUTEX: Mutex<bool> = Mutex::new(false);
}

#[repr(C)]
fn handler(_: c_int) {
    CVAR.notify_all();
}

#[allow(missing_copy_implementations)]
pub struct CtrlC;

impl CtrlC {
    pub fn set_handler<F: Fn() -> () + 'static + Send>(user_handler: F) {
        unsafe {
            signal(SIGINT, mem::transmute(handler));
        }

        thread::spawn(move || {
            loop {
                let _ = CVAR.wait(MUTEX.lock().unwrap());
                user_handler();
            }
        });
    }
}
