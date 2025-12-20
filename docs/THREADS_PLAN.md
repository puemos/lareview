# Threaded Feedback System (Sync-Ready Plan)

## Goals
- Thread-based discussion for feedback.
- Thread status: todo, wip, done, reject (reject can be reopened to todo).
- Thread impact: blocking, nice-to-have, nitpick (single-select).
- Support threads at any level: review-level, task-level, or line-level.
- Be ready for GitHub PR sync via embedded metadata in comments.

## Non-Goals (Phase 1)
- No GitHub webhook or App yet.
- No real-time multi-user collaboration.
- No advanced conflict resolution UI (capture and log only).

## Key Decisions
- Reject is not terminal; users can reopen to todo.
- Impact is single-select.
- Any anchor is allowed (including no anchor).

## Domain Model

### ThreadStatus
- `todo | wip | done | reject`

### ThreadImpact
- `blocking | nice_to_have | nitpick`

### Thread
- `id`
- `review_id`
- `task_id` (optional; allow review-level threads)
- `title`
- `status`
- `impact`
- `anchor` (optional)
  - `file_path` (optional)
  - `line_number` (optional)
  - `side` (optional: "old" | "new")
  - `hunk_ref` (optional: old_start, old_lines, new_start, new_lines)
  - `head_sha` (optional)
- `author`
- `created_at`, `updated_at`

### Comment
- `id`
- `thread_id`
- `author`
- `body`
- `parent_id` (optional, for future nesting)
- `created_at`, `updated_at`

### ThreadLink (future sync)
- `id`
- `thread_id`
- `provider` ("github")
- `provider_thread_id` (review thread id or issue comment id)
- `provider_root_comment_id`
- `last_synced_at`

## Data Model Changes (SQLite)
Add new tables and migrate existing notes into threads + comments.

### New Tables
- `threads`
- `comments`
- `thread_links`

### Migration Strategy
1. Group existing notes by `root_id` (fallback to file+line).
2. Create one `thread` per group.
3. Map note status to thread status:
   - open -> todo
   - resolved -> done
4. Map severity to impact:
   - blocking -> blocking
   - non-blocking -> nice_to_have
5. Convert notes into comments linked to the new thread.

## UI Plan (Egui)

### Discussion List (Task Detail)
- Show threads with:
  - status badge (todo/wip/done/reject)
  - impact pill (blocking/nice-to-have/nitpick)
  - title, anchor, reply count
  - quick actions: Done, Reject, Reopen

### Thread Detail
- Header controls:
  - status dropdown
  - impact selector
  - title edit
- Timeline of comments only.
- Reply composer at bottom.

### Diff Interaction
- New thread from line:
  - default status: todo
  - default impact: nitpick
  - quick edit for title/status/impact

## GitHub Sync Strategy (Later Phase)

### Metadata Embedding
Embed a hidden marker in the root GitHub comment:
```
<!-- lareview:thread_id=abc123;status=todo;impact=blocking;
anchor=path:line:side@sha;updated_at=2025-02-10T12:30:00Z -->
```

### Rules
- If marker exists, update thread fields based on metadata.
- If marker missing, create a thread with defaults and append marker on next sync.
- Thread resolve mapping:
  - done -> resolve review thread
  - todo/wip -> unresolve
  - reject -> keep open but add a rejection note

### Phased Sync
1. Local-only app with metadata support (Phase 1).
2. `gh api` poller for PR comments (Phase 2).
3. GitHub App + webhooks for real-time sync (Phase 3).

## Export Updates (Markdown)
Include thread status + impact and full comment timeline:
- Header: `[todo][blocking] Title`
- Comment list: `- author (timestamp): body`
- Anchor shown if available.

## Implementation Steps (Phase 1)
1. Add domain types: `ThreadStatus`, `ThreadImpact`, `Thread`, `Comment`.
2. Add schema + migration for threads/comments.
3. Add repositories for threads/comments.
4. Add store actions/commands for status/impact updates.
5. Rebuild discussion list and thread detail UI.
6. Add metadata parser/serializer (no sync yet).

## Risks / Watchouts
- Existing notes without root_id need reliable grouping (file+line fallback).
- Thread anchors must survive diff changes (store `head_sha` + `hunk_ref`).
- UI must handle review-level threads with no anchor.

## Open Questions
- Should review-level threads be shown in task views or a separate panel?
- Do we want to allow status edits from the thread list only, or also in-line?
