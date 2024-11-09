use std::process::Command;

fn main() {
    let _ = embed_resource::compile("res/emu6502.rc", embed_resource::NONE);

    let result = Command::new("git").args(["rev-parse", "HEAD"]).output();
    let git_hash = if let Ok(output) = result {
        String::from_utf8(output.stdout).unwrap()
    } else {
        "Unknown".into()
    };
    println!("cargo:rustc-env=GIT_HASH={git_hash}");

    #[cfg(target_os = "windows")]
    #[cfg(feature = "pcap")]
    {
        let delay_load_dlls = ["wpcap"];
        for dll in delay_load_dlls {
            println!("cargo:rustc-link-arg=/delayload:{dll}.dll");
        }
        // When using delayload, it's necessary to also link delayimp.lib
        // https://learn.microsoft.com/en-us/cpp/build/reference/dependentloadflag?view=msvc-170
        println!("cargo:rustc-link-arg=delayimp.lib");
    }
}
