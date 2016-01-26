extern crate docopt;
extern crate env_logger;
extern crate filament;
extern crate rustc_serialize;
extern crate mogilefs_client;
extern crate mogilefs_common;
extern crate url;

#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;

use docopt::Docopt;
use filament::util::SocketAddrList;
use mogilefs_client::MogClient;
use mogilefs_common::MogError;
use mogilefs_common::requests::*;
use rustc_serialize::{Decodable, Decoder};
use url::Url;

static VERSION_NUM: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
static GIT_COMMIT: &'static str = include_str!("../../git-revision");

lazy_static!{
    static ref FULL_VERSION: String =
        format!("filament version {} commit {}",
                VERSION_NUM.unwrap_or("unknown"), GIT_COMMIT);
}

pub fn main() {
    env_logger::init().unwrap();

    let opts: Options = Docopt::new(USAGE)
        .and_then(|d| d.version(Some(FULL_VERSION.to_string())).decode())
        .unwrap_or_else(|e| e.exit());
    debug!("opts = {:?}", opts);

    let mut client = MogClient::new(opts.flag_trackers.as_slice());

    let resp_rslt = if opts.cmd_create_domain {
        client.request(&CreateDomain {
            domain: opts.arg_domain.expect("No domain provided."),
        })
    } else if opts.cmd_create_open {
        client.request(&CreateOpen {
            domain: opts.arg_domain.expect("No domain provided."),
            class: opts.flag_class,
            key: opts.arg_key.expect("No key provided."),
            multi_dest: opts.flag_multi_dest,
            size: opts.flag_size,
        })
    } else if opts.cmd_create_close {
        client.request(&CreateClose {
            domain: opts.arg_domain.expect("No domain provided."),
            key: opts.arg_key.expect("No key provided."),
            fid: opts.arg_fid.expect("No fid provided."),
            devid: opts.arg_devid.expect("No devid provided."),
            path: opts.arg_path.expect("No URL provided."),
            checksum: opts.flag_checksum,
        })
    } else if opts.cmd_create_close {
        client.request(&CreateClose {
            domain: opts.arg_domain.expect("No domain provided."),
            key: opts.arg_key.expect("No key provided."),
            fid: opts.arg_fid.expect("No fid provided."),
            devid: opts.arg_devid.expect("No devid provided."),
            path: opts.arg_path.expect("No URL provided."),
            checksum: opts.flag_checksum,
        })
    } else if opts.cmd_create_class {
        client.request(&CreateClass {
            domain: opts.arg_domain.expect("No domain provided."),
            class: opts.arg_class.expect("No class name provided."),
            mindevcount: opts.arg_mindevcount.expect("No minimum device count provided."),
            replpolicy: opts.flag_replpolicy,
            hashtype: opts.flag_hashtype,
            update: opts.flag_update,
        })
    } else if opts.cmd_file_info {
        client.request(&FileInfo {
            domain: opts.arg_domain.expect("No domain provided."),
            key: opts.arg_key.expect("No key provided."),
        })
    } else if opts.cmd_get_paths {
        client.request(&GetPaths {
            domain: opts.arg_domain.expect("No domain provided."),
            key: opts.arg_key.expect("No key provided."),
            noverify: opts.flag_no_verify,
            pathcount: opts.flag_path_count,
        })
    } else if opts.cmd_rename {
        client.request(&Rename {
            domain: opts.arg_domain.expect("No domain provided."),
            from_key: opts.arg_from_key.expect("No source key provided."),
            to_key: opts.arg_to_key.expect("No destination key provided."),
        })
    } else if opts.cmd_update_class {
        client.request(&UpdateClass {
            domain: opts.arg_domain.expect("No domain provided."),
            key: opts.arg_key.expect("No key provided."),
            new_class: opts.arg_new_class.expect("No class name provided."),
        })
    } else if opts.cmd_list_keys {
        client.request(&ListKeys {
            domain: opts.arg_domain.expect("No domain provided."),
            prefix: opts.flag_prefix,
            after: opts.flag_after,
            limit: opts.flag_limit,
        })
    } else if opts.cmd_noop {
        client.request(&Noop)
    } else {
        Err(MogError::Other(format!("No command provided?!?"), None))
    };

    match resp_rslt {
        Ok(resp) => println!("{:?}", resp),
        Err(e) => println!("Error: {}", e),
    }
}

static USAGE: &'static str = "
A command-line tool for querying a MogileFS system.

Usage:
  filament-cli [options] create-domain <domain>
  filament-cli [options] create-open <domain> <key> [--class=STRING --multi-dest --size=N]
  filament-cli [options] create-close <domain> <key> <fid> <devid> <path> [--checksum=STRING]
  filament-cli [options] create-class <domain> <class> <mindevcount> [--replpolicy=STRING --hashtype=STRING --update]
  filament-cli [options] file-info <domain> <key>
  filament-cli [options] get-paths <domain> <key> [--no-verify --path-count=N]
  filament-cli [options] rename <domain> <from-key> <to-key>
  filament-cli [options] update-class <domain> <key> <new-class>
  filament-cli [options] list-keys <domain> [--prefix=PREFIX --after=AFTER --limit=N]
  filament-cli [options] noop
  filament-cli (-h | --help | -v | --version)

General Options:
  -h, --help                 This help message.
  -v, --version              Print version information.
  -t IPS, --trackers IPS     A comma-separated list of tracker ip:port combinations [default: 127.0.0.1:7001]
";

#[derive(Debug, RustcDecodable)]
struct Options {
    flag_trackers: SocketAddrList,
    flag_class: Option<String>,
    flag_multi_dest: bool,
    flag_size: Option<u64>,
    flag_checksum: Option<String>,
    flag_replpolicy: Option<String>,
    flag_hashtype: Option<String>,
    flag_update: bool,
    flag_prefix: Option<String>,
    flag_after: Option<String>,
    flag_limit: Option<u64>,
    flag_no_verify: bool,
    flag_path_count: Option<u64>,

    arg_domain: Option<String>,
    arg_key: Option<String>,
    arg_from_key: Option<String>,
    arg_to_key: Option<String>,
    arg_fid: Option<u64>,
    arg_devid: Option<u64>,
    arg_path: Option<Url>,
    arg_class: Option<String>,
    arg_mindevcount: Option<u64>,
    arg_new_class: Option<String>,

    cmd_create_domain: bool,
    cmd_create_open: bool,
    cmd_create_close: bool,
    cmd_create_class: bool,
    cmd_file_info: bool,
    cmd_get_paths: bool,
    cmd_rename: bool,
    cmd_update_class: bool,
    cmd_list_keys: bool,
    cmd_noop: bool,
}
