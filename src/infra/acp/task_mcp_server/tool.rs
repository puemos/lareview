use super::comment_ingest::save_agent_comment;
use super::config::ServerConfig;
use super::logging::log_to_file;
use super::task_ingest::{save_task, update_review_metadata};
use grep::{
    regex::RegexMatcherBuilder,
    searcher::{BinaryDetection, SearcherBuilder, sinks::Lossy},
};
use ignore::WalkBuilder;
use pmcp::{SimpleTool, ToolHandler};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

const DEFAULT_SEARCH_LIMIT: usize = 200;
const MAX_SEARCH_LIMIT: usize = 1000;
const DEFAULT_LIST_LIMIT: usize = 2000;
const MAX_LIST_LIMIT: usize = 5000;

/// Create the return_task tool for streaming task submission.
pub(super) fn create_return_task_tool(config: Arc<ServerConfig>) -> impl ToolHandler {
    SimpleTool::new("return_task", move |args: Value, _extra| {
        let config = config.clone();
        Box::pin(async move {
            log_to_file(&config, "return_task called");
            let raw_task = args.clone();

            let persist_config = config.clone();
            let persist_result = tokio::task::spawn_blocking(move || {
                save_task(&persist_config, raw_task)
            })
            .await;

            match persist_result {
                Ok(Ok(task)) => {
                    log_to_file(
                        &config,
                        &format!("ReturnTaskTool persisted task to DB: {}", task.id),
                    );

                    if let Some(path) = &config.tasks_out {
                        log_to_file(
                            &config,
                            &format!("ReturnTaskTool appending to {}", path.display()),
                        );

                        // Append the task as a JSON line to support streaming
                        let json_line = format!("{}\n", serde_json::to_string(&task).unwrap_or_default());
                        if let Err(e) = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(path)
                            .and_then(|mut file| std::io::Write::write_all(&mut file, json_line.as_bytes()))
                        {
                            log_to_file(&config, &format!("Failed to write to tasks out file: {}", e));
                        }
                        log_to_file(&config, "ReturnTaskTool append complete");
                    }

                    Ok(json!({
                        "status": "ok",
                        "message": format!("Task {} received successfully", task.id),
                        "task_id": task.id
                    }))
                },
                Ok(Err(err)) => {
                    log_to_file(
                        &config,
                        &format!("ReturnTaskTool failed to persist task: {err:?}"),
                    );

                    // Log the full error chain for better debugging
                    for cause in err.chain() {
                        log_to_file(&config, &format!("  Caused by: {cause}"));
                    }

                    // Check if this is a DiffIndexError and format appropriately
                    let error_msg = if let Some(diff_index_err) = err.downcast_ref::<crate::infra::diff_index::DiffIndexError>() {
                        // Return structured JSON error for agent parsing
                        diff_index_err.to_json()
                    } else {
                        format!("invalid return_task payload: {err:?}")
                    };

                    Err(pmcp::Error::Validation(error_msg))
                }
                Err(join_err) => {
                    log_to_file(
                        &config,
                        &format!("ReturnTaskTool task join error: {join_err}"),
                    );
                    Err(pmcp::Error::Internal(format!(
                        "return_task persistence join error: {join_err}"
                    )))
                }
            }
        })
    })
    .with_description(
        "Submit a single code review task for a pull request. Call this repeatedly to submit each task individually. \
         Each task must include: id, title, description, stats (risk, tags), and diff_refs. \
         The server computes files and line additions/deletions from the provided diff_refs using the canonical diff. \
         Optionally include sub_flow (grouping name). Diagram (D2 format) is required.",
    )
    .with_schema(single_task_schema())
}

/// Create the finalize_review tool for finalizing the review.
pub(super) fn create_finalize_review_tool(config: Arc<ServerConfig>) -> impl ToolHandler {
    SimpleTool::new("finalize_review", move |args: Value, _extra| {
        let config = config.clone();
        Box::pin(async move {
            log_to_file(&config, "finalize_review called");
            let persist_args = args.clone();
            let persist_args_for_log = persist_args.clone();

            let persist_config = config.clone();
            let persist_args_for_spawn = persist_args.clone(); // Clone before moving into spawn
            let persist_result = tokio::task::spawn_blocking(move || {
                update_review_metadata(&persist_config, persist_args_for_spawn)
            })
            .await;

            match persist_result {
                Ok(Ok(())) => {
                    log_to_file(
                        &config,
                        &format!("FinalizeReviewTool updated review metadata: {persist_args_for_log}"),
                    );

                    if let Some(path) = &config.tasks_out {
                        log_to_file(
                            &config,
                            &format!("FinalizeReviewTool writing metadata to {}", path.display()),
                        );

                        // Append the metadata as a final record
                        let metadata_record = json!({
                            "type": "review_metadata",
                            "title": persist_args_for_log.get("title").unwrap_or(&json!(null)),
                            "summary": persist_args_for_log.get("summary").unwrap_or(&json!(null))
                        });
                        let json_line = format!("{}\n", serde_json::to_string(&metadata_record).unwrap_or_default());
                        if let Err(e) = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(path)
                            .and_then(|mut file| std::io::Write::write_all(&mut file, json_line.as_bytes()))
                        {
                            log_to_file(&config, &format!("Failed to write metadata to tasks out file: {}", e));
                        }
                        log_to_file(&config, "FinalizeReviewTool write complete");
                    }

                    Ok(json!({ "status": "ok", "message": "Review finalized successfully" }))
                },
                Ok(Err(err)) => {
                    log_to_file(
                        &config,
                        &format!("FinalizeReviewTool failed to update metadata: {err:?}"),
                    );

                    // Log the full error chain for better debugging
                    for cause in err.chain() {
                        log_to_file(&config, &format!("  Caused by: {cause}"));
                    }

                    // Check if this is a DiffIndexError and format appropriately
                    let error_msg = if let Some(diff_index_err) = err.downcast_ref::<crate::infra::diff_index::DiffIndexError>() {
                        // Return structured JSON error for agent parsing
                        diff_index_err.to_json()
                    } else {
                        format!("invalid finalize_review payload: {err:?}")
                    };

                    Err(pmcp::Error::Validation(error_msg))
                }
                Err(join_err) => {
                    log_to_file(
                        &config,
                        &format!("FinalizeReviewTool task join error: {join_err}"),
                    );
                    Err(pmcp::Error::Internal(format!(
                        "finalize_review persistence join error: {join_err}"
                    )))
                }
            }
        })
    })
    .with_description(
        "Finalize the review by submitting the agent-generated review title/summary. \
         Call this once at the end of your analysis after all tasks have been submitted via return_task.",
    )
    .with_schema(review_metadata_schema())
}

/// Create the add_comment tool for submitting inline comments.
pub(super) fn create_add_comment_tool(config: Arc<ServerConfig>) -> impl ToolHandler {
    SimpleTool::new("add_comment", move |args: Value, _extra| {
        let config = config.clone();
        Box::pin(async move {
            log_to_file(&config, "add_comment called");
            let persist_args = args.clone();

            let persist_config = config.clone();
            let persist_result = tokio::task::spawn_blocking(move || {
                save_agent_comment(&persist_config, persist_args)
            })
            .await;

            match persist_result {
                Ok(Ok(thread_id)) => {
                    log_to_file(
                        &config,
                        &format!("AddCommentTool persisted thread: {}", thread_id),
                    );
                    Ok(json!({ "status": "ok", "message": "Comment added successfully", "thread_id": thread_id }))
                },
                Ok(Err(err)) => {
                    log_to_file(
                        &config,
                        &format!("AddCommentTool failed to save comment: {err:?}"),
                    );
                    Err(pmcp::Error::Validation(format!("invalid add_comment payload: {err}")))
                }
                Err(join_err) => {
                    log_to_file(
                        &config,
                        &format!("AddCommentTool task join error: {join_err}"),
                    );
                    Err(pmcp::Error::Internal(format!(
                        "add_comment persistence join error: {join_err}"
                    )))
                }
            }
        })
    })
    .with_description(
        "Add a specific, inline comment on a file line. Use this for targeted feedback (nitpicks, questions, suggestions) \
         that doesn't warrant a full task. Requires file, line, body. Optional title, impact, task_id.",
    )
    .with_schema(add_comment_schema())
}

#[derive(Debug, Deserialize)]
struct RepoSearchArgs {
    query: String,
    path: Option<String>,
    limit: Option<usize>,
    case_sensitive: Option<bool>,
    regex: Option<bool>,
    extensions: Option<Vec<String>>,
    include_hidden: Option<bool>,
}

#[derive(Debug, Serialize)]
struct RepoSearchMatch {
    path: String,
    line: u32,
    text: String,
}

#[derive(Debug, Deserialize)]
struct RepoListArgs {
    path: Option<String>,
    limit: Option<usize>,
    max_depth: Option<usize>,
    extensions: Option<Vec<String>>,
    include_dirs: Option<bool>,
    include_hidden: Option<bool>,
}

#[derive(Debug, Serialize)]
struct RepoListEntry {
    path: String,
    kind: String,
}

pub(super) fn create_repo_search_tool(config: Arc<ServerConfig>) -> impl ToolHandler {
    SimpleTool::new("repo_search", move |args: Value, _extra| {
        let config = config.clone();
        Box::pin(async move {
            log_to_file(&config, "repo_search called");
            let input: RepoSearchArgs = serde_json::from_value(args)
                .map_err(|err| pmcp::Error::Validation(err.to_string()))?;

            let query = input.query.trim();
            if query.is_empty() {
                return Err(pmcp::Error::Validation(
                    "repo_search requires non-empty query".to_string(),
                ));
            }

            let repo_root = config
                .repo_root
                .as_ref()
                .ok_or_else(|| pmcp::Error::InvalidState("repo root not configured".to_string()))?;

            let root_canon = repo_root
                .canonicalize()
                .map_err(|err| pmcp::Error::NotFound(err.to_string()))?;
            let search_root =
                resolve_repo_subpath(&root_canon, input.path.as_deref(), "repo_search")?;

            let limit = input
                .limit
                .unwrap_or(DEFAULT_SEARCH_LIMIT)
                .clamp(1, MAX_SEARCH_LIMIT);
            let case_sensitive = input.case_sensitive.unwrap_or(false);
            let use_regex = input.regex.unwrap_or(false);
            let include_hidden = input.include_hidden.unwrap_or(false);
            let extensions = normalize_extensions(&input.extensions);

            let matcher = build_matcher(query, case_sensitive, use_regex)?;
            let (matches, truncated) = search_repo(
                &root_canon,
                &search_root,
                &matcher,
                extensions.as_ref(),
                include_hidden,
                limit,
            )?;

            Ok(json!({
                "matches": matches,
                "truncated": truncated,
            }))
        })
    })
    .with_description(
        "Search the linked repository for a text query and return matching lines. \
         Accepts `query`, optional `path` (relative to repo root), optional `limit`, \
         optional `case_sensitive`, optional `regex`, optional `extensions`, and \
         optional `include_hidden`.",
    )
    .with_schema(repo_search_schema())
}

pub(super) fn create_repo_list_files_tool(config: Arc<ServerConfig>) -> impl ToolHandler {
    SimpleTool::new("repo_list_files", move |args: Value, _extra| {
        let config = config.clone();
        Box::pin(async move {
            log_to_file(&config, "repo_list_files called");
            let input: RepoListArgs = serde_json::from_value(args)
                .map_err(|err| pmcp::Error::Validation(err.to_string()))?;

            let repo_root = config
                .repo_root
                .as_ref()
                .ok_or_else(|| pmcp::Error::InvalidState("repo root not configured".to_string()))?;

            let root_canon = repo_root
                .canonicalize()
                .map_err(|err| pmcp::Error::NotFound(err.to_string()))?;
            let list_root =
                resolve_repo_subpath(&root_canon, input.path.as_deref(), "repo_list_files")?;

            let limit = input
                .limit
                .unwrap_or(DEFAULT_LIST_LIMIT)
                .clamp(1, MAX_LIST_LIMIT);
            let include_dirs = input.include_dirs.unwrap_or(false);
            let include_hidden = input.include_hidden.unwrap_or(false);
            let extensions = normalize_extensions(&input.extensions);

            let (entries, truncated) = list_repo_files(
                &root_canon,
                &list_root,
                extensions.as_ref(),
                include_dirs,
                include_hidden,
                input.max_depth,
                limit,
            )?;

            Ok(json!({
                "entries": entries,
                "truncated": truncated,
            }))
        })
    })
    .with_description(
        "List files under the linked repository. Accepts optional `path` (relative to repo root), \
         `limit`, `max_depth`, `extensions`, `include_dirs`, and `include_hidden`.",
    )
    .with_schema(repo_list_files_schema())
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                let _ = normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn resolve_repo_subpath(
    root: &Path,
    path: Option<&str>,
    tool: &str,
) -> Result<PathBuf, pmcp::Error> {
    let requested = match path {
        Some(path_str) => {
            let requested = Path::new(path_str);
            if requested.is_absolute() {
                requested.to_path_buf()
            } else {
                root.join(requested)
            }
        }
        None => root.to_path_buf(),
    };

    let normalized = normalize_path(&requested);
    if !normalized.starts_with(root) {
        return Err(pmcp::Error::Validation(format!(
            "{tool} path must be within repo root"
        )));
    }

    if !normalized.exists() {
        return Err(pmcp::Error::NotFound(format!(
            "{tool} path not found: {}",
            normalized.display()
        )));
    }

    Ok(normalized)
}

fn normalize_extensions(extensions: &Option<Vec<String>>) -> Option<HashSet<String>> {
    let list = extensions.as_ref()?;
    let mut normalized = HashSet::new();
    for ext in list {
        let cleaned = ext.trim().trim_start_matches('.');
        if !cleaned.is_empty() {
            normalized.insert(cleaned.to_ascii_lowercase());
        }
    }
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn matches_extension(path: &Path, extensions: &HashSet<String>) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    extensions.contains(&ext.to_ascii_lowercase())
}

fn build_matcher(
    query: &str,
    case_sensitive: bool,
    regex: bool,
) -> Result<grep::regex::RegexMatcher, pmcp::Error> {
    let mut builder = RegexMatcherBuilder::new();
    builder.case_insensitive(!case_sensitive);
    builder.line_terminator(Some(b'\n'));
    if !regex {
        builder.fixed_strings(true);
    }
    builder
        .build(query)
        .map_err(|err| pmcp::Error::Validation(err.to_string()))
}

fn build_walker(root: &Path, include_hidden: bool, max_depth: Option<usize>) -> ignore::Walk {
    let mut builder = WalkBuilder::new(root);
    builder.standard_filters(true);
    builder.hidden(!include_hidden);
    if let Some(depth) = max_depth {
        builder.max_depth(Some(depth));
    }
    builder.build()
}

fn search_repo(
    repo_root: &Path,
    search_root: &Path,
    matcher: &grep::regex::RegexMatcher,
    extensions: Option<&HashSet<String>>,
    include_hidden: bool,
    limit: usize,
) -> Result<(Vec<RepoSearchMatch>, bool), pmcp::Error> {
    let mut matches = Vec::new();
    let mut truncated = false;
    let mut searcher = SearcherBuilder::new()
        .binary_detection(BinaryDetection::quit(b'\x00'))
        .line_number(true)
        .build();

    let walk = build_walker(search_root, include_hidden, None);
    for result in walk {
        if truncated {
            break;
        }
        let entry = result.map_err(|err| pmcp::Error::NotFound(err.to_string()))?;
        let Some(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }

        let path = entry.path();
        if let Some(exts) = extensions
            && !matches_extension(path, exts)
        {
            continue;
        }

        let relative = path
            .strip_prefix(repo_root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        let mut sink = Lossy(|line_number, line| {
            let cleaned = line.trim_end_matches(&['\r', '\n'][..]).to_string();
            matches.push(RepoSearchMatch {
                path: relative.clone(),
                line: line_number as u32,
                text: cleaned,
            });
            if matches.len() >= limit {
                truncated = true;
                return Ok(false);
            }
            Ok(true)
        });

        searcher
            .search_path(matcher, path, &mut sink)
            .map_err(|err| pmcp::Error::Internal(err.to_string()))?;
    }

    Ok((matches, truncated))
}

fn list_repo_files(
    repo_root: &Path,
    list_root: &Path,
    extensions: Option<&HashSet<String>>,
    include_dirs: bool,
    include_hidden: bool,
    max_depth: Option<usize>,
    limit: usize,
) -> Result<(Vec<RepoListEntry>, bool), pmcp::Error> {
    let mut entries = Vec::new();
    let mut truncated = false;

    let walk = build_walker(list_root, include_hidden, max_depth);
    for result in walk {
        if truncated {
            break;
        }
        let entry = result.map_err(|err| pmcp::Error::NotFound(err.to_string()))?;
        let path = entry.path();
        let Some(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_dir() {
            if include_dirs && path != list_root {
                let relative = path
                    .strip_prefix(repo_root)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string();
                entries.push(RepoListEntry {
                    path: relative,
                    kind: "dir".to_string(),
                });
                if entries.len() >= limit {
                    truncated = true;
                }
            }
            continue;
        }

        if !file_type.is_file() {
            continue;
        }

        if let Some(exts) = extensions
            && !matches_extension(path, exts)
        {
            continue;
        }

        let relative = path
            .strip_prefix(repo_root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        entries.push(RepoListEntry {
            path: relative,
            kind: "file".to_string(),
        });

        if entries.len() >= limit {
            truncated = true;
        }
    }

    Ok((entries, truncated))
}

fn single_task_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {
                "type": "string",
                "description": "Short stable identifier for the task. Prefer descriptive IDs that include the sub-flow (e.g., 'auth-T1-missing-tests', 'payment-flow-T1-logic-check') or generic IDs like 'T1', 'T2'"
            },
            "title": {
                "type": "string",
                "description": "One-line summary of the review task in imperative mood (e.g., 'Verify authentication flow changes', 'Review database migration logic')"
            },
            "description": {
                "type": "string",
                "description": "2-6 sentences explaining: (1) what this sub-flow does in the system, (2) what changed in this PR, (3) where it appears in the code, (4) why it matters (correctness/safety/performance), (5) what reviewers should verify"
            },
            "stats": {
                "type": "object",
                "properties": {
                    "risk": {
                        "type": "string",
                        "enum": ["LOW", "MEDIUM", "HIGH"],
                        "description": "Risk level: HIGH for dangerous changes (security, data loss, breaking changes), MEDIUM for complex logic or refactors, LOW for safe mechanical changes"
                    },
                    "tags": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Descriptive tags for categorization (e.g., 'security', 'performance', 'refactor', 'bug-fix', 'needs-tests', 'breaking-change')"
                    }
                },
                "required": ["risk", "tags"],
                "description": "Risk and tags for this task. Additions, deletions, and files are computed from diff_refs."
            },
            "diff_refs": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "file": {
                            "type": "string",
                            "description": "File path in the diff (no a/ or b/ prefixes)"
                        },
                        "hunks": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "old_start": {"type": "integer"},
                                    "old_lines": {"type": "integer"},
                                    "new_start": {"type": "integer"},
                                    "new_lines": {"type": "integer"}
                                },
                                "required": ["old_start", "old_lines", "new_start", "new_lines"],
                                "description": "Hunk coordinates: (old_start, old_lines, new_start, new_lines)"
                            },
                            "description": "Hunk references: numeric coordinates, or empty array to select all hunks in the file"
                        }
                    },
                    "required": ["file", "hunks"],
                    "description": "Reference to specific hunks in the diff."
                },
                "description": "Array of references to specific hunks in the canonical diff. Each ref points to a specific file and range of lines."
            },
            "sub_flow": {
                "type": "string",
                "description": "Optional logical grouping name for this task. Use when multiple tasks belong to the same larger feature or concern (e.g., 'authentication-flow', 'data-migration', 'payment-processing'). Helps organize related tasks."
            },
            "diagram": {
                "type": "string",
                "description": "REQUIRED: D2 diagram visualizing the flow, sequence, architecture, or data model. Must be valid D2 syntax (e.g., 'Client -> API: Request\\nAPI -> DB: Query')"
            }
        },
        "required": ["id", "title", "description", "stats", "diff_refs", "diagram"]
    })
}

fn repo_search_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "Text to search for (literal by default unless regex=true)."
            },
            "path": {
                "type": "string",
                "description": "Optional subpath (file or directory) relative to the repo root."
            },
            "limit": {
                "type": "integer",
                "description": "Maximum number of matches to return (default 200, max 1000)."
            },
            "case_sensitive": {
                "type": "boolean",
                "description": "Whether the search should be case-sensitive (default false)."
            },
            "regex": {
                "type": "boolean",
                "description": "Interpret query as a regex (default false)."
            },
            "extensions": {
                "type": "array",
                "items": { "type": "string" },
                "description": "Optional list of file extensions to include (e.g., [\"rs\", \"ts\"])."
            },
            "include_hidden": {
                "type": "boolean",
                "description": "Whether to include hidden files (default false)."
            }
        },
        "required": ["query"]
    })
}

fn repo_list_files_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "path": {
                "type": "string",
                "description": "Optional subpath (file or directory) relative to the repo root."
            },
            "limit": {
                "type": "integer",
                "description": "Maximum number of entries to return (default 2000, max 5000)."
            },
            "max_depth": {
                "type": "integer",
                "description": "Optional maximum traversal depth."
            },
            "extensions": {
                "type": "array",
                "items": { "type": "string" },
                "description": "Optional list of file extensions to include (e.g., [\"rs\", \"ts\"])."
            },
            "include_dirs": {
                "type": "boolean",
                "description": "Whether to include directories in results (default false)."
            },
            "include_hidden": {
                "type": "boolean",
                "description": "Whether to include hidden files/directories (default false)."
            }
        }
    })
}

fn review_metadata_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "title": {
                "type": "string",
                "description": "Agent-generated review title. For GitHub PRs, this may match or improve the PR title."
            },
            "summary": {
                "type": "string",
                "description": "Optional short executive summary of the change and primary risks."
            }
        },
        "required": ["title"]
    })
}

fn add_comment_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "file": {
                "type": "string",
                "description": "Path to the file relative to repo root."
            },
            "line": {
                "type": "integer",
                "description": "Line number where the comment applies."
            },
            "side": {
                "type": "string",
                "enum": ["old", "new"],
                "description": "Which side of the diff (default: new)."
            },
            "body": {
                "type": "string",
                "description": "The comment content (Markdown supported)."
            },
            "title": {
                "type": "string",
                "description": "Short summary of the comment."
            },
            "impact": {
                "type": "string",
                "enum": ["nitpick", "blocking", "nice_to_have"],
                "description": "Severity of the comment (default: nitpick)."
            },
            "task_id": {
                "type": "string",
                "description": "Optional: Link this comment to a specific task ID."
            }
        },
        "required": ["file", "line", "body"]
    })
}
