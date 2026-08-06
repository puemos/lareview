use super::task_generator::build_client_capabilities;
use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::schema::v1::{
    InitializeRequest, NewSessionRequest, SessionConfigKind, SessionConfigOption,
    SessionConfigOptionValue, SessionId, SessionNotification, SetSessionConfigOptionRequest,
};
use agent_client_protocol::{Agent, ByteStreams, Client as AcpClient, ConnectionTo};
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::thread;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::runtime::Builder;
use tokio::task::LocalSet;

/// A user-selected value for an ACP session configuration option.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum AgentConfigValue {
    Select(String),
    Boolean(bool),
}

/// A typed ACP configuration selection supplied by the frontend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfigSelection {
    pub config_id: String,
    pub value: AgentConfigValue,
}

fn supported_value(
    options: &[SessionConfigOption],
    selection: &AgentConfigSelection,
) -> Option<SessionConfigOptionValue> {
    let option = options
        .iter()
        .find(|option| option.id.to_string() == selection.config_id)?;

    match (&option.kind, &selection.value) {
        (SessionConfigKind::Select(select), AgentConfigValue::Select(value)) => {
            let supported = match &select.options {
                agent_client_protocol::schema::v1::SessionConfigSelectOptions::Ungrouped(
                    values,
                ) => values
                    .iter()
                    .any(|candidate| candidate.value.to_string() == *value),
                agent_client_protocol::schema::v1::SessionConfigSelectOptions::Grouped(groups) => {
                    groups.iter().any(|group| {
                        group
                            .options
                            .iter()
                            .any(|candidate| candidate.value.to_string() == *value)
                    })
                }
                _ => false,
            };

            supported.then(|| SessionConfigOptionValue::value_id(value.clone()))
        }
        (SessionConfigKind::Boolean(_), AgentConfigValue::Boolean(value)) => {
            Some(SessionConfigOptionValue::boolean(*value))
        }
        _ => None,
    }
}

/// Apply selections in order, using the complete option set returned after each change.
///
/// ACP agents can change the available reasoning levels when the model changes. Validating each
/// selection against the most recent response keeps dependent settings accurate and lets stale
/// persisted preferences fall back to the harness default.
pub(crate) async fn apply_session_config_options(
    connection: &ConnectionTo<Agent>,
    session_id: &SessionId,
    mut options: Vec<SessionConfigOption>,
    selections: &[AgentConfigSelection],
    mut on_skip: impl FnMut(&AgentConfigSelection),
) -> Result<Vec<SessionConfigOption>, agent_client_protocol::Error> {
    for selection in selections {
        let Some(value) = supported_value(&options, selection) else {
            on_skip(selection);
            continue;
        };

        let response = connection
            .send_request(SetSessionConfigOptionRequest::new(
                session_id.clone(),
                selection.config_id.clone(),
                value,
            ))
            .block_task()
            .await?;
        options = response.config_options;
    }

    Ok(options)
}

#[cfg(unix)]
fn kill_process_group(pid: u32) {
    if pid != 0 {
        // SAFETY: `pid` is the child process group created immediately before this guard.
        let _ = unsafe { libc::killpg(pid as i32, libc::SIGKILL) };
    }
}

#[cfg(not(unix))]
fn kill_process_group(_pid: u32) {}

struct ProbeProcessGuard {
    pid: u32,
    armed: bool,
}

impl ProbeProcessGuard {
    fn new(pid: u32) -> Self {
        Self { pid, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for ProbeProcessGuard {
    fn drop(&mut self) {
        if self.armed {
            kill_process_group(self.pid);
        }
    }
}

async fn probe_session_config_inner(
    command: String,
    args: Vec<String>,
    selections: Vec<AgentConfigSelection>,
) -> Result<Vec<SessionConfigOption>> {
    let mut process = Command::new(&command);
    process
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    #[cfg(unix)]
    {
        #[allow(unused_imports)]
        use std::os::unix::process::CommandExt;
        process.process_group(0);
    }

    let mut child = process.spawn().with_context(|| {
        format!(
            "Failed to spawn agent process: {command} {}",
            args.join(" ")
        )
    })?;
    let child_pid = child.id().unwrap_or(0);
    let mut process_guard = ProbeProcessGuard::new(child_pid);

    let stdin = child.stdin.take().context("Failed to get agent stdin")?;
    let stdout = child.stdout.take().context("Failed to get agent stdout")?;
    let stderr = child.stderr.take().context("Failed to get agent stderr")?;

    let stderr_lines = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let stderr_capture = stderr_lines.clone();
    tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if let Ok(mut captured) = stderr_capture.lock() {
                captured.push(line);
            }
        }
    });

    use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
    let transport = ByteStreams::new(stdin.compat_write(), stdout.compat());
    let temp_cwd = tempfile::tempdir().context("create ACP probe working directory")?;
    let cwd = temp_cwd.path().to_path_buf();

    let result = AcpClient
        .builder()
        .name("lareview-config-probe")
        .on_receive_notification(
            async move |_notification: SessionNotification, _connection| Ok(()),
            agent_client_protocol::on_receive_notification!(),
        )
        .connect_with(transport, async |connection| {
            connection
                .send_request(
                    InitializeRequest::new(ProtocolVersion::V1)
                        .client_info(agent_client_protocol::schema::v1::Implementation::new(
                            "lareview",
                            env!("CARGO_PKG_VERSION"),
                        ))
                        .client_capabilities(build_client_capabilities(false)),
                )
                .block_task()
                .await?;

            let session = connection
                .send_request(NewSessionRequest::new(cwd))
                .block_task()
                .await?;

            apply_session_config_options(
                &connection,
                &session.session_id,
                session.config_options.unwrap_or_default(),
                &selections,
                |_| {},
            )
            .await
        })
        .await;

    let _ = child.start_kill();
    kill_process_group(child_pid);
    let _ = child.wait().await;
    process_guard.disarm();

    result.map_err(|error| {
        let stderr = stderr_lines
            .lock()
            .map(|lines| lines.join("\n"))
            .unwrap_or_default();
        if stderr.is_empty() {
            anyhow::anyhow!("ACP config probe failed: {error:?}")
        } else {
            anyhow::anyhow!("ACP config probe failed: {error:?}\n{stderr}")
        }
    })
}

/// Start a short-lived session against the installed harness and return its advertised options.
pub async fn probe_agent_session_config(
    agent_id: &str,
    selections: Vec<AgentConfigSelection>,
) -> Result<Vec<SessionConfigOption>> {
    let candidate = super::list_agent_candidates()
        .into_iter()
        .find(|candidate| candidate.id == agent_id)
        .with_context(|| format!("Agent '{agent_id}' was not found"))?;
    let command = candidate.command.with_context(|| {
        format!(
            "Agent '{}' is not available. Configure its installed harness in Settings.",
            candidate.label
        )
    })?;
    let args = candidate.args;

    let (sender, receiver) = futures::channel::oneshot::channel();
    thread::spawn(move || {
        let runtime = Builder::new_current_thread().enable_all().build();
        let result = match runtime {
            Ok(runtime) => LocalSet::new().block_on(&runtime, async move {
                tokio::time::timeout(
                    Duration::from_secs(30),
                    probe_session_config_inner(command, args, selections),
                )
                .await
                .map_err(|_| anyhow::anyhow!("Agent config probe timed out after 30 seconds"))?
            }),
            Err(error) => Err(error.into()),
        };
        let _ = sender.send(result);
    });

    receiver.await.unwrap_or_else(|_| {
        Err(anyhow::anyhow!(
            "ACP config probe thread unexpectedly closed"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::schema::v1::{
        SessionConfigOptionCategory, SessionConfigSelectGroup, SessionConfigSelectOption,
    };

    #[test]
    fn validates_ungrouped_and_grouped_select_values() {
        let ungrouped = SessionConfigOption::select(
            "model",
            "Model",
            "one",
            vec![
                SessionConfigSelectOption::new("one", "One"),
                SessionConfigSelectOption::new("two", "Two"),
            ],
        )
        .category(SessionConfigOptionCategory::Model);
        let grouped = SessionConfigOption::select(
            "effort",
            "Effort",
            "low",
            vec![SessionConfigSelectGroup::new(
                "reasoning",
                "Reasoning",
                vec![SessionConfigSelectOption::new("low", "Low")],
            )],
        )
        .category(SessionConfigOptionCategory::ThoughtLevel);
        let options = vec![ungrouped, grouped];

        assert!(
            supported_value(
                &options,
                &AgentConfigSelection {
                    config_id: "model".into(),
                    value: AgentConfigValue::Select("two".into()),
                },
            )
            .is_some()
        );
        assert!(
            supported_value(
                &options,
                &AgentConfigSelection {
                    config_id: "effort".into(),
                    value: AgentConfigValue::Select("low".into()),
                },
            )
            .is_some()
        );
    }

    #[test]
    fn rejects_stale_or_wrongly_typed_values() {
        let options = vec![SessionConfigOption::select(
            "model",
            "Model",
            "one",
            vec![SessionConfigSelectOption::new("one", "One")],
        )];

        assert!(
            supported_value(
                &options,
                &AgentConfigSelection {
                    config_id: "model".into(),
                    value: AgentConfigValue::Select("removed".into()),
                },
            )
            .is_none()
        );
        assert!(
            supported_value(
                &options,
                &AgentConfigSelection {
                    config_id: "model".into(),
                    value: AgentConfigValue::Boolean(true),
                },
            )
            .is_none()
        );
    }
}
