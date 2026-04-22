// build.rs
use std::env;

fn main() {
    let ref_name = env::var("GITHUB_REF_NAME").unwrap_or_else(|_| "dev".to_string());

    println!("cargo:rustc-env=GIT_REF_NAME={}", ref_name);
    println!("cargo:rerun-if-env-changed=GITHUB_REF_NAME");
}
