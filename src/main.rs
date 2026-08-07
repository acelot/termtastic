mod app;
mod log2state;
mod meshtastic;
mod repository;
mod serde;
mod service;
mod state;
mod types;
mod ui;

use std::{env, process};

use etcetera::BaseStrategy;
use tracing_unwrap::ResultExt;

pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
pub const APP_VERSION: &str = env!("APP_VERSION");
pub const APP_REPO: &str = env!("CARGO_PKG_REPOSITORY");

#[tokio::main]
async fn main() {
    let xdg = etcetera::choose_base_strategy().expect_or_log("xdg config build failed");
    let data_dir = xdg.data_dir().join(APP_NAME);
    let config_dir = xdg.config_dir().join(APP_NAME);

    let args: Vec<String> = env::args().collect();

    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        println!(
            "\x1b[1m{}\x1b[22m {}

{}

Feature-rich handmade Meshtastic® console client written in Rust.
\x1b[4m\x1b[34m{}\x1b[0m\x1b[24m

\x1b[1mUsage\x1b[22m: {}{} [OPTIONS]

\x1b[1mOptions\x1b[22m:
  -h, --help     Print help
  -V, --version  Print version

\x1b[1mDirectories used\x1b[22m:
  \x1b[2m  data\x1b[22m  {}
  \x1b[2mconfig\x1b[22m  {}",
            APP_NAME,
            APP_VERSION,
            ui::logo::LOGO_ASCII.join("\n"),
            APP_REPO,
            APP_NAME,
            if cfg!(target_os = "windows") { ".exe" } else { "" },
            data_dir.display(),
            config_dir.display()
        );

        process::exit(0);
    }

    if args.contains(&"--version".to_string()) || args.contains(&"-V".to_string()) {
        println!("{}", APP_VERSION);

        process::exit(0);
    }

    app::run(data_dir, config_dir).await;
}
