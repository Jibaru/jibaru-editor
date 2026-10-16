//! `jb-mcp`: MCP server over stdio. Logs go to stderr; stdout carries only
//! protocol messages.

use rmcp::ServiceExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("jb-mcp {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let service = jb_mcp::server::JbServer::new().serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
