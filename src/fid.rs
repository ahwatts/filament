use store::Store;
use mysql::value::FromValue;

#[allow(dead_code)]
pub struct Fid<'a> {
    pub fid_id: i32,
    pub domain_id: i32,
    pub key:  String,
    pub length: i64,
    pub class_id: i32,
    pub devcount: i32,
    pub device_ids: Vec<i32>,
    store: &'a Store,
}

impl<'a> Fid<'a> {
    // pub fn new_from_domain_and_key<T: AsRef<str>, U: AsRef<str>>(store: &'a Store, domain: T, key: U) -> Result<Fid<'a>, String> {
    //     let domain_id_opt = try!(store.get_domain_id(domain.as_ref()).map_err(|e| e.to_error_string()));
    //     let domain_id = try!(domain_id_opt.ok_or("unreg_domain Domain+name+invalid/not+found".to_string()));
    //     Self::new_from_dmid_and_key(store, domain_id, key)
    // }

    pub fn new_from_dmid_and_key<U: AsRef<str>>(store: &'a Store, domain_id: i32, key: U) -> Result<Fid<'a>, String> {
        let row_opt = try!(store.get_file_row_from_dmid_and_key(domain_id, key.as_ref()).map_err(|e| e.to_error_string()));
        let row = try!(row_opt.ok_or("unknown_key unknown_key"));
        let fid_id = i32::from_value(&row["fid"]);
        let devids = try!(store.get_devids_for_fid(fid_id).map_err(|e| e.to_error_string()));

        Ok(Fid {
            fid_id: fid_id,
            domain_id: domain_id,
            key: key.as_ref().to_string(),
            length: i64::from_value(&row["length"]),
            class_id: i32::from_value(&row["classid"]),
            devcount: i32::from_value(&row["devcount"]),
            device_ids: devids,
            store: store,
        })
    }
}
