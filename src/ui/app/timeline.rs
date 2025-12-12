use agent_client_protocol::{ContentBlock, ContentChunk, Meta, SessionUpdate};

#[derive(Debug, Clone)]
pub struct TimelineItem {
    #[allow(dead_code)]
    pub seq: u64,
    pub stream_key: Option<String>,
    pub content: TimelineContent,
}

#[derive(Debug, Clone)]
pub enum TimelineContent {
    Update(Box<SessionUpdate>),
    LocalLog(String),
}

pub(super) fn stream_key_for_update(update: &SessionUpdate) -> Option<String> {
    match update {
        SessionUpdate::UserMessageChunk(chunk) => {
            extract_chunk_id(chunk.meta.as_ref()).map(|id| format!("user_msg:{id}"))
        }
        SessionUpdate::AgentMessageChunk(chunk) => {
            extract_chunk_id(chunk.meta.as_ref()).map(|id| format!("agent_msg:{id}"))
        }
        SessionUpdate::AgentThoughtChunk(chunk) => {
            extract_chunk_id(chunk.meta.as_ref()).map(|id| format!("agent_thought:{id}"))
        }
        SessionUpdate::ToolCall(call) => Some(format!("tool:{}", call.tool_call_id)),
        SessionUpdate::ToolCallUpdate(update) => Some(format!("tool:{}", update.tool_call_id)),
        SessionUpdate::Plan(_) => Some("plan".to_string()),
        _ => None,
    }
}

fn extract_chunk_id(meta: Option<&Meta>) -> Option<String> {
    meta.and_then(|meta| {
        ["message_id", "messageId", "id"]
            .iter()
            .find_map(|key| meta.get(*key).and_then(|val| val.as_str()))
            .map(|s| s.to_string())
    })
}

pub(super) fn can_merge_contiguous(existing: &TimelineItem, incoming: &SessionUpdate) -> bool {
    if existing.stream_key.is_some() {
        return false;
    }
    match (&existing.content, incoming) {
        (TimelineContent::Update(boxed_update), SessionUpdate::AgentMessageChunk(_))
            if matches!(**boxed_update, SessionUpdate::AgentMessageChunk(_)) =>
        {
            true
        }
        (TimelineContent::Update(boxed_update), SessionUpdate::AgentThoughtChunk(_))
            if matches!(**boxed_update, SessionUpdate::AgentThoughtChunk(_)) =>
        {
            true
        }
        (TimelineContent::Update(boxed_update), SessionUpdate::UserMessageChunk(_))
            if matches!(**boxed_update, SessionUpdate::UserMessageChunk(_)) =>
        {
            true
        }
        _ => false,
    }
}

pub(super) fn merge_update_in_place(existing: &mut TimelineItem, incoming: &SessionUpdate) {
    match (&mut existing.content, incoming) {
        (TimelineContent::Update(boxed_update), SessionUpdate::AgentMessageChunk(next)) => {
            if let SessionUpdate::AgentMessageChunk(prev) = &mut **boxed_update {
                merge_content_chunk(prev, next);
            } else {
                existing.content = TimelineContent::Update(Box::new(incoming.clone()));
            }
        }
        (TimelineContent::Update(boxed_update), SessionUpdate::AgentThoughtChunk(next)) => {
            if let SessionUpdate::AgentThoughtChunk(prev) = &mut **boxed_update {
                merge_content_chunk(prev, next);
            } else {
                existing.content = TimelineContent::Update(Box::new(incoming.clone()));
            }
        }
        (TimelineContent::Update(boxed_update), SessionUpdate::UserMessageChunk(next)) => {
            if let SessionUpdate::UserMessageChunk(prev) = &mut **boxed_update {
                merge_content_chunk(prev, next);
            } else {
                existing.content = TimelineContent::Update(Box::new(incoming.clone()));
            }
        }
        (TimelineContent::Update(boxed_update), SessionUpdate::ToolCallUpdate(update)) => {
            if let SessionUpdate::ToolCall(call) = &mut **boxed_update {
                call.update(update.fields.clone());
            } else {
                existing.content = TimelineContent::Update(Box::new(incoming.clone()));
            }
        }
        (TimelineContent::Update(boxed_update), SessionUpdate::ToolCall(call)) => {
            if let SessionUpdate::ToolCallUpdate(_existing_update) = &mut **boxed_update {
                **boxed_update = SessionUpdate::ToolCall(call.clone());
            } else if let SessionUpdate::ToolCall(existing_call) = &mut **boxed_update {
                *existing_call = call.clone();
            } else {
                existing.content =
                    TimelineContent::Update(Box::new(SessionUpdate::ToolCall(call.clone())));
            }
        }
        (TimelineContent::Update(boxed_update), SessionUpdate::Plan(plan)) => {
            if let SessionUpdate::Plan(existing_plan) = &mut **boxed_update {
                *existing_plan = plan.clone();
            } else {
                existing.content =
                    TimelineContent::Update(Box::new(SessionUpdate::Plan(plan.clone())));
            }
        }
        (_, _) => {
            existing.content = TimelineContent::Update(Box::new(incoming.clone()));
        }
    }
}

fn merge_content_chunk(existing: &mut ContentChunk, incoming: &ContentChunk) {
    match (&mut existing.content, &incoming.content) {
        (ContentBlock::Text(prev_text), ContentBlock::Text(next_text)) => {
            prev_text.text.push_str(&next_text.text);
        }
        _ => {
            existing.content = incoming.content.clone();
        }
    }
    if existing.meta.is_none() {
        existing.meta = incoming.meta.clone();
    }
}
