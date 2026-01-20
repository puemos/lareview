export interface DiffFile {
  name: string;
  old_path?: string | null;
  new_path: string;
  hunks: DiffHunk[];
  status?: 'added' | 'modified' | 'deleted' | 'renamed';
}

export interface DiffHunk {
  old_start: number;
  old_lines: number;
  new_start: number;
  new_lines: number;
  content?: string;
  header?: string;
}

export interface ParsedDiff {
  diff_text: string;
  files?: DiffFile[];
  total_additions: number;
  total_deletions: number;
  changed_files?: number;
  hunk_manifest?: string;
  source?: ReviewSource;
  title?: string | null;
}

export interface CommentThread {
  id: string;
  lineNumber: number;
  side: 'original' | 'modified';
  comments: DiffComment[];
}

export interface DiffComment {
  id: string;
  author: string;
  content: string;
  createdAt: string;
}

export interface ReviewTask {
  id: string;
  run_id: string;
  title: string;
  description: string;
  files: string[];
  stats: TaskStats;
  diff_refs: DiffRef[];
  insight?: string;
  diagram?: string;
  ai_generated: boolean;
  status: 'pending' | 'in_progress' | 'done' | 'ignored';
  sub_flow?: string;
  risk_level: 'low' | 'medium' | 'high';
  file_path?: string | null;
  line_number?: number | null;
}

export interface TaskStats {
  additions: number;
  deletions: number;
  risk: 'low' | 'medium' | 'high';
  tags: string[];
}

export interface DiffRef {
  file: string;
  hunks: HunkRef[];
}

export interface HunkRef {
  old_start: number;
  old_lines: number;
  new_start: number;
  new_lines: number;
}

export interface ReviewRun {
  id: string;
  review_id: string;
  agent_id: string;
  input_ref: string;
  diff_text: string;
  created_at: string;
  task_count: number;
  status: string;
}

export interface Feedback {
  id: string;
  review_id: string;
  task_id: string | null;
  rule_id?: string | null;
  finding_id?: string | null;
  title: string;
  status: 'todo' | 'in_progress' | 'done' | 'ignored';
  impact: 'blocking' | 'nice_to_have' | 'nitpick';
  anchor: FeedbackAnchor | null;
  author: string;
  created_at: string;
  updated_at: string;
  comments?: Comment[];
}

export interface FeedbackAnchor {
  file_path: string | null;
  line_number: number | null;
  side: 'old' | 'new' | null;
}

export interface Comment {
  id: string;
  feedback_id: string;
  author: string;
  body: string;
  parent_id: string | null;
  created_at: string;
  updated_at: string;
}

export interface LinkedRepo {
  id: string;
  path: string;
  name: string;
  linked_at?: string;
  remotes: string[];
  review_count?: number;
  allow_snapshot_access: boolean;
}

export interface WorktreeSession {
  id: string;
  repo_id: string;
  worktree_path: string;
  commit_sha: string;
  created_at: string;
}

export type RuleScope = 'global' | 'repo';

export interface ReviewRule {
  id: string;
  scope: RuleScope;
  repo_id?: string | null;
  glob?: string | null;
  category?: string | null;
  text: string;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

// Issue Checklist Types
export type CheckStatus = 'found' | 'not_found' | 'not_applicable' | 'skipped';
export type Confidence = 'high' | 'medium' | 'low';

export interface IssueCheck {
  id: string;
  run_id: string;
  rule_id?: string | null;
  category: string;
  display_name: string;
  status: CheckStatus;
  confidence: Confidence;
  summary?: string | null;
  created_at: string;
}

export interface IssueFinding {
  id: string;
  check_id: string;
  title: string;
  description: string;
  evidence: string;
  file_path?: string | null;
  line_number?: number | null;
  impact: 'blocking' | 'nice_to_have' | 'nitpick';
  created_at: string;
}

export interface IssueCheckWithFindings extends IssueCheck {
  findings: IssueFinding[];
}

// Rule Library Types
export type LibraryCategory =
  | 'security'
  | 'code_quality'
  | 'testing'
  | 'documentation'
  | 'performance'
  | 'api_design'
  | 'language_specific'
  | 'framework_specific';

export interface LibraryRule {
  id: string;
  name: string;
  library_category: LibraryCategory;
  category?: string | null;
  description: string;
  text: string;
  glob?: string | null;
  tags: string[];
}

export interface DefaultIssueCategory {
  id: string;
  name: string;
  description: string;
  examples: string[];
  enabled_by_default: boolean;
}

export interface Agent {
  id: string;
  name: string;
  description?: string;
  path?: string;
  args?: string[];
  logo?: string;
  available?: boolean;
}

export interface VcsStatus {
  id: string;
  name: string;
  cliPath: string;
  login?: string;
  error?: string;
}

export interface CliStatus {
  isInstalled: boolean;
  version?: string;
  path?: string;
}

export interface EditorCandidate {
  id: string;
  label: string;
  path: string;
}

export interface EditorConfig {
  preferred_editor_id: string | null;
}

export interface AppState {
  diffText: string;
  parsedDiff: ParsedDiff | null;
  selectedFile: DiffFile | null;
  commentThreads: Map<number, CommentThread[]>;
  tasks: ReviewTask[];
  selectedTaskId: string | null;
  isGenerating: boolean;
  currentView: string;
  reviewId?: string;
  runId?: string;
}

export interface Review {
  id: string;
  title: string;
  summary: string | null;
  source: ReviewSource;
  active_run_id: string | null;
  created_at: string;
  updated_at: string;
  task_count: number;
  agent_id?: string;
  status: string;
  active_run_status?: string | null;
}

export type ReviewSource =
  | { type: 'diff_paste'; diff_hash: string }
  | {
      type: 'github_pr';
      owner: string;
      repo: string;
      number: number;
      url?: string;
      head_sha?: string;
      base_sha?: string;
    }
  | {
      type: 'gitlab_mr';
      host: string;
      project_path: string;
      number: number;
      url?: string;
      head_sha?: string;
      base_sha?: string;
      start_sha?: string;
    };

export type ViewType = 'generate' | 'review' | 'repos' | 'rules' | 'settings';

export interface Plan {
  entries: PlanEntry[];
  meta?: Record<string, unknown>;
}

export interface PlanEntry {
  content: string;
  priority?: string | number;
  status: string;
  meta?: Record<string, unknown>;
}
