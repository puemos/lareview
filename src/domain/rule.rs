use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuleScope {
    Global,
    Repo,
}

impl std::fmt::Display for RuleScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            RuleScope::Global => "global",
            RuleScope::Repo => "repo",
        };
        write!(f, "{value}")
    }
}

impl FromStr for RuleScope {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "global" => Ok(RuleScope::Global),
            "repo" => Ok(RuleScope::Repo),
            other => Err(format!("invalid rule scope: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewRule {
    pub id: String,
    pub scope: RuleScope,
    pub repo_id: Option<String>,
    pub glob: Option<String>,
    /// Category name for rules (e.g., "security", "breaking-changes")
    pub category: Option<String>,
    pub text: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedRule {
    pub id: String,
    pub scope: RuleScope,
    pub repo_id: Option<String>,
    pub glob: Option<String>,
    pub category: Option<String>,
    pub text: String,
    #[serde(default)]
    pub matched_files: Vec<String>,
    #[serde(default)]
    pub has_matches: bool,
}
