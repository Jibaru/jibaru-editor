//! MCP server for Jibaru Editor. `tools` holds the session and tool logic
//! independent of the protocol; `server` adapts it to rmcp over stdio.

pub mod server;
pub mod tools;
