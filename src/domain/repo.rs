use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LinkedRepo {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub remotes: Vec<String>,
    pub created_at: String,
    #[serde(default)]
    pub allow_snapshot_access: bool,
}
