// build.rs
use std::env::{self, VarError};

fn main() {
    let app_version = match env::var("GIT_REF_TYPE").as_ref().map(|v| v.as_str()) {
        Ok("tag") => env::var("GIT_REF_NAME").expect("GIT_REF_NAME should be set"),
        Ok("branch") => env::var("GIT_SHA")
            .expect("GIT_SHA should be set")
            .chars()
            .take(7)
            .collect(),
        Ok(_) => panic!("invalid GIT_REF_TYPE"),
        Err(VarError::NotPresent) => "dev".to_owned(),
        Err(e) => panic!("{:?}", e),
    };

    println!("cargo:rustc-env=APP_VERSION={}", app_version);
}
