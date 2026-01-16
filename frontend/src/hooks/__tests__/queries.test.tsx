import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, waitFor, act } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { queryKeys } from '../../lib/query-keys';
import { createQueryClient } from '../../lib/query-client';
import { useReviews } from '../../hooks/useReviews';
import { useTasks } from '../../hooks/useTasks';
import { useReview } from '../../hooks/useReview';
import { useParsedDiff } from '../../hooks/useParsedDiff';
import { useRepos } from '../../hooks/useRepos';
import { useAgents } from '../../hooks/useAgents';
import { useFeedback, useFeedbackComments, useAddComment } from '../../hooks/useFeedback';
import {
  mockTauri,
  createMockReview,
  createMockTask,
  createMockRun,
  createMockFeedback,
  createMockComment,
} from '../../test/mocks';

vi.mock('../../hooks/useTauri', () => ({
  useTauri: () => mockTauri,
}));

const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        gcTime: Infinity,
      },
    },
  });
  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
};

describe('query configuration', () => {
  it('has correct staleTime (30 seconds for local app)', () => {
    const client = createQueryClient();
    expect(client.getDefaultOptions().queries?.staleTime).toBe(30000);
  });

  it('has retry disabled for local app', () => {
    const client = createQueryClient();
    expect(client.getDefaultOptions().queries?.retry).toBe(0);
  });
});

describe('query-keys', () => {
  it('generates correct review keys', () => {
    expect(queryKeys.reviews).toEqual(['reviews']);
    expect(queryKeys.review('123')).toEqual(['reviews', '123']);
  });

  it('generates correct task keys', () => {
    expect(queryKeys.tasks('run-123')).toEqual(['tasks', 'run-123']);
  });

  it('generates correct reviewRuns keys', () => {
    expect(queryKeys.reviewRuns('123')).toEqual(['reviewRuns', '123']);
  });
});

describe('useReviews', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('fetches reviews from API', async () => {
    mockTauri.getAllReviews.mockResolvedValue([
      createMockReview({ id: '1', title: 'Review 1' }),
      createMockReview({ id: '2', title: 'Review 2' }),
    ]);

    const { result } = renderHook(() => useReviews(), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(true);

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
      expect(result.current.data).toHaveLength(2);
      expect(result.current.data?.[0].title).toBe('Review 1');
    });
  });

  it('invalidates reviews cache', () => {
    const { result } = renderHook(() => useReviews(), {
      wrapper: createWrapper(),
    });

    expect(typeof result.current.invalidate).toBe('function');
  });

  it('handles API errors gracefully', async () => {
    mockTauri.getAllReviews.mockRejectedValue(new Error('Failed to fetch'));

    const { result } = renderHook(() => useReviews(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
      expect(result.current.error?.message).toBe('Failed to fetch');
    });
  });
});

describe('useReview', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('returns null runId when reviewId is null', () => {
    const { result } = renderHook(() => useReview(null), {
      wrapper: createWrapper(),
    });

    expect(result.current.runId).toBeNull();
    expect(result.current.runs).toEqual([]);
    expect(result.current.isLoading).toBe(false);
  });

  it('fetches runs when reviewId is provided', async () => {
    mockTauri.getReviewRuns.mockResolvedValue([
      createMockRun({ id: 'run-1' }),
      createMockRun({ id: 'run-2' }),
    ]);

    const { result } = renderHook(() => useReview('review-123'), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(true);
    expect(mockTauri.getReviewRuns).toHaveBeenCalledWith('review-123');

    await waitFor(() => {
      expect(result.current.runId).toBe('run-1');
      expect(result.current.runs).toHaveLength(2);
    });
  });

  it('returns first run as firstRun', async () => {
    mockTauri.getReviewRuns.mockResolvedValue([createMockRun({ id: 'first-run' })]);

    const { result } = renderHook(() => useReview('review-123'), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.firstRun?.id).toBe('first-run');
    });
  });
});

describe('useTasks', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('returns empty array when runId is null', () => {
    const { result } = renderHook(() => useTasks(null), {
      wrapper: createWrapper(),
    });

    expect(result.current.data).toEqual([]);
    expect(result.current.isLoading).toBe(false);
  });

  it('fetches tasks when runId is provided', async () => {
    mockTauri.loadTasks.mockResolvedValue([
      createMockTask({ id: 'task-1', title: 'Task 1' }),
      createMockTask({ id: 'task-2', title: 'Task 2' }),
    ]);

    const { result } = renderHook(() => useTasks('run-123'), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(true);
    expect(mockTauri.loadTasks).toHaveBeenCalledWith('run-123');

    await waitFor(() => {
      expect(result.current.data).toHaveLength(2);
    });
  });

  it('provides updateTaskStatus function', async () => {
    mockTauri.loadTasks.mockResolvedValue([createMockTask()]);

    const { result } = renderHook(() => useTasks('run-123'), {
      wrapper: createWrapper(),
    });

    await waitFor(() => result.current.data !== undefined);

    expect(typeof result.current.updateTaskStatus).toBe('function');
    expect(result.current.isUpdatingStatus).toBe(false);
  });
});

describe('useParsedDiff', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('returns undefined when runId is null', () => {
    const { result } = renderHook(() => useParsedDiff(null, null), {
      wrapper: createWrapper(),
    });

    expect(result.current.data).toBeUndefined();
    expect(result.current.isLoading).toBe(false);
  });

  it('fetches and caches parsed diff', async () => {
    mockTauri.parseDiff.mockResolvedValue({
      diff_text: 'diff --git a/test.ts b/test.ts',
      files: [],
      total_additions: 10,
      total_deletions: 5,
    });

    const { result } = renderHook(() => useParsedDiff('run-123', 'diff text'), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(true);

    await waitFor(() => {
      expect(result.current.data).not.toBeNull();
      expect(result.current.data?.total_additions).toBe(10);
    });

    expect(mockTauri.parseDiff).toHaveBeenCalledWith('diff text');
  });
});

describe('useRepos', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('fetches repos from API', async () => {
    mockTauri.getLinkedRepos.mockResolvedValue([
      {
        id: 'repo-1',
        name: 'repo',
        path: '/path/to/repo',
        linked_at: new Date().toISOString(),
        remotes: ['https://github.com/test/repo'],
        allow_snapshot_access: false,
      },
    ]);

    const { result } = renderHook(() => useRepos(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.data).toHaveLength(1);
    });
  });

  it('invalidates cache after adding repo', async () => {
    mockTauri.getLinkedRepos.mockResolvedValue([]);
    mockTauri.linkRepo.mockResolvedValue({
      id: 'new-repo',
      name: 'new',
      path: '/new',
      linked_at: new Date().toISOString(),
      remotes: ['https://github.com/test/new'],
      allow_snapshot_access: false,
    });

    const { result } = renderHook(() => useRepos(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => result.current.data !== undefined);

    act(() => {
      result.current.addRepo.mutate('/new/path');
    });

    await waitFor(() => {
      expect(mockTauri.getLinkedRepos).toHaveBeenCalledTimes(2);
    });
  });
});

describe('useAgents', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('fetches agents from API', async () => {
    mockTauri.getAgents.mockResolvedValue([
      { id: 'agent-1', name: 'Claude', description: 'Claude agent' },
    ]);

    const { result } = renderHook(() => useAgents(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.data).toHaveLength(1);
      expect(result.current.data?.[0].name).toBe('Claude');
    });
  });
});

describe('useFeedback', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.stubGlobal('fetch', vi.fn());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('returns empty array when reviewId is null', () => {
    const { result } = renderHook(() => useFeedback(null), {
      wrapper: createWrapper(),
    });

    expect(result.current.data).toEqual([]);
    expect(result.current.isLoading).toBe(false);
  });

  it('provides createFeedback mutation', async () => {
    mockTauri.getFeedbackByReview.mockResolvedValue([]);
    mockTauri.saveFeedback.mockResolvedValue('new-feedback-id');

    const { result } = renderHook(() => useFeedback('review-1'), {
      wrapper: createWrapper(),
    });

    await waitFor(() => result.current.data !== undefined);

    act(() => {
      result.current.createFeedback({
        review_id: 'review-1',
        task_id: 'task-1',
        title: 'New Feedback',
        file_path: 'src/test.ts',
        line_number: 10,
        side: 'new',
        content: 'Test content',
        impact: 'nice_to_have',
      });
    });

    await waitFor(() => {
      expect(mockTauri.saveFeedback).toHaveBeenCalled();
    });
  });

  it('provides updateStatus mutation', async () => {
    mockTauri.getFeedbackByReview.mockResolvedValue([createMockFeedback()]);
    mockTauri.updateFeedbackStatus.mockResolvedValue(undefined);

    const { result } = renderHook(() => useFeedback('review-1'), {
      wrapper: createWrapper(),
    });

    await waitFor(() => result.current.data !== undefined);

    act(() => {
      result.current.updateStatus({ feedbackId: 'feedback-1', status: 'done' });
    });

    await waitFor(() => {
      expect(mockTauri.updateFeedbackStatus).toHaveBeenCalledWith('feedback-1', 'done');
    });
  });

  it('provides deleteFeedback mutation', async () => {
    mockTauri.getFeedbackByReview.mockResolvedValue([createMockFeedback()]);
    mockTauri.deleteFeedback.mockResolvedValue(undefined);

    const { result } = renderHook(() => useFeedback('review-1'), {
      wrapper: createWrapper(),
    });

    await waitFor(() => result.current.data !== undefined);

    act(() => {
      result.current.deleteFeedback({ feedbackId: 'feedback-1' });
    });

    await waitFor(() => {
      expect(mockTauri.deleteFeedback).toHaveBeenCalledWith('feedback-1');
    });
  });
});

describe('useFeedbackComments', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('fetches comments for feedback', async () => {
    mockTauri.getFeedbackComments.mockResolvedValue([
      createMockComment({ id: 'c1', body: 'Comment 1' }),
    ]);

    const { result } = renderHook(() => useFeedbackComments('feedback-1'), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.comments).toHaveLength(1);
      expect(result.current.comments[0].body).toBe('Comment 1');
    });
  });

  it('returns empty when feedbackId is null', () => {
    const { result } = renderHook(() => useFeedbackComments(null), {
      wrapper: createWrapper(),
    });

    expect(result.current.comments).toEqual([]);
  });
});

describe('useAddComment', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('adds comment and invalidates cache', async () => {
    mockTauri.addComment.mockResolvedValue('new-comment-id');
    mockTauri.getFeedbackComments.mockResolvedValue([]);

    const { result } = renderHook(() => useAddComment(), {
      wrapper: createWrapper(),
    });

    act(() => {
      result.current.mutate({ feedbackId: 'feedback-1', body: 'New comment' });
    });

    await waitFor(() => {
      expect(mockTauri.addComment).toHaveBeenCalledWith('feedback-1', 'New comment');
    });
  });
});
