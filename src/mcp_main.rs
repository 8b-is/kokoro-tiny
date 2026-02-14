//! kokoro-mcp: MCP Server for kokoro-tiny TTS
//!
//! This binary implements a Model Context Protocol (MCP) server that allows
//! AI assistants like Claude to use text-to-speech capabilities.
//!
//! # Usage
//! Add to Claude Desktop config:
//! ```json
//! {
//!   "mcpServers": {
//!     "kokoro-tts": {
//!       "command": "kokoro-mcp",
//!       "args": []
//!     }
//!   }
//! }
//! ```

use kokoro_tiny::mcp_server::McpServer;

#[tokio::main]
async fn main() {
    // Initialize MCP server
    let mut server = match McpServer::new().await {
        Ok(server) => server,
        Err(e) => {
            eprintln!("❌ Failed to initialize MCP server: {}", e);
            eprintln!("💡 Make sure the kokoro models are downloaded.");
            eprintln!("   They will be downloaded automatically on first use.");
            std::process::exit(1);
        }
    };

    // Run the server
    if let Err(e) = server.run() {
        eprintln!("❌ MCP server error: {}", e);
        std::process::exit(1);
    }
}
