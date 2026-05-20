#![allow(clippy::manual_unwrap_or)]

mod cli;

#[tokio::main]
async fn main() {
    if let Err(err) = cli::run().await {
        eprintln!("qbank: {err}");
        std::process::exit(1);
    }
}
