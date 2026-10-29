import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { GenerateView } from '../GenerateView';
import { useAppStore } from '../../../store';

const generationMock = vi.hoisted(() => ({
  startGeneration: vi.fn(),
  stopGeneration: vi.fn(),
}));
const localGitMock = vi.hoisted(() => ({
  fetch: vi.fn(),
  repos: [] as Array<{ id: string; name: string; path: string; remotes: string[] }>,
}));
const remoteMock = vi.hoisted(() => ({ fetch: vi.fn() }));

vi.mock('../../../store');
vi.mock('../../../hooks/useTauri', () => ({
  useTauri: () => ({ fetchRemotePr: remoteMock.fetch, fetchLocalGitDiff: localGitMock.fetch }),
}));
vi.mock('../../../hooks/useAgents', () => ({
  useAgents: () => ({ data: [{ id: 'test-agent', name: 'Test Agent', available: true }] }),
}));
vi.mock('../../../hooks/useRepos', () => ({
  useRepos: () => ({
    data: localGitMock.repos,
    addRepo: { isPending: false, mutateAsync: vi.fn() },
    cloneRepo: { isPending: false, mutateAsync: vi.fn() },
    selectRepoFolder: vi.fn(),
  }),
}));
vi.mock('../../../contexts/useGeneration', () => ({
  useGeneration: () => generationMock,
}));
vi.mock('../DiffEditorPanel', () => ({
  DiffEditorPanel: () => <div data-testid="diff-editor-panel" />,
  countAdditions: () => 0,
  countDeletions: () => 0,
}));
vi.mock('../AgentConfigPanel', () => ({
  AgentConfigPanel: ({ onGenerate, isStarting, isDiffValid }: any) => (
    <button data-testid="start-review" onClick={onGenerate} disabled={isStarting || !isDiffValid}>
      Start Review
    </button>
  ),
}));
vi.mock('../VcsInputCard', () => ({
  VcsInputCard: ({ prominent, onFetch }: { prominent?: boolean; onFetch: () => void }) => (
    <div data-testid="vcs-input-card" data-prominent={prominent ? 'true' : 'false'}>
      <button onClick={onFetch}>Fetch</button>
    </div>
  ),
}));
vi.mock('../ViewModeToggle', () => ({
  ViewModeToggle: () => <div data-testid="view-mode-toggle" />,
}));
vi.mock('../DiffStats', () => ({
  DiffStats: () => <div data-testid="diff-stats" />,
}));

describe('GenerateView', () => {
  let store: Record<string, unknown>;

  beforeEach(() => {
    vi.clearAllMocks();
    localGitMock.repos = [];
    generationMock.startGeneration.mockResolvedValue({
      review_id: 'review-new',
      run_id: 'run-new',
      status: 'queued',
    });
    store = {
      diffText: 'diff --git a/a.ts b/a.ts\n--- a/a.ts\n+++ b/a.ts\n+const value = 1;',
      setDiffText: vi.fn(),
      agentId: 'test-agent',
      setAgentId: vi.fn(),
      agentConfigPreferences: {},
      setParsedDiff: vi.fn(),
      setReviewId: vi.fn(),
      setTasks: vi.fn(),
      selectTask: vi.fn(),
      selectFeedback: vi.fn(),
      selectFile: vi.fn(),
      reviewViewMode: 'summary',
      setReviewViewMode: vi.fn(),
      pendingSource: null,
      setPendingSource: vi.fn(),
      selectedRepoId: '',
      setSelectedRepoId: vi.fn(),
      prRef: '',
      setPrRef: vi.fn(),
      viewMode: 'diff',
      setViewMode: vi.fn(),
    };
    (useAppStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: typeof store) => unknown) => selector(store)
    );
  });

  it('creates a review, clears the composer, and opens its activity page', async () => {
    const onNavigate = vi.fn();
    render(<GenerateView onNavigate={onNavigate} />);

    fireEvent.click(await screen.findByTestId('start-review'));

    await waitFor(() => {
      expect(generationMock.startGeneration).toHaveBeenCalledTimes(1);
      expect(store.setReviewId).toHaveBeenCalledWith('review-new');
      expect(onNavigate).toHaveBeenCalledWith('review');
    });

    expect(store.setTasks).toHaveBeenCalledWith([]);
    expect(store.setReviewViewMode).toHaveBeenCalledWith('summary');
    expect(store.setDiffText).toHaveBeenLastCalledWith('');
    expect(screen.queryByText('Activity')).not.toBeInTheDocument();
  });

  it('centers the remote input only while the composer is empty', async () => {
    store.diffText = '';
    const { rerender } = render(<GenerateView onNavigate={vi.fn()} />);

    expect(screen.getByTestId('vcs-input-card')).toHaveAttribute('data-prominent', 'true');
    expect(screen.getByText('Start a review')).toBeInTheDocument();

    store.diffText = 'diff --git a/a.ts b/a.ts\n--- a/a.ts\n+++ b/a.ts\n+const value = 1;';
    rerender(<GenerateView onNavigate={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByTestId('vcs-input-card')).toHaveAttribute('data-prominent', 'false');
    });
    expect(screen.queryByText('Start a review')).not.toBeInTheDocument();
  });

  it('loads local refs into the diff preview and keeps their resolved source', async () => {
    store.diffText = '';
    store.selectedRepoId = 'on-prem';
    localGitMock.repos = [{ id: 'on-prem', name: 'Internal repo', path: '/repo', remotes: [] }];
    const source = {
      type: 'local_git',
      repo_id: 'on-prem',
      base_ref: 'release/v1',
      head_ref: 'HEAD',
      base_sha: 'abc',
      head_sha: 'def',
    };
    const diff = {
      diff_text: 'diff --git a/a.ts b/a.ts\n--- a/a.ts\n+++ b/a.ts\n+const a = 1;',
      source,
    };
    localGitMock.fetch.mockResolvedValue(diff);
    const { rerender } = render(<GenerateView onNavigate={vi.fn()} />);
    fireEvent.click(screen.getByRole('button', { name: 'Local Git' }));
    fireEvent.change(screen.getByRole('textbox', { name: 'Base ref' }), {
      target: { value: 'release/v1' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Compare changes' }));
    await waitFor(() =>
      expect(localGitMock.fetch).toHaveBeenCalledWith('on-prem', 'release/v1', 'HEAD')
    );
    await waitFor(() => expect(store.setPendingSource).toHaveBeenCalledWith(source));
    expect(store.setParsedDiff).toHaveBeenCalledWith(diff);
    expect(store.setViewMode).toHaveBeenCalledWith('diff');
    expect(screen.getByTestId('start-review')).toBeEnabled();
    // Switching a fetched diff to Raw must not immediately switch back to Preview.
    (store.setViewMode as ReturnType<typeof vi.fn>).mockClear();
    store.viewMode = 'raw';
    rerender(<GenerateView onNavigate={vi.fn()} />);
    expect(store.setViewMode).not.toHaveBeenCalled();
  });

  it('requires a linked repository and links to the repositories view', () => {
    store.diffText = '';
    const navigate = vi.fn();
    render(<GenerateView onNavigate={navigate} />);
    fireEvent.click(screen.getByRole('button', { name: 'Local Git' }));
    expect(screen.getByRole('button', { name: 'Compare changes' })).toBeDisabled();
    fireEvent.click(screen.getByRole('button', { name: 'link a repository' }));
    expect(navigate).toHaveBeenCalledWith('repos');
  });

  it('opens the paste editor without remounting it when content arrives', () => {
    store.diffText = '';
    const { rerender } = render(<GenerateView onNavigate={vi.fn()} />);
    fireEvent.click(screen.getByRole('button', { name: 'Paste diff' }));
    expect(screen.queryByText('Start a review')).not.toBeInTheDocument();
    const editor = screen.getByTestId('diff-editor-panel');
    expect(editor.parentElement).not.toHaveClass('invisible');
    store.diffText = 'diff --git a/a.ts b/a.ts\n--- a/a.ts\n+++ b/a.ts\n+const a = 1;';
    rerender(<GenerateView onNavigate={vi.fn()} />);
    expect(screen.getByTestId('diff-editor-panel')).toBe(editor);
  });

  it('shows remote fetch errors inside the empty composer', async () => {
    store.diffText = '';
    store.prRef = 'owner/repo#1';
    remoteMock.fetch.mockRejectedValue('Pull request not found');
    render(<GenerateView onNavigate={vi.fn()} />);
    fireEvent.click(screen.getByRole('button', { name: 'Fetch' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Pull request not found');
    expect(screen.getByText('Start a review')).toBeInTheDocument();
    expect(screen.getByTestId('start-review')).toBeDisabled();
  });

  it('shows Git errors and prevents starting a review after a failed comparison', async () => {
    store.diffText = '';
    store.selectedRepoId = 'on-prem';
    localGitMock.repos = [{ id: 'on-prem', name: 'Internal repo', path: '/repo', remotes: [] }];
    localGitMock.fetch.mockRejectedValue('Could not resolve ref: missing-branch');
    render(<GenerateView onNavigate={vi.fn()} />);
    fireEvent.click(screen.getByRole('button', { name: 'Local Git' }));
    fireEvent.click(screen.getByRole('button', { name: 'Compare changes' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not resolve ref');
    expect(screen.getByTestId('start-review')).toBeDisabled();
    expect(store.setPendingSource).not.toHaveBeenCalled();
  });

  it('passes the local Git source and repository into review generation', async () => {
    store.selectedRepoId = 'on-prem';
    store.pendingSource = {
      type: 'local_git',
      repo_id: 'on-prem',
      base_ref: 'main',
      head_ref: 'HEAD',
      base_sha: 'abc',
      head_sha: 'def',
    };
    render(<GenerateView onNavigate={vi.fn()} />);
    fireEvent.click(await screen.findByTestId('start-review'));
    await waitFor(() =>
      expect(generationMock.startGeneration).toHaveBeenCalledWith(
        expect.objectContaining({ repoId: 'on-prem', source: store.pendingSource })
      )
    );
  });
});
