import type { ReviewTask, Review, Agent } from '../types';

export const queryKeys = {
  reviews: ['reviews'] as const,
  review: (id: string) => ['reviews', id] as const,
  reviewRuns: (reviewId: string) => ['reviewRuns', reviewId] as const,
  tasks: (runId: string) => ['tasks', runId] as const,
  feedback: ['feedback'] as const,
  feedbackByReview: (reviewId: string) => ['feedback', 'byReview', reviewId] as const,
  feedbackFilterConfig: ['feedbackFilterConfig'] as const,
  timeoutConfig: ['timeoutConfig'] as const,
  repos: ['repos'] as const,
  agents: ['agents'] as const,
  rules: ['rules'] as const,
  issueChecks: (runId: string) => ['issueChecks', runId] as const,
  ruleLibrary: ['ruleLibrary'] as const,
  defaultIssueCategories: ['defaultIssueCategories'] as const,
  ruleRejectionStats: ['ruleRejectionStats'] as const,
  // Learning system
  learnedPatterns: ['learnedPatterns'] as const,
  learningStatus: ['learningStatus'] as const,
  // Merge confidence
  mergeConfidence: (runId: string) => ['mergeConfidence', runId] as const,
};

export type QueryKeyType = typeof queryKeys;

export type ReviewQuery = Review[];
export type ReviewRunsQuery = Array<{
  id: string;
  review_id: string;
  agent_id: string;
  input_ref: string;
  diff_text: string;
  created_at: string;
  task_count: number;
  status: string;
}>;
export type TasksQuery = ReviewTask[];
export type ReposQuery = Array<{
  id: string;
  name: string;
  path: string;
  review_count: number;
  linked_at: string;
}>;
export type AgentsQuery = Agent[];
