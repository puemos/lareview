# Agent-to-Frontend Data Streaming

## Overview

This document explains how data flows from the ACP agent all the way to the React frontend, including the streaming, buffering, and rendering pipeline.

## End-to-End Data Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Agent Layer                                     │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │  Agent Process (Claude, OpenCode, etc.)                               │  │
│  │                                                                       │  │
│  │  Emits SessionUpdate messages over stdio:                            │  │
│  │  - AgentMessageChunk (streaming text)                                 │  │
│  │  - AgentThoughtChunk (reasoning)                                      │  │
│  │  - ToolCall (agent wants to call a tool)                              │  │
│  │  - ToolCallUpdate (tool result)                                       │  │
│  │  - Plan (structured task list)                                        │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                    │ stdio JSON                              │
│                                    ▼                                        │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                           Rust Backend Layer                                 │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │  LaReviewClient (client.rs)                                           │  │
│  │                                                                       │  │
│  │  1. Receives stdio messages from agent                               │  │
│  │  2. Parses SessionUpdate messages                                    │  │
│  │  3. Extracts streaming content (messages, thoughts)                  │  │
│  │  4. Concatenates chunks for same message ID                          │  │
│  │  5. Emits ProgressEvent to mpsc channel                              │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                    │                                         │
│                                    ▼ mpsc::unbounded_channel                 │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │  generate_review Tauri Command (commands/mod.rs)                      │  │
│  │                                                                       │  │
│  │  1. Creates mpsc channel                                             │  │
│  │  2. Spawns async task to forward events                              │  │
│  │  3. Transforms ProgressEvent → ProgressEventPayload                  │  │
│  │  4. Sends via Tauri Channel<T>                                       │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                    │                                         │
│                                    ▼ Tauri IPC Channel                       │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                         Frontend (React) Layer                               │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │  GenerateView.tsx                                                     │  │
│  │                                                                       │  │
│  │  1. Creates Tauri Channel                                            │  │
│  │  2. Sets up onmessage handler                                        │  │
│  │  3. Calls generateReview Tauri command                               │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                    │                                         │
│                                    ▼                                         │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │  useTauri Hook / Store                                               │  │
│  │                                                                       │  │
│  │  1. Receives ProgressEventPayload                                    │  │
│  │  2. Type guards for SessionUpdate variants                           │  │
│  │  3. Buffers streaming content (30ms batch)                           │  │
│  │  4. Concatenates chunks                                              │  │
│  │  5. Updates Zustand store                                            │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                    │                                         │
│                                    ▼                                         │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │  Timeline.tsx                                                        │  │
│  │                                                                       │  │
│  │  1. Receives messages from store                                     │  │
│  │  2. Filters system/debug messages                                    │  │
│  │  3. Virtualizes visible items (max 50)                               │  │
│  │  4. Renders message components:                                      │  │
│  │     - agent_message → MessageItem                                    │  │
│  │     - agent_thought → ThinkingItem                                   │  │
│  │     - tool_call → ToolCallItem                                       │  │
│  │     - agent_plan → PlanItem                                          │  │
│  │     - error → ErrorItem                                              │  │
│  │     - completed → CompletionItem                                     │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Step-by-Step Breakdown

### Step 1: Agent Emits SessionUpdate

The agent sends JSON messages over stdio:

```json
{
  "sessionUpdate": "agent_message_chunk",
  "content": {
    "type": "text",
    "text": "I'll analyze",
    "_meta": { "messageId": "msg-001" }
  }
}
```

```json
{
  "sessionUpdate": "agent_message_chunk",
  "content": {
    "type": "text",
    "text": " this diff and",
    "_meta": { "messageId": "msg-001" }
  }
}
```

```json
{
  "sessionUpdate": "agent_message_chunk",
  "content": {
    "type": "text",
    "text": " create a plan.",
    "_meta": { "messageId": "msg-001" }
  }
}
```

### Step 2: LaReviewClient Receives and Processes

In `src/infra/acp/task_generator/client.rs`:

```rust
async fn session_notification(&self, notification: SessionNotification) {
    match &notification.update {
        SessionUpdate::AgentMessageChunk(chunk) => {
            // Extract message ID from meta
            let (text, _) = self.append_streamed_content(
                &self.messages,
                &self.last_message_id,
                chunk.content.meta.as_ref(),
                &chunk.content.text,
            );

            // Forward to UI via progress channel
            if let Some(tx) = &self.progress {
                tx.send(ProgressEvent::Update(Box::new(
                    SessionUpdate::AgentMessageChunk(chunk.clone())
                )))?;
            }
        }
        // ... handle other update types
    }
}
```

### Step 3: ProgressEvent → Tauri Channel

In `src/commands/mod.rs`:

```rust
#[tauri::command]
pub async fn generate_review(
    state: State<'_, AppState>,
    diff_text: String,
    agent_id: String,
    on_progress: Channel<ProgressEventPayload>,
) -> Result<ReviewGenerationResult, String> {
    let (mcp_tx, mut mcp_rx) = mpsc::unbounded_channel::<ProgressEvent>();

    // Spawn async forwarder
    let on_progress_clone = on_progress.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(event) = mcp_rx.recv().await {
            let payload = match event {
                ProgressEvent::LocalLog(msg) => {
                    ProgressEventPayload::Log(msg)
                }
                ProgressEvent::Update(update) => {
                    if let SessionUpdate::Plan(plan) = *update {
                        // Transform Plan for frontend
                        let entries = plan.entries.iter().map(|e| {
                            FrontendPlanEntry {
                                content: e.content.clone(),
                                priority: e.priority.clone(),
                                status: e.status.clone(),
                            }
                        }).collect();
                        ProgressEventPayload::Plan(FrontendPlan { entries })
                    } else {
                        ProgressEventPayload::ServerUpdate(*update)
                    }
                }
                ProgressEvent::TaskStarted(id, title) => {
                    ProgressEventPayload::TaskStarted { task_id: id, title }
                }
                ProgressEvent::TaskAdded(id) => {
                    ProgressEventPayload::TaskCompleted { task_id: id }
                }
                ProgressEvent::Finalized => {
                    ProgressEventPayload::Completed { task_count: 0 }
                }
            };
            let _ = on_progress_clone.send(payload);
        }
    });

    // Call ACP worker with mcp_tx
    generate_tasks_with_acp(GenerateTasksInput {
        progress_tx: Some(mcp_tx),
        // ...
    })
}
```

### Step 4: Frontend Receives via Tauri Channel

In `frontend/src/components/Generate/GenerateView.tsx`:

```typescript
const onProgress = new Channel<ProgressEventPayload>();

onProgress.onmessage = (payload: ProgressEventPayload) => {
  switch (payload.event) {
    case "ServerUpdate":
      handleServerUpdate(payload.data as SessionUpdate);
      break;
    case "Log":
      addProgressMessage("log", payload.data as string);
      break;
    case "TaskStarted":
      addProgressMessage("task_started", (payload.data as { title: string }).title);
      break;
    case "TaskCompleted":
      addProgressMessage("task_added", "Task completed");
      break;
    case "Completed":
      addProgressMessage("completed", "Review generation complete!");
      break;
  }
};

const handleGenerate = async () => {
  await invoke('generate_review', {
    diffText: diffInput,
    agentId: selectedAgent.id,
    onProgress,
  });
};
```

### Step 5: Zustand Store Buffers and Updates

In `frontend/src/store/index.ts`:

```typescript
interface ProgressMessage {
  type: 'agent_message' | 'agent_thought' | 'tool_call' |
        'agent_plan' | 'error' | 'completed' | 'log' |
        'task_started' | 'task_added';
  message: string;
  data?: any;
  timestamp: number;
}

class ProgressBuffer {
  private messages: ProgressMessage[] = [];
  private pendingText: Map<string, string> = new Map();
  private flushTimer: ReturnType<typeof setTimeout> | null = null;
  private readonly flushIntervalMs = 30;

  constructor(onFlush: (messages: ProgressMessage[]) => void) {
    this.onFlush = onFlush;
  }

  // Concatenate streaming chunks for same message type
  appendMessage(type: string, message: string, _data?: any): void {
    if (type === 'agent_message' || type === 'agent_thought') {
      const existing = this.pendingText.get(type);
      if (existing !== undefined) {
        this.pendingText.set(type, existing + message);
        return; // Accumulate, don't flush yet
      }
    }

    this.flushPending();
    this.pendingText.set(type, message);
    this.scheduleFlush();
  }

  private scheduleFlush(): void {
    if (this.flushTimer) return;
    this.flushTimer = setTimeout(() => this.flush(), this.flushIntervalMs);
  }

  private flush(): void {
    this.flushTimer = null;
    this.onFlush([...this.messages]);
  }

  handleServerUpdate(update: SessionUpdate): void {
    if (isAgentMessageChunk(update)) {
      const text = update.content?.text || '';
      this.appendMessage('agent_message', text, update);
    } else if (isAgentThoughtChunk(update)) {
      const text = update.content?.text || '';
      this.appendMessage('agent_thought', text, update);
    } else if (isToolCall(update)) {
      this.appendMessage('tool_call', update.title, update);
    } else if (isPlan(update)) {
      this.appendMessage('agent_plan', 'Plan updated', update);
    }
  }
}
```

### Step 6: Timeline Renders Messages

In `frontend/src/components/Generate/Timeline.tsx`:

```typescript
const MAX_VISIBLE_ITEMS = 50;
const OVERSCAN_COUNT = 5;

export const Timeline: React.FC<TimelineProps> = ({ messages }) => {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [visibleStartIndex, setVisibleStartIndex] = useState(0);
  const [isAtBottom, setIsAtBottom] = useState(true);

  // Filter out system/debug messages
  const visibleMessages = messages.filter(m =>
    !['system', 'log', 'debug', 'task_started', 'task_added'].includes(m.type)
  );

  const totalItems = visibleMessages.length;
  const visibleEndIndex = Math.min(visibleStartIndex + MAX_VISIBLE_ITEMS, totalItems);
  const visible = visibleMessages.slice(visibleStartIndex, visibleEndIndex);

  // Auto-scroll to bottom when new messages arrive
  useEffect(() => {
    if (isAtBottom && scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages.length, isAtBottom]);

  // Handle scroll for virtualization
  const handleScroll = useCallback((e: React.UIEvent<HTMLDivElement>) => {
    const element = e.currentTarget;
    const atBottom = Math.abs(
      element.scrollHeight - element.scrollTop - element.clientHeight
    ) < 50;
    setIsAtBottom(atBottom);

    const scrollPosition = element.scrollTop;
    const estimatedItemHeight = 60;
    const newStartIndex = Math.floor(scrollPosition / estimatedItemHeight);
    setVisibleStartIndex(Math.max(0, newStartIndex - OVERSCAN_COUNT));
  }, []);

  return (
    <div className="flex flex-col h-full">
      <div
        className="flex-1 overflow-y-auto p-4 pb-20 scroll-smooth"
        ref={scrollRef}
        onScroll={handleScroll}
      >
        <div style={{ transform: `translateY(${visibleStartIndex * 60}px)` }}>
          {visible.map((msg, index) => {
            const actualIndex = visibleStartIndex + index;
            return <MessageItem key={actualIndex} {...msg} />;
          })}
        </div>
      </div>
    </div>
  );
};

function renderMessage(msg: ProgressMessage) {
  switch (msg.type) {
    case 'agent_message':
      return <MessageItem text={msg.message} />;
    case 'agent_thought':
      return <ThinkingItem text={msg.message} />;
    case 'tool_call':
      return <ToolCallItem data={msg.data} />;
    case 'agent_plan':
      return <PlanItem data={msg.data} />;
    case 'error':
      return <ErrorItem message={msg.message} />;
    case 'completed':
      return <CompletionItem />;
    default:
      return null;
  }
}
```

## Performance Optimizations

### 1. Streaming Concatenation

Messages arrive in chunks. We concatenate them before rendering to avoid flickering:

```
Without concatenation:  [H][e][l][l][o] → 5 re-renders
With concatenation:     [Hello] → 1 re-render
```

### 2. 30ms Buffer Flushing

Frontend buffers updates and flushes every 30ms (~33fps):

```typescript
private readonly flushIntervalMs = 30;
```

This batches multiple chunks into a single state update.

### 3. Virtualization

Only 50 items are rendered at a time:

```typescript
const MAX_VISIBLE_ITEMS = 50;
```

Even with thousands of messages, the DOM stays small.

### 4. Message Type Filtering

System messages are filtered out before rendering:

```typescript
const visibleMessages = messages.filter(m =>
  !['system', 'log', 'debug', 'task_started', 'task_added'].includes(m.type)
);
```

## Message Type Reference

| Type | Source | Purpose | Visible? |
|------|--------|---------|----------|
| `agent_message` | AgentMessageChunk | Agent responses | Yes |
| `agent_thought` | AgentThoughtChunk | Reasoning process | Yes |
| `tool_call` | ToolCall | Tool invocations | Yes |
| `tool_call_update` | ToolCallUpdate | Tool results | Sometimes |
| `agent_plan` | Plan | Structured task list | Yes |
| `log` | LocalLog | Debug output | No |
| `task_started` | TaskStarted | Task creation started | No |
| `task_added` | TaskAdded | Task persisted | No |
| `completed` | Finalized | Review complete | Yes |
| `error` | Error | Error occurred | Yes |

## Debugging the Pipeline

### Backend: Check ACP Events

```bash
RUST_LOG=acp=debug cargo tauri dev
```

Look for:
- `session update:` logs showing received ACP messages
- `ProgressEvent::` logs showing emitted events

### Frontend: Check Tauri Events

In browser DevTools console:

```javascript
// All progress events are logged
console.log('[Progress]', payload.event, payload);
```

### Frontend: Check Store Updates

```javascript
// Inspect the progress buffer
window.__lareviewStore.getState().progressMessages.length
```

## Error Scenarios

### 1. Agent Crashes

- ACP stdio stream closes unexpectedly
- `LaReviewClient` detects EOF and emits error
- Frontend shows error message

### 2. Channel Disconnect

- Tauri Channel closes
- Async forwarder exits
- Error logged, generation stops

### 3. JSON Parse Error

- Agent sends malformed JSON
- ACP layer catches error
- ProgressEvent::Error emitted

## Latency Breakdown

| Stage | Typical Latency |
|-------|-----------------|
| Agent generates chunk | 10-100ms |
| stdio to LaReviewClient | <1ms |
| ProgressEvent transform | <1ms |
| Tauri IPC serialization | 1-5ms |
| Frontend receive | 1-5ms |
| Buffer flush (30ms max) | 0-30ms |
| React render | 1-10ms |
| **Total (worst case)** | **~150ms** |

## Future Improvements

1. **WebWorker**: Move ProgressBuffer to WebWorker to avoid main thread blocking
2. **Binary Protocol**: Use MessagePack instead of JSON for smaller payloads
3. **Backpressure**: Request/response flow to slow agent when UI is overwhelmed
4. **Compression**: Compress large message chunks
5. **Metrics**: Add end-to-end latency monitoring
