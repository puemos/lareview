//! Integration tests for the database functionality
//! These tests verify that different repository modules work together correctly

use lareview::domain::{
    Comment, DiffRef, Feedback, FeedbackImpact, HunkRef, LinkedRepo, Review, ReviewRun,
    ReviewRunStatus, ReviewSource, ReviewStatus, ReviewTask, TaskStats,
};
use lareview::infra::db::{Database, repository::*};
use rusqlite::{Connection, params};
use std::path::PathBuf;

#[test]
fn test_full_database_workflow() -> anyhow::Result<()> {
    // Test that all repository modules work together in a full workflow
    let db = Database::open_in_memory()?;
    let conn = db.connection();

    // Get all repositories
    let review_repo = ReviewRepository::new(conn.clone());
    let run_repo = ReviewRunRepository::new(conn.clone());
    let task_repo = TaskRepository::new(conn.clone());
    let feedback_repo = FeedbackRepository::new(conn.clone());
    let comment_repo = CommentRepository::new(conn.clone());
    let repo_repo = RepoRepository::new(conn.clone());

    // Create a linked repo
    let linked = LinkedRepo {
        id: "repo-1".into(),
        name: "test-repo".into(),
        path: PathBuf::from("/tmp/test"),
        remotes: vec!["https://github.com/test/repo".into()],
        created_at: "now".into(),
        allow_snapshot_access: false,
    };
    repo_repo.save(&linked)?;

    // Create a review
    let review = Review {
        id: "rev-1".to_string(),
        title: "Test Review".to_string(),
        summary: Some("Summary".into()),
        source: ReviewSource::DiffPaste {
            diff_hash: "h".into(),
        },
        active_run_id: None,
        status: ReviewStatus::Todo,
        created_at: "now".to_string(),
        updated_at: "now".to_string(),
    };
    review_repo.save(&review)?;

    // Create a review run
    let run = ReviewRun {
        id: "run-1".into(),
        review_id: "rev-1".into(),
        agent_id: "agent".into(),
        input_ref: "input".into(),
        diff_text: "diff".into(),
        diff_hash: "h".into(),
        status: ReviewRunStatus::Completed,
        created_at: "now".into(),
    };
    run_repo.save(&run)?;

    // Update review to use this run
    review_repo.set_active_run(&review.id, &run.id)?;

    // Create a task
    let task = ReviewTask {
        id: "task-1".to_string(),
        run_id: run.id.clone(),
        title: "Test Task".to_string(),
        description: "Desc".to_string(),
        files: vec![],
        stats: TaskStats::default(),
        diff_refs: vec![],
        insight: None,
        diagram: None,
        ai_generated: false,
        status: ReviewStatus::Todo,
        sub_flow: None,
    };
    task_repo.save(&task)?;

    // Create a feedback
    let feedback = Feedback {
        id: "t-1".into(),
        review_id: review.id.clone(),
        task_id: Some(task.id.clone()),
        rule_id: None,
        finding_id: None,
        category: None,
        title: "Feedback".into(),
        status: ReviewStatus::Todo,
        impact: FeedbackImpact::Nitpick,
        confidence: 1.0,
        anchor: None,
        author: "me".into(),
        created_at: "now".into(),
        updated_at: "now".into(),
    };
    feedback_repo.save(&feedback)?;

    // Create a comment
    let comment = Comment {
        id: "c-1".into(),
        feedback_id: feedback.id.clone(),
        author: "me".into(),
        body: "hello".into(),
        parent_id: None,
        created_at: "now".into(),
        updated_at: "now".into(),
    };
    comment_repo.save(&comment)?;

    // Verify all data can be retrieved correctly
    let retrieved_review = review_repo.find_by_id(&review.id)?.unwrap();
    assert_eq!(retrieved_review.title, "Test Review");

    let retrieved_run = run_repo.find_by_id(&run.id)?.unwrap();
    assert_eq!(retrieved_run.agent_id, "agent");

    let all_tasks = task_repo.find_all()?;
    assert_eq!(all_tasks.len(), 1);

    let all_feedbacks = feedback_repo.find_by_review(&review.id)?;
    assert_eq!(all_feedbacks.len(), 1);

    let feedback_comments = comment_repo.list_for_feedback(&feedback.id)?;
    assert_eq!(feedback_comments.len(), 1);
    assert_eq!(feedback_comments[0].body, "hello");

    // Test that repositories can be used together
    let review_tasks = task_repo.find_by_run(&run.id)?;
    assert_eq!(review_tasks.len(), 1);

    let review_feedbacks = feedback_repo.find_by_review(&review.id)?;
    assert_eq!(review_feedbacks.len(), 1);

    Ok(())
}

#[test]
fn test_task_diff_refs_serialization_deserialization() -> anyhow::Result<()> {
    let db = Database::open_in_memory()?;
    let conn = db.connection();
    let review_repo = ReviewRepository::new(conn.clone());
    let run_repo = ReviewRunRepository::new(conn.clone());
    let task_repo = TaskRepository::new(conn.clone());

    let review_id = "review-test-diff-refs".to_string();
    let run_id = "run-test-diff-refs".to_string();

    let review = Review {
        id: review_id.clone(),
        title: "Test Review".to_string(),
        summary: Some("Summary".into()),
        source: ReviewSource::DiffPaste {
            diff_hash: "h".into(),
        },
        active_run_id: None,
        status: ReviewStatus::Todo,
        created_at: "now".to_string(),
        updated_at: "now".to_string(),
    };
    review_repo.save(&review)?;

    let run = ReviewRun {
        id: run_id.clone(),
        review_id: review_id.clone(),
        agent_id: "agent".to_string(),
        input_ref: "input".to_string(),
        diff_text: "diff".into(),
        diff_hash: "h".to_string(),
        status: ReviewRunStatus::Completed,
        created_at: "now".to_string(),
    };
    run_repo.save(&run)?;

    let diff_refs = vec![
        DiffRef {
            file: "src/core/src/services/loader.ts".to_string(),
            hunks: vec![HunkRef {
                old_start: 1,
                old_lines: 4,
                new_start: 1,
                new_lines: 4,
            }],
        },
        DiffRef {
            file: "src/extension/extension-background/src/services/fetch-loader.ts".to_string(),
            hunks: vec![HunkRef {
                old_start: 1,
                old_lines: 11,
                new_start: 1,
                new_lines: 36,
            }],
        },
    ];

    let task = ReviewTask {
        id: "task-with-diff-refs".to_string(),
        run_id: run_id.clone(),
        title: "Test Task with DiffRefs".to_string(),
        description: "Testing diff_refs serialization".to_string(),
        files: vec![
            "src/core/src/services/loader.ts".to_string(),
            "src/extension/extension-background/src/services/fetch-loader.ts".to_string(),
        ],
        stats: TaskStats {
            additions: 31,
            deletions: 6,
            ..Default::default()
        },
        diff_refs: diff_refs.clone(),
        insight: None,
        diagram: None,
        ai_generated: true,
        status: ReviewStatus::Todo,
        sub_flow: None,
    };

    task_repo.save(&task)?;

    let retrieved_tasks = task_repo.find_by_run(&run_id)?;
    assert_eq!(retrieved_tasks.len(), 1);

    let retrieved_task = &retrieved_tasks[0];
    assert_eq!(retrieved_task.id, "task-with-diff-refs");
    assert_eq!(retrieved_task.diff_refs.len(), 2);

    for diff_ref in &retrieved_task.diff_refs {
        assert!(
            diff_refs.iter().any(|r| r.file == diff_ref.file),
            "File {} not found in original diff_refs",
            diff_ref.file
        );
        assert!(
            !diff_ref.hunks.is_empty(),
            "Hunks should not be empty for file {}",
            diff_ref.file
        );
    }

    assert_eq!(retrieved_task.files.len(), 2);
    assert!(
        retrieved_task
            .files
            .contains(&"src/core/src/services/loader.ts".to_string())
    );
    assert!(
        retrieved_task.files.contains(
            &"src/extension/extension-background/src/services/fetch-loader.ts".to_string()
        )
    );

    Ok(())
}

#[test]
fn test_migrates_legacy_custom_rules_feedback_fk() -> anyhow::Result<()> {
    let tmp_dir = tempfile::tempdir()?;
    let db_path = tmp_dir.path().join("db.sqlite");

    {
        let conn = Connection::open(&db_path)?;
        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;

            CREATE TABLE reviews (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                summary TEXT,
                source_json TEXT NOT NULL,
                active_run_id TEXT,
                status TEXT NOT NULL DEFAULT 'todo',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE tasks (
                id TEXT PRIMARY KEY
            );

            CREATE TABLE custom_rules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                rule_type TEXT NOT NULL,
                content TEXT NOT NULL,
                glob_pattern TEXT,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                is_global INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE feedback (
                id TEXT PRIMARY KEY,
                review_id TEXT NOT NULL,
                task_id TEXT,
                title TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'todo',
                impact TEXT NOT NULL DEFAULT 'nitpick',
                anchor_file_path TEXT,
                anchor_line INTEGER,
                anchor_side TEXT,
                anchor_hunk_ref TEXT,
                anchor_head_sha TEXT,
                author TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                rule_id TEXT,
                rule_name TEXT,
                anchor_diff_line_idx INTEGER,
                anchor_diff_hash TEXT,
                FOREIGN KEY(review_id) REFERENCES reviews(id) ON DELETE CASCADE,
                FOREIGN KEY(task_id) REFERENCES tasks(id) ON DELETE CASCADE,
                FOREIGN KEY(rule_id) REFERENCES custom_rules(id)
            );
            "#,
        )?;

        conn.execute(
            "INSERT INTO reviews (id, title, summary, source_json, active_run_id, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                "rev-1",
                "Legacy Review",
                Option::<String>::None,
                "{}",
                Option::<String>::None,
                "todo",
                "now",
                "now"
            ],
        )?;

        conn.execute(
            "INSERT INTO custom_rules (id, name, rule_type, content, glob_pattern, enabled, created_at, is_global)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                "rule-1",
                "Legacy Rule",
                "text",
                "Legacy rule content",
                Option::<String>::None,
                1,
                "now",
                1
            ],
        )?;

        conn.execute(
            "INSERT INTO feedback (id, review_id, title, status, impact, author, created_at, updated_at, rule_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                "fb-1",
                "rev-1",
                "Legacy Feedback",
                "todo",
                "nitpick",
                "agent",
                "now",
                "now",
                "rule-1"
            ],
        )?;
    }

    let db = Database::open_at(db_path)?;
    let feedback_repo = FeedbackRepository::new(db.connection());
    let feedback = feedback_repo.find_by_id("fb-1")?.expect("feedback exists");
    assert_eq!(feedback.rule_id, Some("rule-1".to_string()));

    let conn = db.connection();
    let conn = conn.lock().unwrap();
    let has_custom_rules = conn
        .prepare("SELECT 1 FROM sqlite_master WHERE type='table' AND name='custom_rules'")?
        .exists([])?;
    assert!(!has_custom_rules);

    let mut stmt = conn.prepare("PRAGMA foreign_key_list('feedback')")?;
    let targets: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(2))?
        .collect::<Result<_, _>>()?;
    assert!(!targets.iter().any(|target| target == "custom_rules"));

    Ok(())
}

#[test]
fn test_task_with_null_diff_refs() -> anyhow::Result<()> {
    let db = Database::open_in_memory()?;
    let conn = db.connection();
    let review_repo = ReviewRepository::new(conn.clone());
    let run_repo = ReviewRunRepository::new(conn.clone());
    let task_repo = TaskRepository::new(conn.clone());

    let review_id = "review-test-null-diff-refs".to_string();
    let run_id = "run-test-null-diff-refs".to_string();

    let review = Review {
        id: review_id.clone(),
        title: "Test Review".to_string(),
        summary: Some("Summary".into()),
        source: ReviewSource::DiffPaste {
            diff_hash: "h".into(),
        },
        active_run_id: None,
        status: ReviewStatus::Todo,
        created_at: "now".to_string(),
        updated_at: "now".to_string(),
    };
    review_repo.save(&review)?;

    let run = ReviewRun {
        id: run_id.clone(),
        review_id: review_id.clone(),
        agent_id: "agent".to_string(),
        input_ref: "input".to_string(),
        diff_text: "diff".into(),
        diff_hash: "h".to_string(),
        status: ReviewRunStatus::Completed,
        created_at: "now".to_string(),
    };
    run_repo.save(&run)?;

    let task = ReviewTask {
        id: "task-with-null-diff-refs".to_string(),
        run_id: run_id.clone(),
        title: "Test Task with Null DiffRefs".to_string(),
        description: "Testing null diff_refs handling".to_string(),
        files: vec!["some/file.ts".to_string()],
        stats: TaskStats::default(),
        diff_refs: vec![],
        insight: None,
        diagram: None,
        ai_generated: false,
        status: ReviewStatus::Todo,
        sub_flow: None,
    };

    task_repo.save(&task)?;

    let retrieved_tasks = task_repo.find_by_run(&run_id)?;
    assert_eq!(retrieved_tasks.len(), 1);

    let retrieved_task = &retrieved_tasks[0];
    assert_eq!(retrieved_task.diff_refs.len(), 0);
    assert_eq!(retrieved_task.files.len(), 1);

    Ok(())
}
