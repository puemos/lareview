use crate::application::review::export::*;
use crate::application::review::ordering::*;
use crate::domain::*;
use std::sync::Arc;

#[tokio::test]
async fn test_export_to_markdown_basics() {
    let data = create_mock_data();
    let result = ReviewExporter::export_to_markdown(&data, false)
        .await
        .unwrap();

    let md = result.markdown;
    assert!(md.contains("# Test Review"));
    assert!(md.contains("This is a summary"));
    assert!(md.contains("## Overview"));
    assert!(md.contains("### Flow A"));
    assert!(md.contains("#### [ ] 🟡 First Task"));
    assert!(md.contains("AI Insight"));
    assert!(md.contains("##### Discussion"));
    assert!(md.contains("- [todo][nitpick] Thread Title"));
    assert!(md.contains("Author (2023-01-01T00:00:00Z): Comment body"));
}

#[tokio::test]
async fn test_export_to_markdown_full() {
    let data = ExportData {
        review: Review {
            id: "r1".into(),
            title: "Full Review".into(),
            summary: Some("Summary".into()),
            source: ReviewSource::DiffPaste {
                diff_hash: "h".into(),
            },
            active_run_id: Some("run1".into()),
            created_at: "now".into(),
            updated_at: "now".into(),
        },
        run: ReviewRun {
            id: "run1".into(),
            review_id: "r1".into(),
            agent_id: "agent1".into(),
            input_ref: "ref1".into(),
            diff_text: "diff".into(),
            diff_hash: "hash".into(),
            created_at: "now".into(),
        },
        tasks: vec![ReviewTask {
            id: "t1".into(),
            run_id: "run1".into(),
            title: "Task 1".into(),
            description: "Desc".into(),
            files: vec!["file.rs".into()],
            stats: TaskStats {
                risk: RiskLevel::High,
                additions: 10,
                deletions: 5,
                tags: vec![],
            },
            diff_refs: vec![DiffRef {
                file: "file.rs".into(),
                hunks: vec![HunkRef {
                    old_start: 1,
                    old_lines: 1,
                    new_start: 1,
                    new_lines: 1,
                }],
            }],
            insight: Some("AI Insight".into()),
            diagram: Some("x -> y".into()),
            ai_generated: true,
            status: ReviewStatus::Done,
            sub_flow: Some("Flow A".into()),
        }],
        threads: vec![Thread {
            id: "thread1".into(),
            review_id: "r1".into(),
            task_id: Some("t1".into()),
            title: "Discussion".into(),
            status: ReviewStatus::Todo,
            impact: ThreadImpact::Blocking,
            anchor: Some(ThreadAnchor {
                file_path: Some("file.rs".into()),
                line_number: Some(1),
                ..Default::default()
            }),
            author: "Agent".into(),
            created_at: "now".into(),
            updated_at: "now".into(),
        }],
        comments: vec![Comment {
            id: "c1".into(),
            thread_id: "thread1".into(),
            author: "User".into(),
            body: "Comment Body".into(),
            parent_id: None,
            created_at: "now".into(),
            updated_at: "now".into(),
        }],
    };

    let result = ReviewExporter::export_to_markdown(&data, false)
        .await
        .unwrap();
    assert!(result.markdown.contains("Full Review"));
    assert!(result.markdown.contains("🔴"));
    assert!(result.markdown.contains("AI Insight"));
    assert!(result.markdown.contains("Comment Body"));
    assert!(result.markdown.contains("blocking"));
}

#[test]
fn test_sub_flows_in_display_order() {
    let mut tasks_by_sub_flow = std::collections::HashMap::new();
    tasks_by_sub_flow.insert(
        Some("B".to_string()),
        vec![create_task(
            "1",
            "T1",
            None,
            ReviewStatus::Todo,
            RiskLevel::Low,
        )],
    );
    tasks_by_sub_flow.insert(
        None,
        vec![create_task(
            "2",
            "T2",
            None,
            ReviewStatus::Todo,
            RiskLevel::Low,
        )],
    );
    tasks_by_sub_flow.insert(
        Some("A".to_string()),
        vec![create_task(
            "3",
            "T3",
            None,
            ReviewStatus::Todo,
            RiskLevel::Low,
        )],
    );

    let ordered = sub_flows_in_display_order(&tasks_by_sub_flow);

    // Ordered by name: A, B, ZZZ (None)
    assert_eq!(ordered[0].0.as_deref(), Some("A"));
    assert_eq!(ordered[1].0.as_deref(), Some("B"));
    assert_eq!(ordered[2].0.as_deref(), None);
}

#[test]
fn test_tasks_in_sub_flow_display_order() {
    let tasks = vec![
        create_task(
            "1",
            "Closed High",
            None,
            ReviewStatus::Done,
            RiskLevel::High,
        ),
        create_task("2", "Open Low B", None, ReviewStatus::Todo, RiskLevel::Low),
        create_task("3", "Open High", None, ReviewStatus::Todo, RiskLevel::High),
        create_task(
            "4",
            "Open Low A",
            None,
            ReviewStatus::InProgress,
            RiskLevel::Low,
        ),
    ];

    let ordered = tasks_in_sub_flow_display_order(&tasks);

    assert_eq!(ordered[0].id, "3");
    assert_eq!(ordered[1].id, "4");
    assert_eq!(ordered[2].id, "2");
    assert_eq!(ordered[3].id, "1");
}

#[test]
fn test_tasks_in_display_order() {
    let mut tasks_by_sub_flow = std::collections::HashMap::new();
    tasks_by_sub_flow.insert(
        Some("B".to_string()),
        vec![create_task(
            "B1",
            "TB1",
            None,
            ReviewStatus::Todo,
            RiskLevel::Low,
        )],
    );
    tasks_by_sub_flow.insert(
        Some("A".to_string()),
        vec![create_task(
            "A1",
            "TA1",
            None,
            ReviewStatus::Todo,
            RiskLevel::Low,
        )],
    );

    let ordered = tasks_in_display_order(&tasks_by_sub_flow);

    assert_eq!(ordered[0].id, "A1");
    assert_eq!(ordered[1].id, "B1");
}

// Helpers

fn create_mock_data() -> ExportData {
    let review_id = "rev_1".to_string();
    let run_id = "run_1".to_string();

    ExportData {
        review: Review {
            id: review_id.clone(),
            title: "Test Review".to_string(),
            summary: Some("This is a summary".to_string()),
            source: ReviewSource::DiffPaste {
                diff_hash: "hash".to_string(),
            },
            active_run_id: Some(run_id.clone()),
            created_at: "2023-01-01T00:00:00Z".to_string(),
            updated_at: "2023-01-01T00:00:00Z".to_string(),
        },
        run: ReviewRun {
            id: run_id.clone(),
            review_id: review_id.clone(),
            agent_id: "agent_1".to_string(),
            input_ref: "input".to_string(),
            diff_text: Arc::from("diff"),
            diff_hash: "hash".to_string(),
            created_at: "2023-01-01T00:00:00Z".to_string(),
        },
        tasks: vec![ReviewTask {
            id: "task_1".to_string(),
            run_id: run_id.clone(),
            title: "First Task".to_string(),
            description: "Task description".to_string(),
            files: vec!["file.txt".to_string()],
            stats: TaskStats {
                additions: 10,
                deletions: 5,
                risk: RiskLevel::Medium,
                ..Default::default()
            },
            diff_refs: vec![DiffRef {
                file: "file.txt".to_string(),
                hunks: Vec::new(),
            }],
            insight: Some(Arc::from("AI Insight")),
            diagram: None,
            ai_generated: true,
            status: ReviewStatus::Todo,
            sub_flow: Some("Flow A".to_string()),
        }],
        threads: vec![Thread {
            id: "thread_1".to_string(),
            review_id: review_id.clone(),
            task_id: Some("task_1".to_string()),
            title: "Thread Title".to_string(),
            status: ReviewStatus::Todo,
            impact: ThreadImpact::Nitpick,
            anchor: None,
            author: "Author".to_string(),
            created_at: "2023-01-01T00:00:00Z".to_string(),
            updated_at: "2023-01-01T00:00:00Z".to_string(),
        }],
        comments: vec![Comment {
            id: "comment_1".to_string(),
            thread_id: "thread_1".to_string(),
            author: "Author".to_string(),
            body: "Comment body".to_string(),
            parent_id: None,
            created_at: "2023-01-01T00:00:00Z".to_string(),
            updated_at: "2023-01-01T00:00:00Z".to_string(),
        }],
    }
}

fn create_task(
    id: &str,
    title: &str,
    sub_flow: Option<&str>,
    status: ReviewStatus,
    risk: RiskLevel,
) -> ReviewTask {
    ReviewTask {
        id: id.to_string(),
        run_id: "run_1".to_string(),
        title: title.to_string(),
        description: String::new(),
        files: Vec::new(),
        stats: TaskStats {
            risk,
            ..Default::default()
        },
        diff_refs: Vec::new(),
        insight: None,
        diagram: None,
        ai_generated: false,
        status,
        sub_flow: sub_flow.map(|s| s.to_string()),
    }
}
