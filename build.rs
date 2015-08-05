use std::fs::File;
use std::io::Write;
use std::process::Command;

#[allow(dead_code)]
fn main() {
    let git_rev_output = Command::new("git")
        .args(&[ "rev-parse", "--short", "HEAD" ])
        .output()
        .unwrap_or_else(|e| {
            panic!("Failed to execute git rev-parse: {}", e);
        });

    let mut rev_file = File::create("git-revision").unwrap();
    write!(rev_file, "{}", String::from_utf8_lossy(&git_rev_output.stdout).trim_right()).unwrap();
}
