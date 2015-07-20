#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::RwLock;

lazy_static!{
    pub static ref STRINGS: StringTable = StringTable::new();
}

pub struct StringTable(RwLock<StringTableUnsync>);

impl StringTable {
    pub fn new() -> StringTable {
        StringTable(RwLock::new(StringTableUnsync::new()))
    }
}

struct StringTableUnsync {
    // Is there a better way than to store two copies of the same
    // string?
    string_to_id: HashMap<String, i32>,
    id_to_string: HashMap<i32, String>,
}

impl StringTableUnsync {
    pub fn new() -> StringTableUnsync {
        StringTableUnsync {
            string_to_id: HashMap::new(),
            id_to_string: HashMap::new(),
        }
    }

    pub fn get<'a>(&'a self, key: &str) -> Option<&'a str> {
        self.string_to_id.get(key)
            .and_then(|i| self.id_to_string.get(i))
            .map(|s| s.as_ref())
    }

    pub fn get_id(&self, key: &str) -> Option<i32> {
        self.string_to_id.get(key).map(|i| *i)
    }

    pub fn get_str<'a>(&'a self, id: i32) -> Option<&'a str> {
        self.id_to_string.get(&id).map(|s| s.as_ref())
    }

    // pub fn get_or_insert<'a>(&self, key: &str) -> &'a str {
    // }
}
