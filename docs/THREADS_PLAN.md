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

## Behavior / Defaults
- New thread defaults: `status=todo`, `impact=nitpick`.
- Status transitions: any -> `todo|wip|done|reject`; `reject` can reopen to `todo`.
- Impact stays single-select; only threads (not individual comments) carry impact.
- Ordering: sort threads by anchor (file/line), then status priority (todo > wip > reject > done), then updated_at.
- Timeline is append-only; edits update `updated_at` on the thread and comment being edited.

## Data Model Changes (SQLite)
Add new tables for threads + comments.

### Table Shapes (SQLite)
- `threads`
  - `id TEXT PRIMARY KEY`
  - `review_id TEXT NOT NULL`
  - `task_id TEXT NULL`
  - `title TEXT NOT NULL`
  - `status TEXT NOT NULL CHECK (status IN ('todo','wip','done','reject'))`
  - `impact TEXT NOT NULL CHECK (impact IN ('blocking','nice_to_have','nitpick'))`
  - `anchor_file_path TEXT NULL`
  - `anchor_line INTEGER NULL`
  - `anchor_side TEXT NULL CHECK (anchor_side IN ('old','new'))`
  - `anchor_hunk_ref TEXT NULL` (json string with old/new ranges)
  - `anchor_head_sha TEXT NULL`
  - `author TEXT NOT NULL`
  - `created_at TEXT NOT NULL`
  - `updated_at TEXT NOT NULL`
  - indexes: `(task_id)`, `(review_id)`, `(anchor_file_path, anchor_line)`
- `comments`
  - `id TEXT PRIMARY KEY`
  - `thread_id TEXT NOT NULL REFERENCES threads(id) ON DELETE CASCADE`
  - `author TEXT NOT NULL`
  - `body TEXT NOT NULL`
  - `parent_id TEXT NULL` (future nesting)
  - `created_at TEXT NOT NULL`
  - `updated_at TEXT NOT NULL`
  - indexes: `(thread_id)`, `(thread_id, created_at)`
- `thread_links`
  - `id TEXT PRIMARY KEY`
  - `thread_id TEXT NOT NULL REFERENCES threads(id) ON DELETE CASCADE`
  - `provider TEXT NOT NULL`
  - `provider_thread_id TEXT NOT NULL`
  - `provider_root_comment_id TEXT NOT NULL`
  - `last_synced_at TEXT NOT NULL`

### New Tables
- `threads`
- `comments`
- `thread_links`

### Migration Strategy
None (fresh schema; threads/comments are the only discussion storage).

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

## Store + UI Integration
- Actions: `ThreadCreated`, `ThreadStatusChanged`, `ThreadImpactChanged`, `ThreadTitleEdited`, `ThreadCommentAdded`, `ThreadRejected`, `ThreadReopened`.
- Reducer: update in-memory thread list and comment timeline; enqueue DB `Command`s (insert thread/comment, update status/impact/title).
- Runtime: add SQLite commands for threads/comments; reuse existing refresh paths to reload after migration.
- Selection: keep current task selection; thread detail should tolerate review-level threads (no `task_id`).
- Notifications/errors: surface DB errors through existing `UiError` pipeline.

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

## Testing
- Schema smoke: create a thread + comment and read them back.
- Reducer tests: status/impact/title changes emit correct commands and mutate state predictably.
- UI smoke: render discussion list + thread detail with review-level and line-level anchors.
- Export test: markdown export includes status/impact header + comment timeline.

## Implementation Steps (Phase 1)
1. Add domain types: `ThreadStatus`, `ThreadImpact`, `Thread`, `Comment`.
2. Add schema for threads/comments.
3. Add repositories for threads/comments.
4. Wire store actions/commands + reducer updates; add tests for new actions.
5. Rebuild discussion list and thread detail UI, including anchor display + quick actions.
6. Add metadata parser/serializer (no sync yet) and export formatting.

## Risks / Watchouts
- Thread anchors must survive diff changes (store `head_sha` + `hunk_ref`).
- UI must handle review-level threads with no anchor.

## Open Questions
- Should review-level threads be shown in task views or a separate panel?
- Do we want to allow status edits from the thread list only, or also in-line?
