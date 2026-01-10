use crate::domain::{ResolvedRule, ReviewRule, RuleScope};
use globset::{GlobBuilder, GlobSetBuilder};

pub fn resolve_rules(
    rules: &[ReviewRule],
    repo_id: Option<&str>,
    diff_paths: &[String],
) -> Vec<ResolvedRule> {
    let mut resolved = Vec::new();

    for rule in rules {
        if !rule.enabled {
            continue;
        }

        match rule.scope {
            RuleScope::Global => {}
            RuleScope::Repo => {
                if rule.repo_id.as_deref() != repo_id {
                    continue;
                }
            }
        }

        let mut matched_files = Vec::new();
        if let Some(glob) = rule
            .glob
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            let glob = match GlobBuilder::new(glob).literal_separator(true).build() {
                Ok(glob) => glob,
                Err(err) => {
                    log::warn!("Skipping rule {} due to invalid glob: {}", rule.id, err);
                    continue;
                }
            };
            let mut set_builder = GlobSetBuilder::new();
            set_builder.add(glob);
            let set = match set_builder.build() {
                Ok(set) => set,
                Err(err) => {
                    log::warn!("Skipping rule {} due to invalid glob set: {}", rule.id, err);
                    continue;
                }
            };
            matched_files = diff_paths
                .iter()
                .filter(|path| set.is_match(path))
                .cloned()
                .collect();
            if matched_files.is_empty() {
                continue;
            }
        }

        resolved.push(ResolvedRule {
            id: rule.id.clone(),
            scope: rule.scope.clone(),
            repo_id: rule.repo_id.clone(),
            glob: rule.glob.clone(),
            text: rule.text.clone(),
            has_matches: !matched_files.is_empty(),
            matched_files,
        });
    }

    resolved
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(id: &str, scope: RuleScope, repo_id: Option<&str>, glob: Option<&str>) -> ReviewRule {
        ReviewRule {
            id: id.to_string(),
            scope,
            repo_id: repo_id.map(|r| r.to_string()),
            glob: glob.map(|g| g.to_string()),
            text: format!("rule {id}"),
            enabled: true,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn resolves_global_and_repo_scoped_rules() {
        let rules = vec![
            rule("g1", RuleScope::Global, None, None),
            rule("r1", RuleScope::Repo, Some("repo-1"), None),
            rule("r2", RuleScope::Repo, Some("repo-2"), None),
        ];
        let resolved = resolve_rules(&rules, Some("repo-1"), &[]);
        let ids: Vec<_> = resolved.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["g1", "r1"]);
    }

    #[test]
    fn filters_by_glob_matches() {
        let rules = vec![rule("g1", RuleScope::Global, None, Some("src/**/*.rs"))];
        let resolved = resolve_rules(
            &rules,
            None,
            &["src/main.rs".to_string(), "README.md".to_string()],
        );
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].matched_files, vec!["src/main.rs".to_string()]);
    }
}
