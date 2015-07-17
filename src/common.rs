use std::collections::HashMap;

pub struct FileInfo {
    pub key: String,
    pub content: Option<Vec<u8>>,
    pub size: Option<usize>,
}

pub struct Backend(pub HashMap<String, FileInfo>);
