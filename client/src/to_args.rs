use std::collections::HashMap;
use mogilefs_common::requests::*;

pub trait ToArgs {
    fn args(&self) -> Vec<(String, String)>;

    fn args_hash(&self) -> HashMap<String, String> {
        let mut rv = HashMap::new();
        for (k, v) in self.args().into_iter() {
            rv.entry(k).or_insert(v);
        }
        rv
    }
}

impl ToArgs for CreateDomain {
    fn args(&self) -> Vec<(String, String)> {
        vec!{
            ("domain".to_string(), self.domain.clone()),
        }
    }
}

impl ToArgs for CreateOpen {
    fn args(&self) -> Vec<(String, String)> {
        let mut rv = vec!{
            ("domain".to_string(), self.domain.clone()),
            ("key".to_string(), self.key.clone()),
            ("multi_dest".to_string(), self.multi_dest.to_string()),
        };

        if self.size.is_some() {
            rv.push(("size".to_string(), self.size.clone().unwrap().to_string()));
        }

        rv
    }
}

impl ToArgs for CreateClose {
    fn args(&self) -> Vec<(String, String)> {
        let mut rv = vec!{
            ("domain".to_string(), self.domain.clone()),
            ("key".to_string(), self.key.clone()),
            ("fid".to_string(), self.fid.to_string()),
            ("devid".to_string(), self.devid.to_string()),
            ("path".to_string(), self.path.to_string()),
        };

        if self.checksum.is_some() {
            rv.push(("checksum".to_string(), self.checksum.clone().unwrap()));
        }

        rv
    }
}

impl ToArgs for GetPaths {
    fn args(&self) -> Vec<(String, String)> {
        let mut rv = vec!{
            ("domain".to_string(), self.domain.clone()),
            ("key".to_string(), self.key.clone()),
            ("noverify".to_string(), self.noverify.to_string()),
        };

        if self.pathcount.is_some() {
            rv.push(("pathcount".to_string(), self.pathcount.clone().unwrap().to_string()));
        }

        rv
    }
}

impl ToArgs for FileInfo {
    fn args(&self) -> Vec<(String, String)> {
        vec!{
            ("domain".to_string(), self.domain.clone()),
            ("key".to_string(), self.key.clone()),
        }
    }
}

impl ToArgs for Rename {
    fn args(&self) -> Vec<(String, String)> {
        vec!{
            ("domain".to_string(), self.domain.clone()),
            ("from_key".to_string(), self.from_key.clone()),
            ("to_key".to_string(), self.to_key.clone()),
        }
    }
}

impl ToArgs for UpdateClass {
    fn args(&self) -> Vec<(String, String)> {
        vec!{
            ("domain".to_string(), self.domain.clone()),
            ("key".to_string(), self.key.clone()),
            ("class".to_string(), self.new_class.clone()),
        }
    }
}

impl ToArgs for Delete {
    fn args(&self) -> Vec<(String, String)> {
        vec!{
            ("domain".to_string(), self.domain.clone()),
            ("key".to_string(), self.key.clone()),
        }
    }
}

impl ToArgs for ListKeys {
    fn args(&self) -> Vec<(String, String)> {
        let mut rv = vec!{
            ("domain".to_string(), self.domain.clone()),
        };

        if self.prefix.is_some() {
            rv.push(("prefix".to_string(), self.prefix.clone().unwrap()));
        }

        if self.after.is_some() {
            rv.push(("after".to_string(), self.after.clone().unwrap()));
        }

        if self.limit.is_some() {
            rv.push(("limit".to_string(), self.limit.clone().unwrap().to_string()));
        }

        rv
    }
}
