use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct FileInfo {
    pub key: String,
    pub content: Option<Vec<u8>>,
    pub size: Option<usize>,
}

pub struct Backend(pub HashMap<String, FileInfo>);
pub type SyncBackend = Arc<Mutex<Backend>>;
