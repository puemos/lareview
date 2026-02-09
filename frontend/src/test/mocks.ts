import { vi, type Mock } from 'vitest';
import type { Channel } from '@tauri-apps/api/core';
import type {
  ReviewTask,
  Review,
  LinkedRepo,
  Agent,
  ParsedDiff,
  ReviewRun,
  Feedback,
  Comment,
  ReviewSource,
} from '../types';
import type { ProgressEventPayload } from '../hooks/useTauri';

const createMockReview = (overrides = {}): Review => ({
  id: 'review-1',
  title: 'Test Review',
  summary: 'Mock summary for testing',
  source: { type: 'diff_paste', diff_hash: 'mock-hash' },
  active_run_id: 'run-1',
  created_at: new Date().toISOString(),
  updated_at: new Date().toISOString(),
  task_count: 5,
  status: 'todo',
  active_run_status: 'completed',
  ...overrides,
});

const createMockTask = (overrides = {}): ReviewTask => ({
  id: 'task-1',
  run_id: 'run-1',
  title: 'Test Task',
  description: 'Test description',
  files: ['src/test.ts'],
  stats: { additions: 10, deletions: 5, risk: 'low', tags: [] },
  diff_refs: [],
  ai_generated: true,
  status: 'pending',
  risk_level: 'low',
  ...overrides,
});

const createMockRun = (overrides = {}): ReviewRun => ({
  id: 'run-1',
  review_id: 'review-1',
  agent_id: 'agent-1',
  input_ref: 'HEAD',
  diff_text: 'diff --git a/test.ts b/test.ts',
  created_at: new Date().toISOString(),
  task_count: 5,
  status: 'completed',
  ...overrides,
});

const createMockParsedDiff = (overrides = {}): ParsedDiff => ({
  diff_text: 'diff --git a/test.ts b/test.ts',
  files: [],
  total_additions: 10,
  total_deletions: 5,
  ...overrides,
});

const createMockFeedback = (overrides = {}): Feedback => ({
  id: 'feedback-1',
  review_id: 'review-1',
  task_id: 'task-1',
  title: 'Test Feedback',
  status: 'todo',
  impact: 'nice_to_have',
  confidence: 1.0,
  anchor: { file_path: 'src/test.ts', line_number: 10, side: 'new' },
  author: 'test-user',
  created_at: new Date().toISOString(),
  updated_at: new Date().toISOString(),
  ...overrides,
});

const createMockComment = (overrides = {}): Comment => ({
  id: 'comment-1',
  feedback_id: 'feedback-1',
  author: 'test-user',
  body: 'Test comment',
  parent_id: null,
  created_at: new Date().toISOString(),
  updated_at: new Date().toISOString(),
  ...overrides,
});

interface MockTauriReturn {
  getAllReviews: Mock<() => Promise<Review[]>>;
  getPendingReviews: Mock<() => Promise<Review[]>>;
  getReviewRuns: Mock<(reviewId: string) => Promise<ReviewRun[]>>;
  getLinkedRepos: Mock<() => Promise<LinkedRepo[]>>;
  parseDiff: Mock<(diffText: string) => Promise<ParsedDiff>>;
  loadTasks: Mock<(runId: string) => Promise<ReviewTask[]>>;
  updateTaskStatus: Mock<(taskId: string, status: string) => Promise<void>>;
  getAgents: Mock<() => Promise<Agent[]>>;
  addCustomAgent: Mock<
    (id: string, label: string, command: string, args?: string[], logo?: string) => Promise<void>
  >;
  deleteCustomAgent: Mock<(id: string) => Promise<void>>;
  linkRepo: Mock<(path: string) => Promise<LinkedRepo>>;
  cloneAndLinkRepo: Mock<
    (input: {
      provider: 'github' | 'gitlab';
      repo: string;
      host?: string;
      destDir: string;
    }) => Promise<LinkedRepo>
  >;
  selectRepoFolder: Mock<() => Promise<string | null>>;
  saveFeedback: Mock<(feedback: Feedback) => Promise<string>>;
  updateFeedbackStatus: Mock<(feedbackId: string, status: string) => Promise<void>>;
  updateFeedbackImpact: Mock<(feedbackId: string, impact: string) => Promise<void>>;
  deleteFeedback: Mock<(feedbackId: string) => Promise<void>>;
  getFeedbackComments: Mock<(feedbackId: string) => Promise<Comment[]>>;
  addComment: Mock<(feedbackId: string, body: string) => Promise<string>>;
  getFeedbackByReview: Mock<(reviewId: string) => Promise<Feedback[]>>;
  generateReview: Mock<
    (
      diffText: string,
      agentId: string,
      runId?: string,
      repoId?: string,
      source?: ReviewSource,
      useSnapshot?: boolean,
      onProgress?: Channel<ProgressEventPayload>
    ) => Promise<{ task_count: number; review_id: string; run_id?: string }>
  >;
  stop_generation: Mock<(runId: string) => Promise<void>>;
}

function createMockTauri(): MockTauriReturn {
  const mock: MockTauriReturn = {
    getAllReviews: vi.fn().mockResolvedValue([createMockReview()]),
    getPendingReviews: vi.fn().mockResolvedValue([]),
    getReviewRuns: vi.fn().mockResolvedValue([createMockRun()]),
    getLinkedRepos: vi.fn().mockResolvedValue([]),
    parseDiff: vi.fn().mockResolvedValue(createMockParsedDiff()),
    loadTasks: vi.fn().mockResolvedValue([createMockTask()]),
    updateTaskStatus: vi.fn().mockResolvedValue(undefined),
    getAgents: vi
      .fn()
      .mockResolvedValue([{ id: 'agent-1', name: 'Test Agent', description: 'Test' }]),
    addCustomAgent: vi.fn().mockResolvedValue(undefined),
    deleteCustomAgent: vi.fn().mockResolvedValue(undefined),
    linkRepo: vi.fn().mockImplementation((path: string) =>
      Promise.resolve({
        id: 'repo-1',
        path,
        name: path.split('/').pop() || 'repo',
        linked_at: new Date().toISOString(),
      })
    ),
    cloneAndLinkRepo: vi.fn().mockImplementation((input: { destDir: string; repo: string }) => {
      const repoName = input.repo.split('/').pop() || input.repo;
      return Promise.resolve({
        id: 'repo-2',
        path: `${input.destDir}/${repoName}`,
        name: repoName,
        linked_at: new Date().toISOString(),
      });
    }),
    selectRepoFolder: vi.fn().mockResolvedValue('/tmp'),
    saveFeedback: vi.fn().mockResolvedValue('feedback-1'),
    updateFeedbackStatus: vi.fn().mockResolvedValue(undefined),
    updateFeedbackImpact: vi.fn().mockResolvedValue(undefined),
    deleteFeedback: vi.fn().mockResolvedValue(undefined),
    getFeedbackComments: vi.fn().mockResolvedValue([]),
    addComment: vi.fn().mockResolvedValue('comment-1'),
    getFeedbackByReview: vi.fn().mockResolvedValue([createMockFeedback()]),
    generateReview: vi.fn().mockResolvedValue({
      task_count: 5,
      review_id: 'review-1',
      run_id: 'run-1',
    }),
    stop_generation: vi.fn().mockResolvedValue(undefined),
  };
  return mock;
}

export const mockTauri = createMockTauri();

export {
  createMockReview,
  createMockTask,
  createMockRun,
  createMockParsedDiff,
  createMockFeedback,
  createMockComment,
};
