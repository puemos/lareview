import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { useAppStore } from '../../../store';
import { InboxView } from '../InboxView';

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
    candidates,
    errors: [],
    unsupportedProviders: [],
    isLoading: false,
    isFetching: false,
    refetch: vi.fn(),
  }),
}));

vi.mock('../../../hooks/useTauri', () => ({
  useTauri: () => ({
    fetchRemotePr: vi.fn(),
    openUrl: vi.fn(),
  }),
}));

describe('InboxView', () => {
  beforeEach(() => {
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
});
