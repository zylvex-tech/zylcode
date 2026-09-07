#[tokio::main]
async fn main() {
    zylcode_core::cli::run()
        .await
        .unwrap_or_else(|e| {
            eprintln!("error: {e}");
            std::process::exit(1);
        });
}
