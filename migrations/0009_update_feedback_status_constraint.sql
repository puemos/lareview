-- Migration: Update threads table CHECK constraint for ReviewStatus
-- Version: 9
-- Description: Updates the threads table to accept new status values ('todo', 'in_progress', 'done', 'ignored')
--              while maintaining backward compatibility with old values ('wip', 'reject')

-- Create new threads table with updated CHECK constraint
CREATE TABLE threads_new (
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

-- Copy data from old table, mapping old status values to new ones
INSERT INTO threads_new 
SELECT 
    id, review_id, task_id, title,
    CASE status
        WHEN 'wip' THEN 'in_progress'
        WHEN 'reject' THEN 'ignored'
        ELSE status
    END as status,
    impact,
    anchor_file_path, anchor_line, anchor_side, anchor_hunk_ref, anchor_head_sha,
    author, created_at, updated_at
FROM threads;

-- Drop old table
DROP TABLE threads;

-- Rename new table
ALTER TABLE threads_new RENAME TO threads;

-- Recreate indexes
CREATE INDEX IF NOT EXISTS idx_threads_task_id ON threads(task_id);
CREATE INDEX IF NOT EXISTS idx_threads_review_id ON threads(review_id);
CREATE INDEX IF NOT EXISTS idx_threads_anchor ON threads(anchor_file_path, anchor_line);
