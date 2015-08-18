//! In-memory implementations of MogileFS, for testing purposes.

pub use self::mem_backend::{MemBackend, SyncMemBackend};
pub use self::mem_storage::MemStorage;
pub use self::model::{Domain, FileInfo};

mod mem_backend;
mod mem_storage;
mod model;

#[cfg(test)]
pub mod test_support {
    pub use super::mem_backend::test_support::*;
    pub use super::mem_storage::test_support::*;
    pub use super::model::test_support::*;
}
