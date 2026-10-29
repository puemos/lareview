import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { ReposView } from './ReposView';
import { useAppStore } from '../../store';

const repo = {
  id: 'repo-1',
  name: 'Internal project',
  path: '/code/team/internal',
  review_count: 1,
  linked_at: '2026-10-29T12:00:00Z',
  remotes: [],
};
const mocks = vi.hoisted(() => ({
  refetch: vi.fn(),
  link: vi.fn(),
  unlink: vi.fn(),
  selectFolder: vi.fn(),
}));
let state: {
  data: (typeof repo)[];
  isLoading: boolean;
  isError: boolean;
  error: unknown;
  isFetching: boolean;
};
vi.mock('../../hooks/useRepos', () => ({
  useRepos: () => ({
    ...state,
    refetch: mocks.refetch,
    addRepo: { mutate: mocks.link, isPending: false },
    removeRepo: { mutate: mocks.unlink },
    selectRepoFolder: mocks.selectFolder,
  }),
}));

describe('Repositories', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAppStore.getState().reset();
    state = { data: [repo], isLoading: false, isError: false, error: null, isFetching: false };
  });
  it('searches names and paths with trimmed text and clears without losing input focus', () => {
    render(<ReposView onNavigate={vi.fn()} />);
    const input = screen.getByRole('searchbox', { name: 'Search repositories' });
    fireEvent.change(input, { target: { value: ' TEAM/ ' } });
    expect(screen.getByText('Internal project')).toBeInTheDocument();
    fireEvent.change(input, { target: { value: 'missing' } });
    expect(screen.getByText('No matching repositories')).toBeInTheDocument();
    fireEvent.keyDown(input, { key: 'Escape' });
    expect(input).toHaveValue('');
    expect(input).toHaveFocus();
    expect(screen.getByText('Internal project')).toBeInTheDocument();
  });
  it('starts a fresh comparison for the selected repository', () => {
    useAppStore.setState({
      diffText: 'old diff',
      prRef: 'owner/repo#1',
      pendingSource: { type: 'diff_paste', diff_hash: 'old' },
    });
    const navigate = vi.fn();
    render(<ReposView onNavigate={navigate} />);
    fireEvent.click(screen.getByRole('button', { name: 'Review changes in Internal project' }));
    expect(navigate).toHaveBeenCalledWith('generate');
    expect(useAppStore.getState()).toMatchObject({
      selectedRepoId: 'repo-1',
      diffText: '',
      pendingSource: null,
      parsedDiff: null,
      prRef: '',
      viewMode: 'raw',
    });
  });
  it('exposes loading and recoverable query failure without claiming an empty collection', () => {
    state = { ...state, data: [], isLoading: true };
    const { rerender } = render(<ReposView onNavigate={vi.fn()} />);
    expect(screen.getByRole('status', { name: 'Loading repositories' })).toBeInTheDocument();
    expect(screen.queryByText('Link your first repository')).not.toBeInTheDocument();
    state = { ...state, isLoading: false, isError: true, error: 'Database unavailable' };
    rerender(<ReposView onNavigate={vi.fn()} />);
    expect(screen.getByRole('alert')).toHaveTextContent('Database unavailable');
    fireEvent.click(screen.getByRole('button', { name: 'Try again' }));
    expect(mocks.refetch).toHaveBeenCalledOnce();
  });
});
