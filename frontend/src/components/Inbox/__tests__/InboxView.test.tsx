import { beforeEach, describe, expect, it, vi } from 'vitest';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { useAppStore } from '../../../store';
import { InboxView } from '../InboxView';
import type { ParsedDiff } from '../../../types';

const mocks = vi.hoisted(() => ({ fetch: vi.fn(), refetch: vi.fn(), openUrl: vi.fn() }));
let queryError: Error | null = null;

const candidates = [
  {
    providerId: 'github',
    source: { type: 'github_pr' as const, owner: 'puemos', repo: 'lareview', number: 28 },
    title: 'Harden external URL validation',
    url: 'https://github.com/puemos/lareview/pull/28',
    author: 'alice',
    repo: 'puemos/lareview',
    number: 28,
    updatedAt: '2026-08-06T12:00:00.000Z',
    reason: 'review_requested' as const,
    isDraft: false,
    status: 'new' as const,
  },
  {
    providerId: 'github',
    source: { type: 'github_pr' as const, owner: 'puemos', repo: 'web', number: 41 },
    title: 'Refresh the navigation shell',
    url: 'https://github.com/puemos/web/pull/41',
    author: 'bob',
    repo: 'puemos/web',
    number: 41,
    updatedAt: '2026-08-06T11:00:00.000Z',
    reason: 'assigned' as const,
    isDraft: true,
    status: { stale: { review_id: 'review-old', reviewed_head_sha: 'abc123' } },
  },
  {
    providerId: 'github',
    source: { type: 'github_pr' as const, owner: 'puemos', repo: 'desktop', number: 9 },
    title: 'Polish the settings view',
    url: 'https://github.com/puemos/desktop/pull/9',
    author: 'carol',
    repo: 'puemos/desktop',
    number: 9,
    updatedAt: '2026-08-06T10:00:00.000Z',
    reason: 'review_requested' as const,
    isDraft: false,
    status: { reviewed: { review_id: 'review-done' } },
  },
];

vi.mock('../../../hooks/useReviewCandidates', () => ({
  useReviewCandidates: () => ({
    candidates: queryError ? [] : candidates,
    error: queryError,
    errors: [],
    unsupportedProviders: [],
    isLoading: false,
    isFetching: false,
    refetch: mocks.refetch,
  }),
}));

vi.mock('../../../hooks/useTauri', () => ({
  useTauri: () => ({
    fetchRemotePr: mocks.fetch,
    openUrl: mocks.openUrl,
  }),
}));

vi.mock('../../../hooks/useRepos', () => ({
  useRepos: () => ({
    data: [{ id: 'matching-repo', remotes: ['git@github.com:puemos/lareview.git'] }],
  }),
}));

describe('InboxView', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    queryError = null;
    useAppStore.getState().reset();
  });

  it('focuses on awaiting requests by default and can reveal reviewed work', () => {
    render(<InboxView onNavigate={vi.fn()} />);

    expect(screen.getByText('Harden external URL validation')).toBeInTheDocument();
    expect(screen.getByText('Refresh the navigation shell')).toBeInTheDocument();
    expect(screen.queryByText('Polish the settings view')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'All (3)' }));

    expect(screen.getByText('Polish the settings view')).toBeInTheDocument();
  });

  it('searches across candidate metadata and clears the query', () => {
    render(<InboxView onNavigate={vi.fn()} />);

    const search = screen.getByRole('searchbox', { name: 'Search inbox' });
    fireEvent.change(search, { target: { value: 'web' } });

    expect(screen.getByText('Refresh the navigation shell')).toBeInTheDocument();
    expect(screen.queryByText('Harden external URL validation')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Clear inbox search' }));
    expect(screen.getByText('Harden external URL validation')).toBeInTheDocument();
  });

  it('filters the inbox to a single repository', () => {
    render(<InboxView onNavigate={vi.fn()} />);

    const repositoryFilter = screen.getByRole('combobox', { name: 'Filter by repository' });
    fireEvent.keyDown(repositoryFilter, { key: 'ArrowDown' });
    fireEvent.click(screen.getByRole('option', { name: 'puemos/web' }));

    expect(screen.getByText('Refresh the navigation shell')).toBeInTheDocument();
    expect(screen.queryByText('Harden external URL validation')).not.toBeInTheDocument();
  });

  it('preserves the fetched source, replaces an unrelated repository, and prevents competing loads', async () => {
    let finish!: (value: ParsedDiff) => void;
    mocks.fetch.mockReturnValue(
      new Promise<ParsedDiff>(resolve => {
        finish = resolve;
      })
    );
    useAppStore.setState({ selectedRepoId: 'unrelated' });
    const navigate = vi.fn();
    render(<InboxView onNavigate={navigate} />);
    fireEvent.click(screen.getByRole('button', { name: 'Review puemos/lareview#28' }));
    expect(screen.getByRole('button', { name: 'Review puemos/web#41' })).toBeDisabled();
    const diff: ParsedDiff = {
      diff_text: 'new diff',
      source: candidates[0].source,
      total_additions: 1,
      total_deletions: 0,
    };
    await act(async () => finish(diff));
    await waitFor(() => expect(navigate).toHaveBeenCalledWith('generate'));
    expect(useAppStore.getState()).toMatchObject({
      pendingSource: diff.source,
      parsedDiff: diff,
      diffText: 'new diff',
      selectedRepoId: 'matching-repo',
      viewMode: 'diff',
    });
    expect(mocks.fetch).toHaveBeenCalledOnce();
  });

  it('shows a query failure with recovery instead of an empty inbox', () => {
    queryError = new Error('Connection unavailable');
    render(<InboxView onNavigate={vi.fn()} />);
    expect(screen.getByRole('alert')).toHaveTextContent('Connection unavailable');
    expect(screen.queryByText('No review requests')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Try again' }));
    expect(mocks.refetch).toHaveBeenCalledOnce();
  });
});
