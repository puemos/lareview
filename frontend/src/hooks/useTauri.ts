import { invoke, Channel } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import type { EventCallback } from '@tauri-apps/api/event';
import type {
  ParsedDiff,
  ReviewTask,
  Comment,
  Agent,
  LinkedRepo,
  VcsStatus,
  EditorCandidate,
  EditorConfig,
  CliStatus,
  ReviewSource,
  ReviewRule,
  IssueCheckWithFindings,
  LibraryRule,
  DefaultIssueCategory,
  LibraryCategory,
  LearnedPattern,
  LearnedPatternInput,
  LearningStatus,
  LearningCompactionResult,
  MergeConfidence,
} from '../types';
import { useCallback } from 'react';

export interface TextContent {
  type: string;
  text: string;
  annotations?: Record<string, unknown>;
  meta?: Record<string, unknown>;
}

export interface AgentMessageChunk {
  content: TextContent;
  meta?: Record<string, unknown>;
}

export interface AgentThoughtChunk {
  content: TextContent;
  meta?: Record<string, unknown>;
}

export interface ToolCall {
  toolCallId: { id: string };
  title: string;
  rawInput?: Record<string, unknown> | string;
  rawOutput?: Record<string, unknown> | string;
  fields?: {
    title?: string;
    status?: string;
    rawInput?: Record<string, unknown> | string;
    rawOutput?: Record<string, unknown> | string;
  };
}

export interface Plan {
  entries: Array<{
    content: string;
    priority: string;
    status: string;
  }>;
}

export interface AvailableCommand {
  name: string;
  description?: string;
  input?: Record<string, unknown> | null;
  meta?: Record<string, unknown> | null;
}

export interface AvailableCommandsUpdate {
  availableCommands: AvailableCommand[];
  meta?: Record<string, unknown> | null;
}

export interface CurrentModeUpdate {
  mode: string;
  meta?: Record<string, unknown> | null;
}

export interface SessionUpdate {
  sessionUpdate: string;
  content?: TextContent;
  toolCallId?: { id: string };
  title?: string;
  rawInput?: Record<string, unknown> | string;
  rawOutput?: Record<string, unknown> | string;
  fields?: {
    title?: string;
    status?: string;
    rawInput?: Record<string, unknown> | string;
    rawOutput?: Record<string, unknown> | string;
  };
  availableCommands?: AvailableCommand[];
  meta?: Record<string, unknown> | null;
  mode?: string;
}

interface ReviewRuleInput {
  scope: 'global' | 'repo';
  repo_id?: string | null;
  glob?: string | null;
  category?: string | null;
  text: string;
  enabled: boolean;
}

export function isAgentMessageChunk(update: SessionUpdate): update is SessionUpdate & {
  sessionUpdate: 'agent_message_chunk';
  content: TextContent;
} {
  return update.sessionUpdate === 'agent_message_chunk' && !!update.content;
}

export function isAgentThoughtChunk(update: SessionUpdate): update is SessionUpdate & {
  sessionUpdate: 'agent_thought_chunk';
  content: TextContent;
} {
  return update.sessionUpdate === 'agent_thought_chunk' && !!update.content;
}

export function isToolCall(update: SessionUpdate): update is SessionUpdate & {
  sessionUpdate: 'tool_call';
  toolCallId: { id: string };
  title: string;
} {
  return update.sessionUpdate === 'tool_call';
}

export function isToolCallUpdate(update: SessionUpdate): update is SessionUpdate & {
  sessionUpdate: 'tool_call_update';
  toolCallId: { id: string };
} {
  return update.sessionUpdate === 'tool_call_update';
}

export function isPlan(update: SessionUpdate): update is SessionUpdate & { sessionUpdate: 'plan' } {
  return update.sessionUpdate === 'plan';
}

export function isAvailableCommandsUpdate(update: SessionUpdate): update is SessionUpdate & {
  sessionUpdate: 'available_commands_update';
  availableCommands: AvailableCommand[];
} {
  return update.sessionUpdate === 'available_commands_update';
}

export function isCurrentModeUpdate(update: SessionUpdate): update is SessionUpdate & {
  sessionUpdate: 'current_mode_update';
  mode: string;
} {
  return update.sessionUpdate === 'current_mode_update';
}

export interface ProgressEventPayload {
  event:
    | 'Log'
    | 'MessageDelta'
    | 'ThoughtDelta'
    | 'ToolCallStarted'
    | 'ToolCallComplete'
    | 'TaskStarted'
    | 'TaskCompleted'
    | 'Completed'
    | 'Error'
    | 'Plan';
  data:
    | string
    | { id: string; delta: string }
    | { tool_call_id: string; title: string; kind: string }
    | {
        tool_call_id: string;
        status: string;
        title: string;
        raw_input?: unknown;
        raw_output?: unknown;
      }
    | { task_id: string; title: string }
    | { task_id: string }
    | { task_count: number }
    | { message: string }
    | Plan;
}

export const useTauri = () => {
  const getVersion = useCallback(async (): Promise<string> => {
    return invoke('get_app_version');
  }, []);

  const getPendingReviews = useCallback(async (): Promise<
    Array<{
      id: string;
      diff: string;
      repo_root: string | null;
      agent: string | null;
      source: string;
      created_at: string;
    }>
  > => {
    return invoke('get_pending_reviews');
  }, []);

  const getPendingReviewFromState = useCallback(async (): Promise<{
    id: string;
    diff: string;
    repo_root: string | null;
    agent: string | null;
    source: string;
    created_at: string;
  } | null> => {
    return invoke('get_pending_review_from_state');
  }, []);

  const getAllReviews = useCallback(async (): Promise<
    Array<{
      id: string;
      title: string;
      summary: string | null;
      agent_id: string | null;
      task_count: number;
      created_at: string;
      source: ReviewSource;
      status: string;
      active_run_status?: string | null;
    }>
  > => {
    return invoke('get_all_reviews');
  }, []);

  const getReviewRuns = useCallback(
    async (
      reviewId: string
    ): Promise<
      Array<{
        id: string;
        review_id: string;
        agent_id: string;
        input_ref: string;
        diff_text: string;
        created_at: string;
        task_count: number;
        status: string;
      }>
    > => {
      return invoke('get_review_runs', { reviewId });
    },
    []
  );

  const getLinkedRepos = useCallback(async (): Promise<
    Array<{
      id: string;
      name: string;
      path: string;
      review_count: number;
      linked_at: string;
      remotes: string[];
      allow_snapshot_access: boolean;
    }>
  > => {
    return invoke('get_linked_repos');
  }, []);

  const parseDiff = useCallback(async (diffText: string): Promise<ParsedDiff> => {
    return invoke('parse_diff', { diffText });
  }, []);

  const getFileContent = useCallback(
    async (repoRoot: string, filePath: string, commit: string): Promise<string> => {
      return invoke('get_file_content', { repoRoot, filePath, commit });
    },
    []
  );

  const generateReview = useCallback(
    async (
      diffText: string,
      agentId: string,
      runId?: string,
      repoId?: string,
      source?: ReviewSource,
      useSnapshot?: boolean,
      onProgress?: Channel<ProgressEventPayload>
    ): Promise<{ task_count: number; review_id: string; run_id?: string }> => {
      return invoke('generate_review', {
        diffText,
        agentId,
        runId,
        repoId,
        source,
        useSnapshot: useSnapshot || false,
        onProgress,
      });
    },
    []
  );

  const loadTasks = useCallback(async (runId?: string): Promise<ReviewTask[]> => {
    return invoke('load_tasks', { runId });
  }, []);

  const updateTaskStatus = useCallback(async (taskId: string, status: string): Promise<void> => {
    return invoke('update_task_status', { taskId, status });
  }, []);

  const saveFeedback = useCallback(
    async (feedback: {
      review_id: string;
      task_id?: string;
      rule_id?: string;
      title: string;
      file_path?: string;
      line_number?: number;
      side?: string;
      content: string;
      impact: string;
    }): Promise<string> => {
      return invoke('save_feedback', { feedback });
    },
    []
  );

  const getFeedbackComments = useCallback(async (feedbackId: string): Promise<Comment[]> => {
    return invoke('get_feedback_comments', { feedbackId });
  }, []);

  const addComment = useCallback(async (feedbackId: string, body: string): Promise<string> => {
    return invoke('add_comment', { feedbackId, body });
  }, []);

  const updateFeedbackStatus = useCallback(
    async (feedbackId: string, status: string): Promise<void> => {
      return invoke('update_feedback_status', { feedbackId, status });
    },
    []
  );

  const updateFeedbackImpact = useCallback(
    async (feedbackId: string, impact: string): Promise<void> => {
      return invoke('update_feedback_impact', { feedbackId, impact });
    },
    []
  );

  const deleteFeedback = useCallback(async (feedbackId: string): Promise<void> => {
    return invoke('delete_feedback', { feedbackId });
  }, []);

  const getFeedbackByReview = useCallback(
    async (
      reviewId: string
    ): Promise<
      Array<{
        id: string;
        review_id: string;
        task_id: string | null;
        rule_id?: string | null;
        finding_id?: string | null;
        category?: string | null;
        title: string;
        status: string;
        impact: string;
        confidence: number;
        anchor: {
          file_path: string | null;
          line_number: number | null;
          side: string | null;
        } | null;
        author: string;
        created_at: string;
        updated_at: string;
      }>
    > => {
      return invoke('get_feedback_by_review', { reviewId });
    },
    []
  );

  const getFeedbackDiffSnippet = useCallback(
    async (
      feedbackId: string,
      contextLines: number = 3
    ): Promise<{
      file_path: string;
      hunk_header: string;
      lines: Array<{
        line_number: number;
        content: string;
        prefix: string;
        is_addition: boolean;
        is_deletion: boolean;
      }>;
      highlighted_line: number | null;
    } | null> => {
      return invoke('get_feedback_diff_snippet', { feedbackId, contextLines });
    },
    []
  );

  const exportReview = useCallback(async (reviewId: string, format: string): Promise<string> => {
    return invoke('export_review', { reviewId, format });
  }, []);

  const deleteReview = useCallback(async (reviewId: string): Promise<void> => {
    return invoke('delete_review', { reviewId });
  }, []);

  const fetchRemotePr = useCallback(
    async (prRef: string, providerHint?: string | null): Promise<ParsedDiff> => {
      return invoke('fetch_remote_pr', { prRef, providerHint });
    },
    []
  );

  const exportReviewMarkdown = useCallback(
    async (
      reviewId: string,
      selectedTasks: string[] = [],
      selectedFeedbacks: string[] = []
    ): Promise<string> => {
      return invoke('export_review_markdown', {
        reviewId,
        selectedTasks,
        selectedFeedbacks,
      });
    },
    []
  );

  const pushRemoteReview = useCallback(
    async (
      reviewId: string,
      selectedTasks: string[],
      selectedFeedbacks: string[]
    ): Promise<string> => {
      return invoke('push_remote_review', {
        reviewId,
        selectedTasks,
        selectedFeedbacks,
      });
    },
    []
  );

  const pushRemoteFeedback = useCallback(async (feedbackId: string): Promise<string> => {
    return invoke('push_remote_feedback', { feedbackId });
  }, []);

  const openUrl = useCallback(async (url: string): Promise<void> => {
    return invoke('open_url', { url });
  }, []);

  const copyToClipboard = useCallback(async (text: string): Promise<void> => {
    return invoke('copy_to_clipboard', { text });
  }, []);

  const getAgents = useCallback(async (): Promise<Agent[]> => {
    return invoke('get_agents');
  }, []);

  const getGitHubToken = useCallback(async (): Promise<string | null> => {
    return invoke('get_github_token');
  }, []);

  const setGitHubToken = useCallback(async (token: string): Promise<void> => {
    return invoke('set_github_token', { token });
  }, []);

  const getVcsStatus = useCallback(async (): Promise<VcsStatus[]> => {
    return invoke('get_vcs_status');
  }, []);

  const getSingleVcsStatus = useCallback(async (providerId: string): Promise<VcsStatus> => {
    return invoke('get_single_vcs_status', { providerId });
  }, []);

  const getReviewRules = useCallback(async (): Promise<ReviewRule[]> => {
    return invoke('get_review_rules');
  }, []);

  const createReviewRule = useCallback(async (input: ReviewRuleInput): Promise<ReviewRule> => {
    return invoke('create_review_rule', { input });
  }, []);

  const updateReviewRule = useCallback(
    async (id: string, input: ReviewRuleInput): Promise<ReviewRule> => {
      return invoke('update_review_rule', { id, input });
    },
    []
  );

  const deleteReviewRule = useCallback(async (id: string): Promise<void> => {
    return invoke('delete_review_rule', { id });
  }, []);

  const linkRepo = useCallback(async (path: string): Promise<LinkedRepo> => {
    return invoke('link_repo', { path });
  }, []);

  const cloneAndLinkRepo = useCallback(
    async (request: {
      provider: 'github' | 'gitlab';
      repo: string;
      host?: string;
      destDir: string;
    }): Promise<LinkedRepo> => {
      return invoke('clone_and_link_repo', {
        request: {
          provider: request.provider,
          repo: request.repo,
          host: request.host,
          dest_dir: request.destDir,
        },
      });
    },
    []
  );

  const unlinkRepo = useCallback(async (repoId: string): Promise<void> => {
    return invoke('unlink_repo', { repoId });
  }, []);

  const updateAgentConfig = useCallback(
    async (id: string, path: string, args?: string[]): Promise<void> => {
      return invoke('update_agent_config', { id, path, args });
    },
    []
  );

  const addCustomAgent = useCallback(
    async (
      id: string,
      label: string,
      command: string,
      args?: string[],
      logo?: string
    ): Promise<void> => {
      return invoke('add_custom_agent', { id, label, command, args, logo });
    },
    []
  );

  const deleteCustomAgent = useCallback(async (id: string): Promise<void> => {
    return invoke('delete_custom_agent', { id });
  }, []);

  const selectRepoFolder = useCallback(async (): Promise<string | null> => {
    const result = await open({
      directory: true,
      multiple: false,
      title: 'Select Repository Folder',
    });
    return result as string | null;
  }, []);

  const onProgress = useCallback(
    (
      callback: EventCallback<{
        event_type: string;
        message: string;
        progress?: number;
      }>
    ): Promise<() => void> => {
      return listen('progress', callback);
    },
    []
  );

  const onReviewComplete = useCallback(
    (
      callback: EventCallback<{
        review_id: string;
        run_id: string;
        task_count: number;
      }>
    ): Promise<() => void> => {
      return listen('review_complete', callback);
    },
    []
  );

  return {
    getVersion,
    getPendingReviews,
    getPendingReviewFromState,
    getAllReviews,
    getReviewRuns,
    getLinkedRepos,
    parseDiff,
    getFileContent,
    generateReview,
    loadTasks,
    updateTaskStatus,
    saveFeedback,
    getFeedbackComments,
    addComment,
    updateFeedbackStatus,
    updateFeedbackImpact,
    deleteFeedback,
    deleteReview,
    getFeedbackByReview,
    getFeedbackDiffSnippet,
    exportReview,
    fetchRemotePr,
    exportReviewMarkdown,
    pushRemoteReview,
    pushRemoteFeedback,
    openUrl,
    copyToClipboard,
    onProgress,
    onReviewComplete,
    getAgents,
    updateAgentConfig,
    addCustomAgent,
    deleteCustomAgent,
    getGitHubToken,
    setGitHubToken,
    getVcsStatus,
    getSingleVcsStatus,
    getReviewRules,
    createReviewRule,
    updateReviewRule,
    deleteReviewRule,
    linkRepo,
    cloneAndLinkRepo,
    unlinkRepo,

    selectRepoFolder,
    getAvailableEditors: useCallback(async (): Promise<EditorCandidate[]> => {
      return invoke('get_available_editors');
    }, []),
    getEditorConfig: useCallback(async (): Promise<EditorConfig> => {
      return invoke('get_editor_config');
    }, []),
    updateEditorConfig: useCallback(async (editorId: string): Promise<void> => {
      return invoke('update_editor_config', { editorId });
    }, []),
    getFeedbackFilterConfig: useCallback(async (): Promise<{ confidence_threshold: number | null }> => {
      return invoke('get_feedback_filter_config');
    }, []),
    updateFeedbackFilterConfig: useCallback(async (threshold: number | null): Promise<void> => {
      return invoke('update_feedback_filter_config', { threshold });
    }, []),
    openInEditor: useCallback(
      async (filePath: string, lineNumber: number, repoRoot?: string): Promise<void> => {
        return invoke('open_in_editor', { filePath, lineNumber, repoRoot });
      },
      []
    ),
    getRepoRootForReview: useCallback(async (reviewId: string): Promise<string | null> => {
      return invoke('get_repo_root_for_review', { reviewId });
    }, []),

    getCliStatus: useCallback(async (): Promise<CliStatus> => {
      return invoke('get_cli_status');
    }, []),
    installCli: useCallback(async (): Promise<void> => {
      return invoke('install_cli');
    }, []),
    getDiffRequest: useCallback(async (): Promise<{
      from: string;
      to: string;
      agent: string | null;
      source: string;
    } | null> => {
      return invoke('get_diff_request');
    }, []),
    acquireDiffFromRequest: useCallback(async (): Promise<{
      id: string;
      diff: string;
      repo_root: string | null;
      agent: string | null;
      source: string;
      created_at: string;
      review_source?: ReviewSource;
    }> => {
      return invoke('acquire_diff_from_request');
    }, []),
    stop_generation: useCallback(async (runId: string): Promise<void> => {
      return invoke('stop_generation', { runId });
    }, []),
    setRepoSnapshotAccess: useCallback(async (repoId: string, allowed: boolean): Promise<void> => {
      return invoke('set_repo_snapshot_access', { repoId, allowed });
    }, []),

    // Issue checks
    getIssueChecksForRun: useCallback(async (runId: string): Promise<IssueCheckWithFindings[]> => {
      return invoke('get_issue_checks_for_run', { runId });
    }, []),

    // Rule library
    getRuleLibrary: useCallback(async (): Promise<LibraryRule[]> => {
      return invoke('get_rule_library');
    }, []),
    getRuleLibraryByCategory: useCallback(async (category: LibraryCategory): Promise<LibraryRule[]> => {
      return invoke('get_rule_library_by_category', { category });
    }, []),
    getRuleLibraryChecklists: useCallback(async (): Promise<LibraryRule[]> => {
      return invoke('get_rule_library_checklists');
    }, []),
    getRuleLibraryGuidelines: useCallback(async (): Promise<LibraryRule[]> => {
      return invoke('get_rule_library_guidelines');
    }, []),
    addRuleFromLibrary: useCallback(
      async (libraryRuleId: string, scope: 'global' | 'repo', repoId?: string): Promise<ReviewRule> => {
        return invoke('add_rule_from_library', { libraryRuleId, scope, repoId });
      },
      []
    ),
    getDefaultIssueCategories: useCallback(async (): Promise<DefaultIssueCategory[]> => {
      return invoke('get_default_issue_categories');
    }, []),

    // Rule analytics
    getRuleRejectionStats: useCallback(async (): Promise<
      Array<{
        rule_id: string;
        total_feedback: number;
        rejected_count: number;
        rejection_rate: number;
      }>
    > => {
      return invoke('get_rule_rejection_stats');
    }, []),

    // Learning system
    getLearnedPatterns: useCallback(async (): Promise<LearnedPattern[]> => {
      return invoke('get_learned_patterns');
    }, []),
    createLearnedPattern: useCallback(async (input: LearnedPatternInput): Promise<LearnedPattern> => {
      return invoke('create_learned_pattern', { input });
    }, []),
    updateLearnedPattern: useCallback(
      async (id: string, input: LearnedPatternInput): Promise<LearnedPattern> => {
        return invoke('update_learned_pattern', { id, input });
      },
      []
    ),
    deleteLearnedPattern: useCallback(async (id: string): Promise<void> => {
      return invoke('delete_learned_pattern', { id });
    }, []),
    toggleLearnedPattern: useCallback(async (id: string, enabled: boolean): Promise<void> => {
      return invoke('toggle_learned_pattern', { id, enabled });
    }, []),
    getLearningStatus: useCallback(async (): Promise<LearningStatus> => {
      return invoke('get_learning_status');
    }, []),
    triggerLearningCompaction: useCallback(async (agentId: string): Promise<LearningCompactionResult> => {
      return invoke('trigger_learning_compaction', { agentId });
    }, []),

    // Merge confidence
    getMergeConfidence: useCallback(async (runId: string): Promise<MergeConfidence | null> => {
      return invoke('get_merge_confidence', { runId });
    }, []),
  };
};
