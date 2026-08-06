import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { GenerateView } from '../GenerateView';
import { useAppStore } from '../../../store';

const generationMock = vi.hoisted(() => ({
  startGeneration: vi.fn(),
  stopGeneration: vi.fn(),
}));

vi.mock('../../../store');
vi.mock('../../../hooks/useTauri', () => ({
  useTauri: () => ({ fetchRemotePr: vi.fn() }),
}));
vi.mock('../../../hooks/useAgents', () => ({
  useAgents: () => ({ data: [{ id: 'test-agent', name: 'Test Agent', available: true }] }),
}));
vi.mock('../../../hooks/useRepos', () => ({
  useRepos: () => ({
    data: [],
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
  VcsInputCard: ({ prominent }: { prominent?: boolean }) => (
    <div data-testid="vcs-input-card" data-prominent={prominent ? 'true' : 'false'} />
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
});
