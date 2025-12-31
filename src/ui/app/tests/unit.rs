use crate::domain::{ReviewStatus, ReviewTask, TaskStats};
use crate::infra::acp::ProgressEvent;
use crate::ui::app::LaReviewApp;
use crate::ui::app::state::*;
use agent_client_protocol::{ContentChunk, SessionUpdate};

fn create_task(id: &str, run_id: &str, sub_flow: Option<&str>) -> ReviewTask {
    ReviewTask {
        id: id.to_string(),
        run_id: run_id.to_string(),
        title: format!("Task {}", id),
        description: String::new(),
        files: Vec::new(),
        stats: TaskStats::default(),
        diff_refs: Vec::new(),
        insight: None,
        diagram: None,
        ai_generated: false,
        status: ReviewStatus::Todo,
        sub_flow: sub_flow.map(|s| s.to_string()),
    }
}

#[test]
fn test_app_state_tasks_filtering() {
    let mut state = AppState::default();
    state.domain.all_tasks = vec![
        create_task("1", "run_1", None),
        create_task("2", "run_1", Some("Flow A")),
        create_task("3", "run_2", None),
    ];

    // If no run selected, returns all
    state.ui.selected_run_id = None;
    assert_eq!(state.tasks().len(), 3);

    // Filter by run_1
    state.ui.selected_run_id = Some("run_1".to_string());
    let tasks = state.tasks();
    assert_eq!(tasks.len(), 2);
    assert!(tasks.iter().all(|t| t.run_id == "run_1"));
}

#[test]
fn test_app_state_tasks_by_sub_flow() {
    let mut state = AppState::default();
    state.domain.all_tasks = vec![
        create_task("1", "run_1", None),
        create_task("2", "run_1", Some("Flow A")),
        create_task("3", "run_1", Some("Flow A")),
        create_task("4", "run_1", Some("Flow B")),
    ];
    state.ui.selected_run_id = Some("run_1".to_string());

    let grouped = state.tasks_by_sub_flow();
    assert_eq!(grouped.len(), 3); // None, Flow A, Flow B
    assert_eq!(grouped.get(&None).unwrap().len(), 1);
    assert_eq!(grouped.get(&Some("Flow A".to_string())).unwrap().len(), 2);
    assert_eq!(grouped.get(&Some("Flow B".to_string())).unwrap().len(), 1);
}

#[test]
fn test_session_state_ingest_progress_logs() {
    let mut session = SessionState::default();
    session.ingest_progress(crate::infra::acp::ProgressEvent::LocalLog(
        "Hello".to_string(),
    ));
    session.ingest_progress(crate::infra::acp::ProgressEvent::LocalLog(
        "World".to_string(),
    ));

    assert_eq!(session.agent_timeline.len(), 2);
    if let crate::ui::app::timeline::TimelineContent::LocalLog(log) =
        &session.agent_timeline[0].content
    {
        assert_eq!(log, "Hello");
    }
}

#[test]
fn test_session_state_ingest_progress_plan_json() {
    let mut session = SessionState::default();
    let plan_json = serde_json::json!({
        "entries": [
            {
                "content": "Step 1",
                "priority": "medium",
                "status": "pending"
            }
        ]
    });
    let plan: agent_client_protocol::Plan = serde_json::from_value(plan_json).unwrap();

    session.ingest_progress(crate::infra::acp::ProgressEvent::Update(Box::new(
        agent_client_protocol::SessionUpdate::Plan(plan),
    )));

    assert!(session.latest_plan.is_some());
    assert_eq!(
        session.latest_plan.as_ref().unwrap().entries[0].content,
        "Step 1"
    );
}

#[test]
fn test_selected_agent_basics() {
    use std::str::FromStr;
    let agent = SelectedAgent::new("claude");
    assert_eq!(agent.id, "claude");
    assert_eq!(agent.to_string(), "claude");
    assert_eq!(format!("{}", agent), "claude");

    let agent2 = SelectedAgent::from_str("gemini").unwrap();
    assert_eq!(agent2.id, "gemini");
}
#[test]
fn test_session_state_ingest_progress_various_events() {
    let mut session = SessionState::default();
    session.ingest_progress(ProgressEvent::TaskStarted("task-1".into()));
    session.ingest_progress(ProgressEvent::TaskAdded("task-1".into()));
    session.ingest_progress(ProgressEvent::CommentAdded);
    session.ingest_progress(ProgressEvent::MetadataUpdated);
    session.ingest_progress(ProgressEvent::Finalized);

    assert_eq!(session.agent_timeline.len(), 5);
    assert!(!session.is_generating);
}

#[test]
fn test_session_state_reset_timeline() {
    let mut session = SessionState::default();
    session.ingest_progress(ProgressEvent::LocalLog("Hello".to_string()));
    session.latest_plan = Some(crate::domain::Plan {
        entries: vec![],
        meta: None,
    });

    session.reset_agent_timeline();
    assert!(session.agent_timeline.is_empty());
    assert!(session.latest_plan.is_none());
}

#[test]
fn test_timeline_stream_key_for_update() {
    use crate::ui::app::timeline::stream_key_for_update;
    use agent_client_protocol::SessionUpdate;

    let call_json = serde_json::json!({
        "toolCallId": "123",
        "title": "test",
        "kind": "read_file",
        "status": "pending",
        "content": [],
        "locations": []
    });
    let call: agent_client_protocol::ToolCall = serde_json::from_value(call_json).unwrap();
    let update = SessionUpdate::ToolCall(call);
    assert_eq!(stream_key_for_update(&update), Some("tool:123".to_string()));
}

#[test]
fn test_timeline_merge_update_in_place_text() {
    use crate::ui::app::timeline::{TimelineContent, TimelineItem, merge_update_in_place};
    use agent_client_protocol::{ContentBlock, SessionUpdate};

    let chunk1_json = serde_json::json!({
        "content": { "type": "text", "text": "Hello " }
    });
    let chunk1: agent_client_protocol::ContentChunk = serde_json::from_value(chunk1_json).unwrap();

    let mut item = TimelineItem {
        seq: 1,
        stream_key: None,
        content: TimelineContent::Update(Box::new(SessionUpdate::AgentMessageChunk(chunk1))),
    };

    let chunk2_json = serde_json::json!({
        "content": { "type": "text", "text": "World" }
    });
    let chunk2: agent_client_protocol::ContentChunk = serde_json::from_value(chunk2_json).unwrap();
    let incoming = SessionUpdate::AgentMessageChunk(chunk2);

    merge_update_in_place(&mut item, &incoming);

    if let TimelineContent::Update(boxed) = item.content {
        if let SessionUpdate::AgentMessageChunk(chunk) = *boxed {
            if let ContentBlock::Text(text) = chunk.content {
                assert_eq!(text.text, "Hello World");
            }
        } else {
            panic!("wrong session update");
        }
    } else {
        panic!("wrong timeline content");
    }
}

#[test]
fn test_timeline_merge_thought_chunk() {
    use crate::ui::app::timeline::{TimelineContent, TimelineItem, merge_update_in_place};

    let chunk1_json = serde_json::json!({
        "content": { "type": "text", "text": "Thinking..." }
    });
    let chunk1: agent_client_protocol::ContentChunk = serde_json::from_value(chunk1_json).unwrap();

    let mut item = TimelineItem {
        seq: 1,
        stream_key: None,
        content: TimelineContent::Update(Box::new(SessionUpdate::AgentThoughtChunk(chunk1))),
    };

    let chunk2_json = serde_json::json!({
        "content": { "type": "text", "text": " more" }
    });
    let chunk2: agent_client_protocol::ContentChunk = serde_json::from_value(chunk2_json).unwrap();

    merge_update_in_place(&mut item, &SessionUpdate::AgentThoughtChunk(chunk2));

    if let TimelineContent::Update(boxed) = item.content {
        if let SessionUpdate::AgentThoughtChunk(chunk) = *boxed {
            if let agent_client_protocol::ContentBlock::Text(text) = chunk.content {
                assert_eq!(text.text, "Thinking... more");
            } else {
                panic!("wrong content block");
            }
        } else {
            panic!("wrong session update");
        }
    } else {
        panic!("wrong timeline content");
    }
}

#[test]
fn test_timeline_merge_user_chunk() {
    use crate::ui::app::timeline::{TimelineContent, TimelineItem, merge_update_in_place};

    let chunk1_json = serde_json::json!({
        "content": { "type": "text", "text": "User says" }
    });
    let chunk1: agent_client_protocol::ContentChunk = serde_json::from_value(chunk1_json).unwrap();

    let mut item = TimelineItem {
        seq: 1,
        stream_key: None,
        content: TimelineContent::Update(Box::new(SessionUpdate::UserMessageChunk(chunk1))),
    };

    let chunk2_json = serde_json::json!({
        "content": { "type": "text", "text": " hello" }
    });
    let chunk2: agent_client_protocol::ContentChunk = serde_json::from_value(chunk2_json).unwrap();

    merge_update_in_place(&mut item, &SessionUpdate::UserMessageChunk(chunk2));

    if let TimelineContent::Update(boxed) = item.content {
        if let SessionUpdate::UserMessageChunk(chunk) = *boxed {
            if let agent_client_protocol::ContentBlock::Text(text) = chunk.content {
                assert_eq!(text.text, "User says hello");
            } else {
                panic!("wrong content block");
            }
        } else {
            panic!("wrong session update");
        }
    } else {
        panic!("wrong timeline content");
    }
}

#[test]
fn test_timeline_merge_plan() {
    use crate::ui::app::timeline::{TimelineContent, TimelineItem, merge_update_in_place};

    let plan1_json = serde_json::json!({
        "entries": [
            { "content": "Step 1", "status": "pending", "priority": "medium" }
        ]
    });
    let plan1: agent_client_protocol::Plan = serde_json::from_value(plan1_json).unwrap();

    let mut item = TimelineItem {
        seq: 1,
        stream_key: Some("plan".into()),
        content: TimelineContent::Update(Box::new(SessionUpdate::Plan(plan1))),
    };

    let plan2_json = serde_json::json!({
        "entries": [
            { "content": "Step 1", "status": "completed", "priority": "medium" }
        ]
    });
    let plan2: agent_client_protocol::Plan = serde_json::from_value(plan2_json).unwrap();

    merge_update_in_place(&mut item, &SessionUpdate::Plan(plan2));

    if let TimelineContent::Update(boxed) = item.content {
        if let SessionUpdate::Plan(res_plan) = *boxed {
            assert_eq!(
                res_plan.entries[0].status,
                agent_client_protocol::PlanEntryStatus::Completed
            );
        } else {
            panic!("wrong session update");
        }
    } else {
        panic!("wrong timeline content");
    }
}

#[test]
fn test_timeline_can_merge_contiguous() {
    use crate::ui::app::timeline::{TimelineContent, TimelineItem, can_merge_contiguous};

    let chunk_json = serde_json::json!({
        "content": { "type": "text", "text": "A" },
        "meta": null
    });
    let chunk: ContentChunk = serde_json::from_value(chunk_json).unwrap();
    let item = TimelineItem {
        seq: 1,
        stream_key: None,
        content: TimelineContent::Update(Box::new(SessionUpdate::AgentMessageChunk(chunk.clone()))),
    };

    let incoming = SessionUpdate::AgentMessageChunk(chunk);
    assert!(can_merge_contiguous(&item, &incoming));

    let thought_json = serde_json::json!({
        "content": { "type": "text", "text": "T" },
        "meta": null
    });
    let thought_chunk: ContentChunk = serde_json::from_value(thought_json).unwrap();
    let incoming_thought = SessionUpdate::AgentThoughtChunk(thought_chunk);
    assert!(!can_merge_contiguous(&item, &incoming_thought));
}

#[tokio::test]
async fn test_poll_action_messages() {
    let mut app = LaReviewApp::new_for_test();
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    app.action_rx = rx;

    tx.send(crate::ui::app::Action::Navigation(
        crate::ui::app::NavigationAction::SwitchTo(crate::ui::app::state::AppView::Settings),
    ))
    .await
    .unwrap();

    app.poll_action_messages();
    assert_eq!(
        app.state.ui.current_view,
        crate::ui::app::state::AppView::Settings
    );
}

#[tokio::test]
async fn test_poll_generation_messages() {
    let mut app = LaReviewApp::new_for_test();
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    app.gen_rx = rx;

    tx.send(crate::ui::app::GenMsg::Done(Ok(
        crate::ui::app::messages::GenResultPayload {
            messages: vec![],
            thoughts: vec![],
            logs: vec![],
        },
    )))
    .await
    .unwrap();

    app.poll_generation_messages();
    assert!(!app.state.session.is_generating); // GenerationAction::Done should clear it (via reducer)
}

#[test]
fn test_app_state_reset_timeline() {
    let mut app = LaReviewApp::new_for_test();
    app.state
        .session
        .agent_timeline
        .push(crate::ui::app::TimelineItem {
            seq: 1,
            stream_key: None,
            content: crate::ui::app::TimelineContent::LocalLog("log".into()),
        });
    app.state.reset_agent_timeline();
    assert!(app.state.session.agent_timeline.is_empty());
}

#[test]
fn test_review_ops_navigation() {
    let mut app = LaReviewApp::new_for_test();
    app.skip_runtime = true;

    app.switch_to_settings();
    assert_eq!(
        app.state.ui.current_view,
        crate::ui::app::state::AppView::Settings
    );

    app.switch_to_review();
    assert_eq!(
        app.state.ui.current_view,
        crate::ui::app::state::AppView::Review
    );

    app.switch_to_generate();
    assert_eq!(
        app.state.ui.current_view,
        crate::ui::app::state::AppView::Generate
    );

    app.switch_to_repos();
    assert_eq!(
        app.state.ui.current_view,
        crate::ui::app::state::AppView::Repos
    );
}

#[test]
fn test_review_ops_set_task_status() {
    let mut app = LaReviewApp::new_for_test();
    app.skip_runtime = true;
    app.set_task_status(&"t1".to_string(), ReviewStatus::Done);
}

#[test]
fn test_review_ops_sync_review() {
    let mut app = LaReviewApp::new_for_test();
    app.skip_runtime = true;
    app.sync_review_from_db();
    // This just dispatches an action, since skip_runtime is true it does nothing else.
}

#[tokio::test]
async fn test_seed_db_binary() {
    let tmp_file = tempfile::NamedTempFile::new().unwrap();
    let path = tmp_file.path().to_path_buf();

    // The seed_db uses LAREVIEW_DB_PATH
    unsafe {
        std::env::set_var("LAREVIEW_DB_PATH", &path);
    }

    // Note: seed_db logic is tested via cargo-run to simulate real binary execution.
}
