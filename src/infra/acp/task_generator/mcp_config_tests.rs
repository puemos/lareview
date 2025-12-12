use std::path::{Path, PathBuf};

#[test]
fn resolve_prefers_mcp_server_binary_override() {
    let override_path = PathBuf::from("/tmp/custom-mcp-bin");
    let resolved =
        super::worker::resolve_task_mcp_server_path(Some(&override_path), Path::new("/fallback"));
    assert_eq!(resolved, override_path);
}
