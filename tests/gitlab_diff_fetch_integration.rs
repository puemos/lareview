//! Integration test for `fetch_mr_diff`: verifies it calls the GitLab
//! `/diffs` endpoint via `glab api`, reconstructs a multi-file unified
//! diff from the JSON response, and produces output that `DiffIndex`
//! parses into the expected set of files.

#![cfg(unix)]

use lareview::infra::diff::index::DiffIndex;
use lareview::infra::vcs::gitlab::{GitLabMrRef, fetch_mr_diff};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::Mutex;
use tempfile::TempDir;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn write_fake_glab(dir: &std::path::Path, script: &str) {
    let glab_path = dir.join("glab");
    fs::write(&glab_path, script).expect("write fake glab");
    let mut perms = fs::metadata(&glab_path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&glab_path, perms).unwrap();
}

struct PathGuard {
    original: Option<std::ffi::OsString>,
}

impl PathGuard {
    // Prepend `new_path` to a minimal system PATH. The fake `glab` still
    // wins (tempdir comes first) but `/bin:/usr/bin` keeps `cat`, `printf`,
    // etc. available to the fake shell script — without them, heredocs
    // silently produce empty stdout.
    fn set(new_path: &std::path::Path) -> Self {
        let original = std::env::var_os("PATH");
        let combined = format!("{}:/bin:/usr/bin", new_path.display());
        unsafe {
            std::env::set_var("PATH", combined);
        }
        Self { original }
    }
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        unsafe {
            match self.original.take() {
                Some(v) => std::env::set_var("PATH", v),
                None => std::env::remove_var("PATH"),
            }
        }
    }
}

// Holding a std Mutex across the .await below is intentional — the guard
// serialises PATH/env mutation across tests that share the process.
#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn fetch_mr_diff_reconstructs_multi_file_diff_via_diffs_endpoint() {
    let _guard = ENV_LOCK.lock().unwrap();

    // Fake `glab`: rejects any `--raw` invocation and, for a `/diffs`
    // call that includes `--paginate --output ndjson`, emits a canned
    // two-file newline-delimited payload — the shape `glab api` would
    // produce after walking `Link: rel="next"` itself across all pages.
    let script = r#"#!/bin/sh
have_paginate=0
have_ndjson=0
prev=""
for arg in "$@"; do
  if [ "$arg" = "--raw" ]; then
    echo "reintroduced --raw; this path must not be used" >&2
    exit 1
  fi
  if [ "$arg" = "--paginate" ]; then
    have_paginate=1
  fi
  if [ "$prev" = "--output" ] && [ "$arg" = "ndjson" ]; then
    have_ndjson=1
  fi
  prev="$arg"
done

if [ "$1" != "api" ]; then
  echo "unexpected fake glab invocation: $*" >&2
  exit 2
fi

# Our call shape always has the endpoint as the last positional arg.
endpoint=""
for arg in "$@"; do
  endpoint="$arg"
done

case "$endpoint" in
  *diffs*)
    if [ "$have_paginate" != "1" ] || [ "$have_ndjson" != "1" ]; then
      echo "fetch_mr_diff must pass --paginate --output ndjson (perf: avoid N subprocess spawns)" >&2
      exit 1
    fi
    cat <<'NDJSON'
{"old_path":"README.md","new_path":"README.md","a_mode":"100644","b_mode":"100644","new_file":false,"renamed_file":false,"deleted_file":false,"diff":"@@ -1,2 +1,3 @@\n line one\n line two\n+line three\n"}
{"old_path":"docs/new.md","new_path":"docs/new.md","a_mode":null,"b_mode":"100644","new_file":true,"renamed_file":false,"deleted_file":false,"diff":"@@ -0,0 +1,2 @@\n+hello\n+world\n"}
NDJSON
    exit 0
    ;;
  *merge_requests*)
    cat <<'JSON'
{
  "title": "E2E: multi-file diff repro",
  "web_url": "http://localhost/e2e/raw-diffs-repro/-/merge_requests/1",
  "diff_refs": { "head_sha": "h", "base_sha": "b", "start_sha": "s" }
}
JSON
    exit 0
    ;;
esac

echo "unexpected endpoint: $endpoint" >&2
exit 2
"#;

    let tmp = TempDir::new().expect("tmp");
    write_fake_glab(tmp.path(), script);
    let _path = PathGuard::set(tmp.path());

    let mr = GitLabMrRef {
        host: "gitlab.com".to_string(),
        project_path: "e2e/raw-diffs-repro".to_string(),
        number: 1,
        url: "https://gitlab.com/e2e/raw-diffs-repro/-/merge_requests/1".to_string(),
    };

    let diff = fetch_mr_diff(&mr)
        .await
        .expect("fetch_mr_diff should succeed against /diffs endpoint");

    let header_count = diff
        .lines()
        .filter(|line| line.starts_with("diff --git "))
        .count();
    assert_eq!(
        header_count, 2,
        "expected exactly two `diff --git` headers, got {header_count}\n---\n{diff}\n---"
    );

    let index = DiffIndex::new(&diff).expect("unidiff parser should accept synthesised diff");
    assert!(
        index.files.contains_key("README.md"),
        "README.md missing from index; files={:?}",
        index.files.keys().collect::<Vec<_>>()
    );
    assert!(
        index.files.contains_key("docs/new.md"),
        "docs/new.md missing from index; files={:?}",
        index.files.keys().collect::<Vec<_>>()
    );
}
