use async_trait::async_trait;
use pmcp::error::TransportError;
use pmcp::shared::{StdioTransport, Transport, TransportMessage};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Stdin, Stdout};
use tokio::sync::Mutex;

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
        let mut stdin = self.stdin.lock().await;
        let mut line = String::new();

        let bytes = stdin
            .read_line(&mut line)
            .await
            .map_err(|e| pmcp::Error::Transport(TransportError::Io(e.to_string())))?;

        if bytes == 0 {
            return Err(pmcp::Error::Transport(TransportError::ConnectionClosed));
        }

        let json_value: serde_json::Value = serde_json::from_str(&line).map_err(|e| {
            pmcp::Error::Transport(TransportError::InvalidMessage(format!("Invalid JSON: {e}")))
        })?;

        if json_value.get("method").is_some() {
            if json_value.get("id").is_some() {
                let _request: pmcp::types::JSONRPCRequest<serde_json::Value> =
                    serde_json::from_value(json_value).map_err(|e| {
                        pmcp::Error::Transport(TransportError::InvalidMessage(format!(
                            "Invalid request: {e}"
                        )))
                    })?;

                return StdioTransport::parse_message(line.as_bytes());
            }

            return StdioTransport::parse_message(line.as_bytes());
        }

        StdioTransport::parse_message(line.as_bytes())
    }

    async fn close(&mut self) -> pmcp::Result<()> {
        Ok(())
    }
}
