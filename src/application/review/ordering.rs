use crate::domain::ReviewTask;

pub fn sub_flows_in_display_order(
    tasks_by_sub_flow: &std::collections::HashMap<Option<String>, Vec<ReviewTask>>,
) -> Vec<(&Option<String>, &Vec<ReviewTask>)> {
    let mut sub_flows: Vec<_> = tasks_by_sub_flow.iter().collect();
    sub_flows.sort_by(|(name_a, _), (name_b, _)| {
        name_a
            .as_deref()
            .unwrap_or("ZZZ")
            .cmp(name_b.as_deref().unwrap_or("ZZZ"))
    });
    sub_flows
}

pub fn tasks_in_sub_flow_display_order(tasks: &[ReviewTask]) -> Vec<&ReviewTask> {
    let mut tasks_sorted: Vec<_> = tasks.iter().collect();
    tasks_sorted.sort_by_key(|t| {
        let is_closed = t.status.is_closed();
        (
            is_closed,
            std::cmp::Reverse(t.stats.risk.rank()),
            t.title.as_str(),
        )
    });
    tasks_sorted
}

pub fn tasks_in_display_order(
    tasks_by_sub_flow: &std::collections::HashMap<Option<String>, Vec<ReviewTask>>,
) -> Vec<&ReviewTask> {
    let mut out = Vec::new();
    for (_sub_flow_name, tasks) in sub_flows_in_display_order(tasks_by_sub_flow) {
        out.extend(tasks_in_sub_flow_display_order(tasks));
    }
    out
}
