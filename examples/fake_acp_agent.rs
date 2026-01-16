use agent_client_protocol::{
    Agent, AgentSideConnection, AuthenticateRequest, AuthenticateResponse, Client, Implementation,
    InitializeRequest, InitializeResponse, NewSessionRequest, NewSessionResponse, PromptRequest,
    PromptResponse, ProtocolVersion, SessionId, SessionNotification, SessionUpdate, StopReason,
    ToolCall, ToolCallId, ToolCallStatus, ToolCallUpdate, ToolCallUpdateFields,
};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::task::LocalSet;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

struct FakeAgent {
    notifications: mpsc::UnboundedSender<SessionNotification>,
    session_id: Arc<Mutex<SessionId>>,
}

#[async_trait::async_trait(?Send)]
impl Agent for FakeAgent {
    async fn initialize(
        &self,
        _args: InitializeRequest,
    ) -> agent_client_protocol::Result<InitializeResponse> {
        Ok(InitializeResponse::new(ProtocolVersion::V1)
            .agent_info(Implementation::new("fake-agent", "0.1.0")))
    }

    async fn authenticate(
        &self,
        _args: AuthenticateRequest,
    ) -> agent_client_protocol::Result<AuthenticateResponse> {
        Ok(AuthenticateResponse::new())
    }

    async fn new_session(
        &self,
        _args: NewSessionRequest,
    ) -> agent_client_protocol::Result<NewSessionResponse> {
        let session_id = SessionId::new("fake-session");
        *self.session_id.lock().unwrap() = session_id.clone();
        Ok(NewSessionResponse::new(session_id))
    }

    async fn prompt(&self, _args: PromptRequest) -> agent_client_protocol::Result<PromptResponse> {
        let session_id = self.session_id.lock().unwrap().clone();

        if std::env::var("PLAN_STRESS").is_ok() {
            // Send initial plan
            let plan1: agent_client_protocol::Plan = serde_json::from_value(serde_json::json!({
                "entries": [
                    {
                        "content": "Task 1",
                        "priority": "medium",
                        "status": "pending"
                    }
                ]
            }))
            .unwrap();

            let _ = self.notifications.send(SessionNotification::new(
                session_id.clone(),
                SessionUpdate::Plan(plan1),
            ));

            // Send update that preserves Task 1 and adds Task 2
            let plan2: agent_client_protocol::Plan = serde_json::from_value(serde_json::json!({
                "entries": [
                    {
                        "content": "Task 1",
                        "priority": "medium",
                        "status": "in_progress"
                    },
                    {
                        "content": "Task 2",
                        "priority": "low",
                        "status": "pending"
                    }
                ]
            }))
            .unwrap();

            let _ = self.notifications.send(SessionNotification::new(
                session_id.clone(),
                SessionUpdate::Plan(plan2),
            ));
        }

        let tool_call_id = ToolCallId::new("finalize_review");

        let tool_call = ToolCall::new(tool_call_id.clone(), "finalize_review");
        let _ = self.notifications.send(SessionNotification::new(
            session_id.clone(),
            SessionUpdate::ToolCall(tool_call),
        ));

        let mut fields = ToolCallUpdateFields::new();
        fields.status = Some(ToolCallStatus::Completed);
        fields.title = Some("finalize_review".to_string());
        fields.raw_input = Some(serde_json::json!({
            "title": "Fake Review",
            "summary": "Fake summary",
        }));

        let update = ToolCallUpdate::new(tool_call_id, fields);
        let _ = self.notifications.send(SessionNotification::new(
            session_id,
            SessionUpdate::ToolCallUpdate(update),
        ));

        Ok(PromptResponse::new(StopReason::EndTurn))
    }

    async fn cancel(
        &self,
        _args: agent_client_protocol::CancelNotification,
    ) -> agent_client_protocol::Result<()> {
        Ok(())
    }

    async fn load_session(
        &self,
        _args: agent_client_protocol::LoadSessionRequest,
    ) -> agent_client_protocol::Result<agent_client_protocol::LoadSessionResponse> {
        Ok(agent_client_protocol::LoadSessionResponse::new())
    }

    async fn set_session_mode(
        &self,
        _args: agent_client_protocol::SetSessionModeRequest,
    ) -> agent_client_protocol::Result<agent_client_protocol::SetSessionModeResponse> {
        Ok(agent_client_protocol::SetSessionModeResponse::new())
    }

    async fn ext_method(
        &self,
        _args: agent_client_protocol::ExtRequest,
    ) -> agent_client_protocol::Result<agent_client_protocol::ExtResponse> {
        Ok(agent_client_protocol::ExtResponse::new(
            serde_json::value::RawValue::from_string("null".to_string())?.into(),
        ))
    }

    async fn ext_notification(
        &self,
        _args: agent_client_protocol::ExtNotification,
    ) -> agent_client_protocol::Result<()> {
        Ok(())
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let local = LocalSet::new();
    local
        .run_until(async {
            let (tx, mut rx) = mpsc::unbounded_channel();
            let session_id = Arc::new(Mutex::new(SessionId::new("fake-session")));

            let agent = FakeAgent {
                notifications: tx,
                session_id,
            };

            let stdin = tokio::io::stdin().compat();
            let stdout = tokio::io::stdout().compat_write();
            let spawn_fn = |fut| {
                tokio::task::spawn_local(fut);
            };

            let (connection, io_task) = AgentSideConnection::new(agent, stdout, stdin, spawn_fn);

            let notify_task = tokio::task::spawn_local(async move {
                while let Some(notification) = rx.recv().await {
                    let _ = connection.session_notification(notification).await;
                }
            });

            let _ = io_task.await;
            notify_task.abort();
        })
        .await;

    Ok(())
}
