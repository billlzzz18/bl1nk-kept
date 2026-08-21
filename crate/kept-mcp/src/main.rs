//! `bl1nk-kept-mcp` — the unified bl1nk-kept MCP server.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // `--check` / `--setup` let installers verify the binary without launching
    // the long-running stdio server.
    if std::env::args().any(|arg| arg == "--check" || arg == "--setup") {
        println!("bl1nk-kept-mcp {}: OK", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    kept_mcp::mcp::server::run().await
}
