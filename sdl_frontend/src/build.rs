extern crate embed_resource;

use std::process::Command;

fn main() {
    embed_resource::compile("res/emu6502.rc");

    let output = Command::new("git").args(&["rev-parse", "HEAD"]).output().unwrap();
    let git_hash = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);    
}
