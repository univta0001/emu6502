extern crate embed_resource;

use std::process::Command;

fn main() {
    embed_resource::compile("res/emu6502.rc");

    let result= Command::new("git").args(&["rev-parse", "HEAD"]).output();
    let git_hash = if let Ok(output) = result {
        String::from_utf8(output.stdout).unwrap()
    } else {
        "Unknown".into()
    };
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);    
}
