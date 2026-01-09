import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { Sidebar } from '../Sidebar';
import { useAppStore } from '../../../store';

const { mockGetReviewRuns, mockDeleteReview, mockUseTauri } = vi.hoisted(() => {
  const mockGetReviewRuns = vi.fn().mockResolvedValue([]);
  const mockDeleteReview = vi.fn().mockResolvedValue(undefined);
  const mockUseTauri = vi.fn(() => ({
    getReviewRuns: mockGetReviewRuns,
    deleteReview: mockDeleteReview,
  }));
  return { mockGetReviewRuns, mockDeleteReview, mockUseTauri };
});

vi.mock('../../../hooks/useTauri', async () => {
  const actual = await vi.importActual<typeof import('../../../hooks/useTauri')>(
    '../../../hooks/useTauri'
  );
  return {
    ...actual,
    useTauri: mockUseTauri,
  };
});

vi.mock('../../../hooks/useReviews', () => ({
  useReviews: () => ({
    data: [
      {
        id: 'review-1',
        title: 'Review 1',
        summary: null,
        agent_id: null,
        task_count: 0,
        created_at: new Date().toISOString(),
        source: { type: 'diff_paste', diff_hash: 'mock-hash' },
        status: 'todo',
        active_run_status: 'running',
      },
    ],
    isLoading: false,
    invalidate: vi.fn(),
  }),
}));

const renderSidebar = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <Sidebar currentView="review" onViewChange={vi.fn()} />
    </QueryClientProvider>
  );
};

describe('Sidebar', () => {
  beforeEach(() => {
    useAppStore.getState().reset();
    mockGetReviewRuns.mockClear();
    mockDeleteReview.mockClear();
    mockUseTauri.mockClear();
  });

  it('prefetches review runs on hover without extra useTauri calls', async () => {
    renderSidebar();

    const reviewItem = await screen.findByLabelText('Review: Review 1');
    fireEvent.mouseEnter(reviewItem);

    await waitFor(() => {
      expect(mockGetReviewRuns).toHaveBeenCalledWith('review-1');
    });

    expect(mockUseTauri).toHaveBeenCalledTimes(1);
  });
});
