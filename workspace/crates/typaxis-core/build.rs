use std::{env, process::Command};

fn main() {
    println!("cargo:rerun-if-env-changed=RUSTC");
    let rustc = env::var_os("RUSTC").expect("Cargo must provide RUSTC to typaxis-core");
    let output = Command::new(rustc)
        .arg("--version")
        .output()
        .expect("typaxis-core build must be able to query rustc --version");
    assert!(output.status.success(), "rustc --version must succeed");
    let version = String::from_utf8(output.stdout).expect("rustc --version must be UTF-8");
    let version = version.trim();
    assert!(!version.is_empty(), "rustc --version must be nonempty");
    println!("cargo:rustc-env=TYPAXIS_RUST_VERSION={version}");
}
