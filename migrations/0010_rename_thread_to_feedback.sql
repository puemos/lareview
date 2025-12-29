-- Migration: Rename threads to feedback
-- Version: 10
-- Description: Renames legacy 'threads' table and 'thread_id' columns to match new 'feedback' terminology

-- Disable foreign keys to allow table recreation and data movement
PRAGMA foreign_keys = OFF;

-- 1. Create feedback table if it doesn't exist
CREATE TABLE IF NOT EXISTS feedback (
    id TEXT PRIMARY KEY,
    review_id TEXT NOT NULL,
    task_id TEXT,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'todo' CHECK (status IN ('todo','in_progress','done','ignored','wip','reject')),
    impact TEXT NOT NULL DEFAULT 'nitpick' CHECK (impact IN ('blocking','nice_to_have','nitpick')),
    anchor_file_path TEXT,
    anchor_line INTEGER,
    anchor_side TEXT CHECK (anchor_side IN ('old','new')),
    anchor_hunk_ref TEXT,
    anchor_head_sha TEXT,
    author TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(review_id) REFERENCES reviews(id) ON DELETE CASCADE,
    FOREIGN KEY(task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

-- 2. Migrate data from threads to feedback if threads exists
INSERT OR IGNORE INTO feedback (
    id, review_id, task_id, title, status, impact, 
    anchor_file_path, anchor_line, anchor_side, anchor_hunk_ref, anchor_head_sha, 
    author, created_at, updated_at
)
SELECT 
    id, review_id, task_id, title, status, impact, 
    anchor_file_path, anchor_line, anchor_side, anchor_hunk_ref, anchor_head_sha, 
    author, created_at, updated_at
FROM threads 
WHERE EXISTS (SELECT 1 FROM sqlite_master WHERE type='table' AND name='threads');

-- 3. Rename thread_id column in comments table
CREATE TABLE comments_new (
    id TEXT PRIMARY KEY,
    feedback_id TEXT NOT NULL,
    author TEXT NOT NULL,
    body TEXT NOT NULL,
    parent_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(feedback_id) REFERENCES feedback(id) ON DELETE CASCADE,
    FOREIGN KEY(parent_id) REFERENCES comments_new(id) ON DELETE CASCADE
);

-- Copy data from old comments table. 
-- In v9, the column was definitively named 'thread_id'.
INSERT INTO comments_new (id, feedback_id, author, body, parent_id, created_at, updated_at)
SELECT id, thread_id, author, body, parent_id, created_at, updated_at
FROM comments;

DROP TABLE comments;
ALTER TABLE comments_new RENAME TO comments;

-- 4. Recreate indices for comments
CREATE INDEX IF NOT EXISTS idx_comments_feedback_id ON comments(feedback_id);
CREATE INDEX IF NOT EXISTS idx_comments_feedback_created_at ON comments(feedback_id, created_at);

-- 5. Cleanup legacy table
DROP TABLE IF EXISTS threads;

-- 6. Handle feedback_links if they exist
CREATE TABLE IF NOT EXISTS feedback_links (
    id TEXT PRIMARY KEY,
    feedback_id TEXT NOT NULL,
    provider TEXT NOT NULL,
    provider_feedback_id TEXT NOT NULL,
    provider_root_comment_id TEXT NOT NULL,
    last_synced_at TEXT NOT NULL,
    FOREIGN KEY(feedback_id) REFERENCES feedback(id) ON DELETE CASCADE
);

INSERT OR IGNORE INTO feedback_links (id, feedback_id, provider, provider_feedback_id, provider_root_comment_id, last_synced_at)
SELECT id, thread_id, provider, provider_feedback_id, provider_root_comment_id, last_synced_at
FROM thread_links
WHERE EXISTS (SELECT 1 FROM sqlite_master WHERE type='table' AND name='thread_links');

DROP TABLE IF EXISTS thread_links;

-- 7. Indices for feedback
CREATE INDEX IF NOT EXISTS idx_feedback_task_id ON feedback(task_id);
CREATE INDEX IF NOT EXISTS idx_feedback_review_id ON feedback(review_id);
CREATE INDEX IF NOT EXISTS idx_feedback_anchor ON feedback(anchor_file_path, anchor_line);

-- Re-enable foreign keys
PRAGMA foreign_keys = ON;
