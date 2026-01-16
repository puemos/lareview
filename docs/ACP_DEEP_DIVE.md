# ACP (Agent Client Protocol) Deep Dive

## Overview

LaReview uses the **Agent Client Protocol (ACP)** to communicate with AI agents like Claude, OpenCode, and others. This document provides a detailed explanation of how ACP works in the LaReview system.

## What is ACP?

ACP is a standardized protocol for communicating with AI agents. It defines:
- Message formats for sending prompts and receiving responses
- Tool calling semantics for agents to invoke external tools
- Streaming capabilities for real-time progress updates
- Session management for maintaining agent context

## ACP Architecture in LaReview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           LaReview Application                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────────────┐ │
│  │ Tauri Command   │───▶│ ACP Task        │───▶│ MCP Task Server         │ │
│  │ generate_review │    │ Generator       │    │ (stdio JSON-RPC)        │ │
│  └────────┬────────┘    └────────┬────────┘    └────────────┬────────────┘ │
│           │                      │                          │               │
│           │                      │                          │ stdio         │
│           │                      │                          ▼               │
│           │                      │              ┌─────────────────────────┐ │
│           │                      │              │ ACP Agent               │ │
│           │                      │              │ (Claude, OpenCode, etc.)│ │
│           │                      │              └─────────────────────────┘ │
│           │                      │                         │               │
│           │                      ▼                         │               │
│           │              ┌─────────────────┐              │               │
│           │              │ ProgressEvent   │◀─────────────┘               │
│           │              │ Channel         │   streaming updates           │
│           │              └────────┬────────┘                              │
│           │                       │                                       │
│           ▼                       ▼                                       │
│  ┌─────────────────────────────────────────────────────────────┐         │
│  │                    Frontend (React)                          │         │
│  │  - Receives progress events via Tauri Channel               │         │
│  │  - Displays streaming messages in Timeline                  │         │
│  │  - Shows task creation in real-time                         │         │
│  └─────────────────────────────────────────────────────────────┘         │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Key Components

### 1. LaReviewClient (`src/infra/acp/task_generator/client.rs`)

The core ACP client that:
- Implements `agent_client_protocol::Client` trait
- Receives session notifications from the agent
- Manages streaming content (messages, thoughts, tool calls)
- Emits `ProgressEvent`s for UI updates

```rust
pub(super) struct LaReviewClient {
    pub(super) messages: Arc<Mutex<Vec<String>>>,
    pub(super) thoughts: Arc<Mutex<Vec<String>>>,
    pub(super) tasks: Arc<Mutex<Vec<ReviewTask>>>,
    pub(super) progress: Option<mpsc::UnboundedSender<ProgressEvent>>,
    // ... tracking fields for streaming
}
```

### 2. ACP Session Lifecycle

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Session Lifecycle                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  1. INITIALIZE                                                               │
│     ├─ Create LaReviewClient                                                │
│     ├─ Build agent command (with prompts, tools, capabilities)              │
│     └─ Spawn agent process via stdio                                        │
│                                                                              │
│  2. CONNECT                                                                  │
│     ├─ Send `SessionStart` message                                          │
│     ├─ Agent responds with `SessionInfo`                                    │
│     └─ Session is now ready for tool calls                                  │
│                                                                              │
│  3. RUN (Main Loop)                                                          │
│     ├─ Agent sends `SessionUpdate` messages:                                │
│     │   ├─ AgentMessageChunk (streaming text)                               │
│     │   ├─ AgentThoughtChunk (reasoning process)                            │
│     │   ├─ ToolCall (agent wants to use a tool)                             │
│     │   ├─ ToolCallUpdate (tool execution result)                           │
│     │   └─ Plan (structured task list)                                      │
│     │                                                                    │
│     └─ Client processes each update and emits ProgressEvent                │
│                                                                              │
│  4. TOOL CALLS (MCP Server)                                                  │
│     ├─ Agent calls `return_task`, `finalize_review`, etc.                   │
│     ├─ MCP server receives call via stdio JSON-RPC                          │
│     ├─ Server validates and executes the tool                               │
│     ├─ Server sends result back to agent                                    │
│     └─ ProgressEvent emitted to frontend                                    │
│                                                                              │
│  5. FINALIZE                                                                 │
│     ├─ Agent calls `finalize_review` tool                                   │
│     ├─ MCP server persists all tasks/feedback                               │
│     ├─ Agent sends `CompletionStatus`                                       │
│     └─ Client emits Finalized event                                         │
│                                                                              │
│  6. CLEANUP                                                                  │
│     ├─ Send `SessionEnd` message                                            │
│     ├─ Close stdio pipes                                                    │
│     └─ Terminate agent process                                              │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3. ProgressEvent Types (`src/infra/acp/task_generator/types.rs`)

```rust
#[derive(Debug, Clone)]
pub enum ProgressEvent {
    /// Raw ACP session update, streamed to the UI
    Update(Box<SessionUpdate>),
    /// Local log output from the ACP worker/process
    LocalLog(String),
    /// Signal that the agent has finished its work
    Finalized,
    /// A new task is being generated
    TaskStarted(String, String),
    /// A new task has been persisted
    TaskAdded(String),
    /// A comment has been persisted
    FeedbackAdded,
    /// Review metadata has been updated
    MetadataUpdated,
}
```

### 4. MCP Task Server (`src/infra/acp/task_mcp_server/`)

The MCP (Model Context Protocol) server provides tools that the agent can call:

| Tool | Purpose |
|------|---------|
| `return_task` | Agent returns a structured review task |
| `finalize_review` | Agent indicates review is complete |
| `add_comment` | Agent adds feedback on specific lines |
| `repo_search` | Agent searches repository files |
| `repo_list_files` | Agent lists directory contents |

### 5. Tool Capabilities (`src/infra/acp/task_generator/capabilities.rs`)

The agent's capabilities are restricted based on:
- Whether a repository is linked
- Whether the agent needs to request permission for file access
- Sandbox restrictions for untrusted code

## ACP Protocol Messages

### SessionStart

```json
{
  "sessionStart": {
    "agentId": "claude",
    "systemPrompt": "You are LaReview, a code review assistant..."
  }
}
```

### SessionUpdate (AgentMessageChunk)

```json
{
  "sessionUpdate": "agent_message_chunk",
  "content": {
    "type": "text",
    "text": "I'll analyze this diff and create a review plan.",
    "_meta": {
      "messageId": "msg-001"
    }
  }
}
```

### ToolCall

```json
{
  "sessionUpdate": "tool_call",
  "toolCallId": "tc-001",
  "title": "return_task",
  "kind": "other",
  "status": "in_progress",
  "rawInput": {
    "id": "T1",
    "title": "Fix memory leak in connection pool",
    "description": "...",
    "hunkIds": ["src/main.rs#H1"]
  }
}
```

## Streaming Content Concatenation

Messages from the agent arrive in chunks. LaReview concatenates them for a smooth streaming effect:

```
Timeline of Agent Message Streaming:
────────────────────────────────────
Agent: [chunk 1 "Hello"] ──→ Store: ["Hello"]
Agent: [chunk 2 " wor"] ──→ Store: ["Hello wor"]
Agent: [chunk 3 "ld!"] ──→ Store: ["Hello world!"]
```

Implementation in `client.rs`:

```rust
fn append_streamed_content(
    &self,
    store: &Arc<Mutex<Vec<String>>>,
    last_id: &Arc<Mutex<Option<String>>>,
    meta: Option<&Meta>,
    text: &str,
) -> (String, bool) {
    let chunk_id = Self::extract_chunk_id(meta);
    let mut id_guard = last_id.lock().unwrap();
    let mut store_guard = store.lock().unwrap();

    // New chunk ID → start new message
    if let Some(ref incoming) = chunk_id {
        if id_guard.as_deref() != Some(incoming.as_str()) {
            store_guard.push(String::new());
            *id_guard = Some(incoming.clone());
        }
    }

    // Append text to last message
    if let Some(last) = store_guard.last_mut() {
        last.push_str(text);
    }

    (store_guard.last().cloned().unwrap_or_default(), is_new)
}
```

## Agent Configuration

Agents are configured in `src/infra/acp/agent_discovery.rs`:

```rust
pub struct AgentCandidate {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}
```

Common agents:
- **Claude**: `claude --print`
- **OpenCode**: `opencode --agent`
- **Ollama**: `ollama serve` (local models)
- **Custom**: User-defined agents

## Error Handling

ACP errors propagate through the system:

1. **Agent Process Error**: Agent crashes or times out
2. **MCP Tool Error**: Tool validation or execution fails
3. **Serialization Error**: JSON parsing fails
4. **Channel Error**: Progress events can't be sent

All errors are caught and surfaced to the user via the frontend.

## Debugging ACP

### Backend Logs

```bash
RUST_LOG=acp=debug cargo tauri dev
```

### Frontend Logs

Check browser DevTools console for progress event logs.

### Common Issues

1. **Agent not starting**
   - Check agent path in settings
   - Verify agent binary is executable

2. **No progress updates**
   - Check `RUST_LOG=debug` output
   - Verify Tauri Channel is connected

3. **Tool calls failing**
   - Check MCP server logs
   - Verify tool parameters are correct

## Security Considerations

- ACP agents run locally as the user
- File system access is restricted to linked repositories
- Agent prompts are sanitized to prevent injection
- Network access is user-initiated only
