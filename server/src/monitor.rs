//! A version of filament which will sit alongside the MogileFS
//! cluster. It will check for errors / inconsistencies (like a
//! continuous fsck), and it will slowly back up files to S3.

pub struct Monitor;

impl Monitor {
    pub fn new() -> Monitor {
        Monitor
    }
}
