//! Example: Testing the MCP server manually
//!
//! This example shows how to test the MCP server by sending JSON-RPC requests manually.
//!
//! Run the MCP server:
//! ```bash
//! cargo run --features playback --bin kokoro-mcp
//! ```
//!
//! Then send JSON-RPC requests via stdin:
//! ```json
//! {"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
//! {"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
//! {"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"speak_to_user","arguments":{"text":"Hello from MCP!","voice":"af_bella"}}}
//! ```

use std::io::{self, Write};

fn main() {
    println!("🎤 MCP Server Test Helper");
    println!("==========================\n");
    println!("Start the MCP server in another terminal:");
    println!("  cargo run --features playback --bin kokoro-mcp\n");
    println!("Then paste these JSON-RPC requests (one per line):\n");

    let requests = vec![
        (
            "Initialize",
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
        ),
        (
            "List Tools",
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
        ),
        (
            "List Voices",
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"list_voices","arguments":{}}}"#,
        ),
        (
            "Speak (Happy)",
            r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"speak_with_emotion","arguments":{"text":"Amazing work! All tests are passing!","emotion":"happy"}}}"#,
        ),
        (
            "Speak (Alert)",
            r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"speak_with_emotion","arguments":{"text":"Alert! Build failed with three errors.","emotion":"alert"}}}"#,
        ),
        (
            "Speak (Info)",
            r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"speak_to_user","arguments":{"text":"Just finished the analysis. Found two optimization opportunities.","voice":"af_sky"}}}"#,
        ),
        (
            "Save to File",
            r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"synthesize_to_file","arguments":{"text":"This is a test message","output_path":"/tmp/mcp_test.wav","voice":"af_bella"}}}"#,
        ),
    ];

    for (i, (name, request)) in requests.iter().enumerate() {
        println!("{}. {} :", i + 1, name);
        println!("{}\n", request);
    }

    println!("\n📋 Expected Response Format:");
    println!(r#"{{"jsonrpc":"2.0","id":1,"result":{{...}}}}"#);
    println!("\n💡 Tip: Use jq to pretty-print responses:");
    println!("  echo '<request>' | jq");
}
