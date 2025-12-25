//! Integration tests for the review workflow functionality
//! These tests verify that different modules work together correctly

use lareview::application::review::ordering::*;
use lareview::domain::{ReviewStatus, ReviewTask, RiskLevel, TaskStats};
use std::collections::HashMap;

#[test]
fn test_full_review_ordering_workflow() {
    // Test that the ordering functions work together as expected
    let mut tasks_by_sub_flow: HashMap<Option<String>, Vec<ReviewTask>> = HashMap::new();

    // Create tasks in different sub-flows and with different statuses
    let task1 = ReviewTask {
        id: "task1".to_string(),
        run_id: "run1".to_string(),
        title: "High Risk Task".to_string(),
        status: ReviewStatus::Todo,
        stats: TaskStats {
            risk: RiskLevel::High,
            ..Default::default()
        },
        sub_flow: Some("Backend".to_string()),
        ..Default::default()
    };

    let task2 = ReviewTask {
        id: "task2".to_string(),
        run_id: "run1".to_string(),
        title: "Low Risk Task".to_string(),
        status: ReviewStatus::Done,
        stats: TaskStats {
            risk: RiskLevel::Low,
            ..Default::default()
        },
        sub_flow: Some("Frontend".to_string()),
        ..Default::default()
    };

    tasks_by_sub_flow.insert(Some("Backend".to_string()), vec![task1]);
    tasks_by_sub_flow.insert(Some("Frontend".to_string()), vec![task2]);

    // Test the full ordering workflow
    let ordered_tasks = tasks_in_display_order(&tasks_by_sub_flow);

    // Should have both tasks
    assert_eq!(ordered_tasks.len(), 2);

    // First task should be high risk (from open tasks)
    assert_eq!(ordered_tasks[0].id, "task1");

    // Second task should be the closed one
    assert_eq!(ordered_tasks[1].id, "task2");
}

#[test]
fn test_sub_flow_ordering() {
    let mut tasks_by_sub_flow: HashMap<Option<String>, Vec<ReviewTask>> = HashMap::new();

    let task_a = ReviewTask {
        id: "task_a".to_string(),
        sub_flow: Some("A".to_string()),
        ..Default::default()
    };

    let task_b = ReviewTask {
        id: "task_b".to_string(),
        sub_flow: Some("B".to_string()),
        ..Default::default()
    };

    let task_z = ReviewTask {
        id: "task_z".to_string(),
        sub_flow: Some("Z".to_string()),
        ..Default::default()
    };

    // No sub-flow task
    let task_none = ReviewTask {
        id: "task_none".to_string(),
        sub_flow: None,
        ..Default::default()
    };

    tasks_by_sub_flow.insert(Some("B".to_string()), vec![task_b.clone()]);
    tasks_by_sub_flow.insert(Some("A".to_string()), vec![task_a.clone()]);
    tasks_by_sub_flow.insert(Some("Z".to_string()), vec![task_z.clone()]);
    tasks_by_sub_flow.insert(None, vec![task_none.clone()]);

    let ordered = sub_flows_in_display_order(&tasks_by_sub_flow);

    // Should be ordered: A, B, Z, None (which maps to "ZZZ")
    assert_eq!(ordered[0].0.as_deref(), Some("A"));
    assert_eq!(ordered[1].0.as_deref(), Some("B"));
    assert_eq!(ordered[2].0.as_deref(), Some("Z"));
    assert_eq!(ordered[3].0.as_deref(), None);
}
