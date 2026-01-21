use super::config::ServerConfig;
use super::feedback_ingest::save_agent_comment;
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
                    let error_msg = if let Some(diff_index_err) = err.downcast_ref::<crate::infra::diff::index::DiffIndexError>() {
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
         Optionally include sub_flow (grouping name). Diagram is required.",
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
                    let error_msg = if let Some(diff_index_err) =
                        err.downcast_ref::<crate::infra::diff::index::DiffIndexError>()
                    {
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

/// Create the add_feedback tool for submitting inline feedback.
pub(super) fn create_add_feedback_tool(config: Arc<ServerConfig>) -> impl ToolHandler {
    SimpleTool::new("add_feedback", move |args: Value, _extra| {
        let config = config.clone();
        Box::pin(async move {
            log_to_file(&config, "add_feedback called");
            let persist_args = args.clone();

            let persist_config = config.clone();
            let persist_result = tokio::task::spawn_blocking(move || {
                save_agent_comment(&persist_config, persist_args)
            })
            .await;

            match persist_result {
                Ok(Ok(feedback_id)) => {
                    log_to_file(
                        &config,
                        &format!("AddFeedbackTool persisted feedback: {}", feedback_id),
                    );
                    Ok(json!({ "status": "ok", "message": "Feedback added successfully", "feedback_id": feedback_id }))
                },
                Ok(Err(err)) => {
                    log_to_file(
                        &config,
                        &format!("AddFeedbackTool failed to save feedback: {err:?}"),
                    );
                    Err(pmcp::Error::Validation(format!("invalid add_feedback payload: {err}")))
                }
                Err(join_err) => {
                    log_to_file(
                        &config,
                        &format!("AddFeedbackTool task join error: {join_err}"),
                    );
                    Err(pmcp::Error::Internal(format!(
                        "add_feedback persistence join error: {join_err}"
                    )))
                }
            }
        })
    })
    .with_description(
        "Add feedback on the diff—inline (specific line) or general (cross-cutting).\n\n\
         **Format:**\n\
         ```json\n\
         {\n\
           \"hunk_id\": \"src/auth.rs#H1\",\n\
           \"line_id\": \"L3\",\n\
           \"body\": \"Your comment here\",\n\
           \"impact\": \"blocking\",\n\
           \"confidence\": 0.9\n\
         }\n\
         ```\n\n\
         **Impact levels** (severity if the issue is real):\n\
         - `blocking`: Must fix before merge (security, correctness, data integrity)\n\
         - `nice_to_have`: Should fix, improves quality (missing tests, naming, tech debt)\n\
         - `nitpick`: Optional polish (style, minor typos)\n\n\
         **Confidence (0.0-1.0):** How certain you are this is a real issue (not a false positive).\n\
         - 0.9-1.0: High confidence - you're sure this is a real problem\n\
         - 0.7-0.89: Medium confidence - likely real but could be intentional\n\
         - 0.5-0.69: Low confidence - speculative, might be wrong\n\n\
         **General feedback:** For cross-cutting concerns, anchor to the most representative hunk and prefix body with \"**General feedback:**\"\n\n\
         **Optional fields:** title, impact (default: nitpick), confidence (default: 1.0), side (old|new, default: new), task_id",
    )
    .with_schema(add_feedback_schema())
}

/// Create the report_issue_check tool for reporting issue checklist verification results.
pub(super) fn create_report_issue_check_tool(config: Arc<ServerConfig>) -> impl ToolHandler {
    SimpleTool::new("report_issue_check", move |args: Value, _extra| {
        let config = config.clone();
        Box::pin(async move {
            log_to_file(&config, "report_issue_check called");
            let persist_args = args.clone();

            let persist_config = config.clone();
            let persist_result = tokio::task::spawn_blocking(move || {
                super::persistence::save_issue_check(&persist_config, persist_args)
            })
            .await;

            match persist_result {
                Ok(Ok(check_id)) => {
                    log_to_file(
                        &config,
                        &format!("ReportIssueCheckTool persisted check: {}", check_id),
                    );
                    Ok(json!({ "status": "ok", "message": "Issue check reported successfully", "check_id": check_id }))
                },
                Ok(Err(err)) => {
                    log_to_file(
                        &config,
                        &format!("ReportIssueCheckTool failed to save check: {err:?}"),
                    );
                    Err(pmcp::Error::Validation(format!("invalid report_issue_check payload: {err}")))
                }
                Err(join_err) => {
                    log_to_file(
                        &config,
                        &format!("ReportIssueCheckTool task join error: {join_err}"),
                    );
                    Err(pmcp::Error::Internal(format!(
                        "report_issue_check persistence join error: {join_err}"
                    )))
                }
            }
        })
    })
    .with_description(
        "Report the verification result for an issue checklist category.\n\n\
         **MANDATORY**: You must call this for every checklist item before finalizing the review.\n\n\
         **Format:**\n\
         ```json\n\
         {\n\
           \"category\": \"security\",\n\
           \"status\": \"found\",\n\
           \"confidence\": \"high\",\n\
           \"summary\": \"Found SQL injection vulnerability\",\n\
           \"findings\": [\n\
             {\n\
               \"title\": \"SQL injection in user query\",\n\
               \"description\": \"User input concatenated directly into SQL\",\n\
               \"evidence\": \"Line 45: query = 'SELECT * FROM users WHERE id=' + user_id\",\n\
               \"file_path\": \"src/db/users.rs\",\n\
               \"line_number\": 45,\n\
               \"impact\": \"blocking\"\n\
             }\n\
           ]\n\
         }\n\
         ```\n\n\
         **Status values:**\n\
         - `found`: Issues detected in this category\n\
         - `not_found`: Checked thoroughly, no issues\n\
         - `not_applicable`: Category doesn't apply to this PR\n\
         - `skipped`: Could not fully check\n\n\
         **Confidence:** `high`, `medium`, or `low`\n\n\
         **Note:** When reporting file locations in findings, both `file_path` AND `line_number` must be provided together.",
    )
    .with_schema(report_issue_check_schema())
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
                "description": "Risk and tags for this task. Additions, deletions, and files are computed from hunk_ids."
            },
            "hunk_ids": {
                "type": "array",
                "items": { "type": "string" },
                "description": "Array of hunk IDs referencing specific code sections. Format: 'path/to/file#H1' (e.g., 'src/auth.rs#H3'). Copy these directly from the hunk manifest above."
            },
            "sub_flow": {
                "type": "string",
                "description": "Optional logical grouping name for this task. Use when multiple tasks belong to the same larger feature or concern (e.g., 'authentication-flow', 'data-migration', 'payment-processing'). Helps organize related tasks."
            },
            "diagram": {
                "type": "string",
                "description": "REQUIRED: diagram string describing the flow, sequence, architecture, or data model. Must be valid Mermaid syntax."
            }
        },
        "required": ["id", "title", "description", "stats", "hunk_ids", "diagram"]
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

fn add_feedback_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "hunk_id": {
                "type": "string",
                "description": "The hunk ID from the manifest (e.g., 'src/auth.rs#H1'). Required for both line_id and line_content methods."
            },
            "line_id": {
                "type": "string",
                "description": "PREFERRED: The line ID from the manifest (e.g., 'L3'). Copy directly from the manifest - no string matching needed."
            },
            "line_content": {
                "type": "string",
                "description": "LEGACY: The exact line content to comment on. Only use if line_id is not available."
            },
            "file": {
                "type": "string",
                "description": "Path to the file relative to repo root. (Legacy format - prefer hunk_id)."
            },
            "line": {
                "type": "integer",
                "description": "Line number where the comment applies. (Legacy format - prefer hunk_id + line_id)."
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
                "description": "Severity of the issue if it's real (default: nitpick)."
            },
            "confidence": {
                "type": "number",
                "minimum": 0.0,
                "maximum": 1.0,
                "description": "How certain you are this is a real issue (0.0-1.0, default: 1.0). High (0.9-1.0): sure it's real. Medium (0.7-0.89): likely real. Low (0.5-0.69): speculative."
            },
            "task_id": {
                "type": "string",
                "description": "Optional: Link this comment to a specific task ID."
            },
            "rule_id": {
                "type": "string",
                "description": "Optional: Rule ID that motivated this feedback (include when applying a rule)."
            }
        },
        "required": ["body"]
    })
}

fn report_issue_check_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "category": {
                "type": "string",
                "description": "The category being checked (e.g., 'security', 'breaking-changes', 'performance'). Use the ID from the checklist."
            },
            "rule_id": {
                "type": "string",
                "description": "Optional rule ID if this check is for a custom checklist rule."
            },
            "display_name": {
                "type": "string",
                "description": "Human-readable name for the category (e.g., 'Security', 'Breaking Changes')."
            },
            "status": {
                "type": "string",
                "enum": ["found", "not_found", "not_applicable", "skipped"],
                "description": "Result of the check: found (issues detected), not_found (no issues), not_applicable (category doesn't apply), skipped (couldn't check)."
            },
            "confidence": {
                "type": "string",
                "enum": ["high", "medium", "low"],
                "description": "Confidence level in the assessment."
            },
            "summary": {
                "type": "string",
                "description": "Brief explanation of findings or why the category is N/A."
            },
            "findings": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "description": "Brief title of the finding."
                        },
                        "description": {
                            "type": "string",
                            "description": "Detailed description of the issue."
                        },
                        "evidence": {
                            "type": "string",
                            "description": "Code snippet or reasoning supporting the finding."
                        },
                        "file_path": {
                            "type": "string",
                            "description": "Path to the file where the issue was found. Must be provided with line_number."
                        },
                        "line_number": {
                            "type": "integer",
                            "description": "Line number where the issue occurs. Must be provided with file_path."
                        },
                        "impact": {
                            "type": "string",
                            "enum": ["blocking", "nice_to_have", "nitpick"],
                            "description": "Severity of the finding."
                        }
                    },
                    "required": ["title", "description", "evidence", "impact"]
                },
                "description": "Array of specific issues found (required if status is 'found')."
            }
        },
        "required": ["category", "status", "confidence"]
    })
}

/// Create the submit_learned_patterns tool for the learning compaction agent
pub(super) fn create_submit_learned_patterns_tool(config: Arc<ServerConfig>) -> impl ToolHandler {
    SimpleTool::new("submit_learned_patterns", move |args: Value, _extra| {
        let config = config.clone();
        Box::pin(async move {
            log_to_file(&config, "submit_learned_patterns called");
            let persist_args = args.clone();

            let persist_config = config.clone();
            let persist_result = tokio::task::spawn_blocking(move || {
                super::persistence::save_learned_patterns(&persist_config, persist_args)
            })
            .await;

            match persist_result {
                Ok(Ok(result)) => {
                    log_to_file(
                        &config,
                        &format!(
                            "SubmitLearnedPatternsTool: created={}, updated={}, errors={}",
                            result.patterns_created,
                            result.patterns_updated,
                            result.errors.len()
                        ),
                    );
                    Ok(json!({
                        "status": "ok",
                        "patterns_created": result.patterns_created,
                        "patterns_updated": result.patterns_updated,
                        "rejections_processed": result.rejections_processed,
                        "errors": result.errors
                    }))
                }
                Ok(Err(err)) => {
                    log_to_file(
                        &config,
                        &format!("SubmitLearnedPatternsTool failed: {err:?}"),
                    );
                    Err(pmcp::Error::Validation(format!(
                        "invalid submit_learned_patterns payload: {err}"
                    )))
                }
                Err(join_err) => {
                    log_to_file(
                        &config,
                        &format!("SubmitLearnedPatternsTool task join error: {join_err}"),
                    );
                    Err(pmcp::Error::Internal(format!(
                        "submit_learned_patterns persistence join error: {join_err}"
                    )))
                }
            }
        })
    })
    .with_description(
        "Submit learned patterns from rejection analysis. Used by the learning compaction agent \
         to record patterns of feedback that reviewers found unhelpful.\n\n\
         **Format:**\n\
         ```json\n\
         {\n\
           \"patterns\": [\n\
             {\n\
               \"pattern_text\": \"Don't flag unwrap() in test files\",\n\
               \"category\": \"testing\",\n\
               \"file_extension\": \"rs\",\n\
               \"source_count\": 5\n\
             },\n\
             {\n\
               \"pattern_text\": \"Avoid suggesting error handling in examples\",\n\
               \"category\": \"documentation\",\n\
               \"source_count\": 3,\n\
               \"merge_with_id\": \"pattern-abc123\"\n\
             }\n\
           ]\n\
         }\n\
         ```\n\n\
         **Fields:**\n\
         - `pattern_text`: The negative example (what to avoid flagging)\n\
         - `category`: Classification (testing, performance, style, error-handling, security, documentation, naming)\n\
         - `file_extension`: (optional) File extension if pattern applies to specific file types\n\
         - `source_count`: Number of rejections this pattern explains\n\
         - `merge_with_id`: (optional) ID of existing pattern to merge with instead of creating new",
    )
    .with_schema(submit_learned_patterns_schema())
}

fn submit_learned_patterns_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "patterns": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "pattern_text": {
                            "type": "string",
                            "description": "The negative example pattern (what to avoid flagging)."
                        },
                        "category": {
                            "type": "string",
                            "description": "Classification: testing, performance, style, error-handling, security, documentation, naming."
                        },
                        "file_extension": {
                            "type": "string",
                            "description": "File extension if pattern applies only to certain files (e.g., 'rs', 'ts')."
                        },
                        "source_count": {
                            "type": "integer",
                            "description": "Number of rejections this pattern explains."
                        },
                        "merge_with_id": {
                            "type": "string",
                            "description": "ID of existing pattern to merge with instead of creating a new one."
                        }
                    },
                    "required": ["pattern_text", "source_count"]
                },
                "description": "Array of learned patterns to submit."
            }
        },
        "required": ["patterns"]
    })
}

/// Create the finalize_learning tool to signal learning analysis completion.
pub(super) fn create_finalize_learning_tool(config: Arc<ServerConfig>) -> impl ToolHandler {
    SimpleTool::new("finalize_learning", move |_args: Value, _extra| {
        let config = config.clone();
        Box::pin(async move {
            log_to_file(&config, "finalize_learning called");
            Ok(json!({"status": "ok", "message": "Learning analysis complete"}))
        })
    })
    .with_description(
        "Signal that learning analysis is complete. Call this after submitting all learned patterns \
         via submit_learned_patterns to indicate the analysis is finished.",
    )
    .with_schema(json!({
        "type": "object",
        "properties": {},
        "additionalProperties": false
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_normalize_extensions() {
        let exts = Some(vec![
            ".rs".to_string(),
            "TS".to_string(),
            "  .js  ".to_string(),
        ]);
        let norm = normalize_extensions(&exts).unwrap();
        assert!(norm.contains("rs"));
        assert!(norm.contains("ts"));
        assert!(norm.contains("js"));
        assert_eq!(norm.len(), 3);

        assert_eq!(
            normalize_extensions(&Some(vec!["...rs".into()]))
                .unwrap()
                .iter()
                .next()
                .unwrap(),
            "rs"
        );

        assert!(normalize_extensions(&None).is_none());
        assert!(normalize_extensions(&Some(vec![])).is_none());
        assert!(normalize_extensions(&Some(vec![" ".into()])).is_none());
    }

    #[test]
    fn test_normalize_path_tool() {
        assert_eq!(normalize_path(Path::new("a/b/../c")), PathBuf::from("a/c"));
    }

    #[test]
    fn test_matches_extension() {
        let mut exts = HashSet::new();
        exts.insert("rs".to_string());
        assert!(matches_extension(Path::new("main.rs"), &exts));
        assert!(!matches_extension(Path::new("main.ts"), &exts));
    }

    #[test]
    fn test_build_matcher() {
        assert!(build_matcher("query", false, false).is_ok());
        assert!(build_matcher("query", true, true).is_ok());
    }

    #[test]
    fn test_resolve_repo_subpath() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().canonicalize().unwrap();

        assert_eq!(resolve_repo_subpath(&root, None, "test").unwrap(), root);
        assert_eq!(
            resolve_repo_subpath(&root, Some("."), "test").unwrap(),
            root
        );
    }

    #[test]
    fn test_resolve_repo_subpath_error() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        // Path outside root
        assert!(resolve_repo_subpath(root, Some("/etc/passwd"), "test").is_err());
        // Missing path
        assert!(resolve_repo_subpath(root, Some("missing"), "test").is_err());
    }

    #[test]
    fn test_search_repo_truncation() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let file = root.join("test.rs");
        std::fs::write(&file, "line1\nline2\nline3").unwrap();

        let matcher = build_matcher("line", false, false).unwrap();
        let (matches, truncated) = search_repo(root, root, &matcher, None, false, 2).unwrap();

        assert_eq!(matches.len(), 2);
        assert!(truncated);
    }

    #[test]
    fn test_list_repo_files_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        fs::create_dir(root.join("subdir")).unwrap();
        fs::write(root.join("subdir/file.rs"), "test").unwrap();

        let (entries, truncated) =
            list_repo_files(&root, &root, None, true, false, None, 10).unwrap();
        assert!(
            entries
                .iter()
                .any(|e| e.kind == "dir" && e.path == "subdir")
        );
        assert!(
            entries
                .iter()
                .any(|e| e.kind == "file" && e.path == "subdir/file.rs")
        );
        assert!(!truncated);
    }
}
