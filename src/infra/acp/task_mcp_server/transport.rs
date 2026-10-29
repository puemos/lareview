use async_trait::async_trait;
use pmcp::error::TransportError;
use pmcp::shared::{StdioTransport, Transport, TransportMessage};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Stdin, Stdout};
use tokio::sync::Mutex;

use super::logging::log_raw_line;

#[derive(Debug)]
pub(super) struct LineDelimitedStdioTransport {
    stdin: Arc<Mutex<BufReader<Stdin>>>,
    stdout: Arc<Mutex<Stdout>>,
}

impl LineDelimitedStdioTransport {
    pub(super) fn new() -> Self {
        Self {
            stdin: Arc::new(Mutex::new(BufReader::new(tokio::io::stdin()))),
            stdout: Arc::new(Mutex::new(tokio::io::stdout())),
        }
    }
}

#[async_trait]
impl Transport for LineDelimitedStdioTransport {
    async fn send(&mut self, message: TransportMessage) -> pmcp::Result<()> {
        let json = serde_json::to_string(&message)
            .map_err(|e| pmcp::Error::Transport(TransportError::Serialization(e.to_string())))?;

        // Log outbound messages for full request/response visibility.
        log_raw_line(&format!("OUT {json}"));

        let mut stdout = self.stdout.lock().await;
        stdout
            .write_all(json.as_bytes())
            .await
            .map_err(|e| pmcp::Error::Transport(TransportError::Io(e.to_string())))?;
        stdout
            .write_all(b"\n")
            .await
            .map_err(|e| pmcp::Error::Transport(TransportError::Io(e.to_string())))?;
        stdout
            .flush()
            .await
            .map_err(|e| pmcp::Error::Transport(TransportError::Io(e.to_string())))?;
        Ok(())
    }

    async fn receive(&mut self) -> pmcp::Result<TransportMessage> {
        loop {
            let mut line = String::new();
            let bytes = {
                let mut stdin = self.stdin.lock().await;
                stdin
                    .read_line(&mut line)
                    .await
                    .map_err(|e| pmcp::Error::Transport(TransportError::Io(e.to_string())))?
            };
            if bytes == 0 {
                return Err(pmcp::Error::Transport(TransportError::ConnectionClosed));
            }

            // Answer the modern stdio probe so dual-era clients can fall back to
            // initialize. pmcp's typed stdio parser does not support discovery;
            // passing it through would close the pipe before fallback is possible.
            log_raw_line(&format!("IN {}", line.trim_end()));
            let value: serde_json::Value = serde_json::from_str(&line).map_err(|e| {
                pmcp::Error::Transport(TransportError::InvalidMessage(format!("Invalid JSON: {e}")))
            })?;
            if value.get("method").and_then(|v| v.as_str()) == Some("server/discover") {
                if let Some(id) = value.get("id") {
                    let response: pmcp::types::JSONRPCResponse = serde_json::from_value(serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {"code": -32601, "message": "Use initialize to negotiate a supported MCP version"}
                    })).map_err(|e| pmcp::Error::Transport(TransportError::InvalidMessage(e.to_string())))?;
                    self.send(TransportMessage::Response(response)).await?;
                }
                continue;
            }
            return StdioTransport::parse_message(line.as_bytes());
        }
    }

    async fn close(&mut self) -> pmcp::Result<()> {
        Ok(())
    }
}
