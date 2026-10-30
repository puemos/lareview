use serde_json::{Value, json};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[tokio::test]
async fn modern_discovery_probe_can_fall_back_to_legacy_initialize() {
    let cwd = tempfile::tempdir().unwrap();
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_lareview"))
        .arg("--task-mcp-server")
        .current_dir(cwd.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap()).lines();

    for (request, expected) in [
        (
            json!({"jsonrpc":"2.0", "id":1, "method":"server/discover", "params":{
                "_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28"}
            }}),
            "error",
        ),
        (
            json!({"jsonrpc":"2.0", "id":2, "method":"initialize", "params":{
                "protocolVersion":"2025-11-25", "capabilities":{},
                "clientInfo":{"name":"copilot-test", "version":"1"}
            }}),
            "result",
        ),
        (
            json!({"jsonrpc":"2.0", "id":3, "method":"tools/list", "params":{}}),
            "result",
        ),
    ] {
        stdin
            .write_all(format!("{request}\n").as_bytes())
            .await
            .unwrap();
        stdin.flush().await.unwrap();
        let line = tokio::time::timeout(std::time::Duration::from_secs(10), stdout.next_line())
            .await
            .expect("MCP server must answer without closing the connection")
            .unwrap()
            .expect("response line");
        let response: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(response["id"], request["id"]);
        assert!(response.get(expected).is_some(), "{response}");
        if expected == "error" {
            assert_eq!(response["error"]["code"], -32601);
        }
        if request["method"] == "tools/list" {
            assert!(
                response["result"]["tools"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|tool| tool["name"] == "finalize_review")
            );
        }
    }
    child.kill().await.unwrap();
}
