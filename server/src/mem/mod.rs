//! In-memory implementations of MogileFS, for testing purposes.

pub use self::mem_backend::{MemBackend, SyncMemBackend};
pub use self::model::{MemDomain, MemFileInfo};

mod mem_backend;
mod model;

#[cfg(test)]
pub mod test_support {
    pub use super::mem_backend::test_support::*;
    pub use super::model::test_support::*;
}
