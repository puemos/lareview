use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::schema::v1::{
    AgentCapabilities, Implementation, InitializeRequest, InitializeResponse, NewSessionRequest,
    NewSessionResponse, Plan, PromptRequest, PromptResponse, SessionConfigOption,
    SessionConfigOptionCategory, SessionConfigSelectOption, SessionId, SessionNotification,
    SessionUpdate, SetSessionConfigOptionRequest, SetSessionConfigOptionResponse, StopReason,
    ToolCall, ToolCallId, ToolCallStatus, ToolCallUpdate, ToolCallUpdateFields,
};
use agent_client_protocol::{Agent, Stdio};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct FakeConfig {
    model: String,
    effort: String,
}

impl Default for FakeConfig {
    fn default() -> Self {
        Self {
            model: "fake-standard".to_string(),
            effort: "low".to_string(),
        }
    }
}

fn session_config_options(config: &FakeConfig) -> Vec<SessionConfigOption> {
    let efforts = if config.model == "fake-fast" {
        vec![SessionConfigSelectOption::new("high", "High")]
    } else {
        vec![
            SessionConfigSelectOption::new("low", "Low"),
            SessionConfigSelectOption::new("high", "High"),
        ]
    };

    vec![
        SessionConfigOption::select(
            "model",
            "Model",
            config.model.clone(),
            vec![
                SessionConfigSelectOption::new("fake-standard", "Fake Standard"),
                SessionConfigSelectOption::new("fake-fast", "Fake Fast"),
            ],
        )
        .category(SessionConfigOptionCategory::Model),
        SessionConfigOption::select(
            "reasoning_effort",
            "Reasoning effort",
            config.effort.clone(),
            efforts,
        )
        .category(SessionConfigOptionCategory::ThoughtLevel),
    ]
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let expected_config = args
        .iter()
        .position(|arg| arg == "--expect-config")
        .and_then(|index| Some((args.get(index + 1)?.clone(), args.get(index + 2)?.clone())));
    let config = Arc::new(Mutex::new(FakeConfig::default()));
    let new_session_config = config.clone();
    let set_config = config.clone();
    let prompt_config = config.clone();

    Agent
        .builder()
        .name("lareview-fake-agent")
        .on_receive_request(
            async move |request: InitializeRequest, responder, _connection| {
                responder.respond(
                    InitializeResponse::new(request.protocol_version)
                        .agent_info(Implementation::new("fake-agent", "0.1.0"))
                        .agent_capabilities(AgentCapabilities::new()),
                )
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |_request: NewSessionRequest, responder, _connection| {
                let options = session_config_options(&new_session_config.lock().unwrap());
                responder.respond(
                    NewSessionResponse::new(SessionId::new("fake-session")).config_options(options),
                )
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: SetSessionConfigOptionRequest, responder, _connection| {
                let mut config = set_config.lock().unwrap();
                let value = request
                    .value
                    .as_value_id()
                    .map(ToString::to_string)
                    .ok_or_else(agent_client_protocol::Error::invalid_params)?;

                match request.config_id.to_string().as_str() {
                    "model" if matches!(value.as_str(), "fake-standard" | "fake-fast") => {
                        config.model = value;
                        if config.model == "fake-fast" {
                            config.effort = "high".to_string();
                        }
                    }
                    "reasoning_effort"
                        if value == "high" || (value == "low" && config.model != "fake-fast") =>
                    {
                        config.effort = value;
                    }
                    _ => return Err(agent_client_protocol::Error::invalid_params()),
                }

                responder.respond(SetSessionConfigOptionResponse::new(session_config_options(
                    &config,
                )))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: PromptRequest, responder, connection| {
                let session_id = request.session_id;

                if let Some((expected_model, expected_effort)) = &expected_config {
                    let config = prompt_config.lock().unwrap();
                    if config.model != *expected_model || config.effort != *expected_effort {
                        return Err(agent_client_protocol::Error::invalid_params().data(
                            serde_json::json!({
                                "expectedModel": expected_model,
                                "expectedEffort": expected_effort,
                                "actualModel": config.model,
                                "actualEffort": config.effort,
                            }),
                        ));
                    }
                }

                if std::env::var("PLAN_STRESS").is_ok() {
                    let first: Plan = serde_json::from_value(serde_json::json!({
                        "entries": [{
                            "content": "Task 1",
                            "priority": "medium",
                            "status": "pending"
                        }]
                    }))
                    .map_err(agent_client_protocol::Error::into_internal_error)?;
                    connection.send_notification(SessionNotification::new(
                        session_id.clone(),
                        SessionUpdate::Plan(first),
                    ))?;

                    let second: Plan = serde_json::from_value(serde_json::json!({
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
                    .map_err(agent_client_protocol::Error::into_internal_error)?;
                    connection.send_notification(SessionNotification::new(
                        session_id.clone(),
                        SessionUpdate::Plan(second),
                    ))?;
                }

                let tool_call_id = ToolCallId::new("finalize_review");
                connection.send_notification(SessionNotification::new(
                    session_id.clone(),
                    SessionUpdate::ToolCall(ToolCall::new(tool_call_id.clone(), "finalize_review")),
                ))?;

                let mut fields = ToolCallUpdateFields::new();
                fields.status = Some(ToolCallStatus::Completed);
                fields.title = Some("finalize_review".to_string());
                fields.raw_input = Some(serde_json::json!({
                    "title": "Fake Review",
                    "summary": "Fake summary"
                }));
                connection.send_notification(SessionNotification::new(
                    session_id,
                    SessionUpdate::ToolCallUpdate(ToolCallUpdate::new(tool_call_id, fields)),
                ))?;

                responder.respond(PromptResponse::new(StopReason::EndTurn))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .connect_to(Stdio::new())
        .await?;

    let _ = ProtocolVersion::V1;
    Ok(())
}
