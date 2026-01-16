import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import React from 'react';
import { GenerateView } from '../GenerateView';
import { useAppStore } from '../../../store';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { GenerationProvider } from '../../../contexts/GenerationContext';

// Mock the hooks used in GenerateView
vi.mock('../../../store');
vi.mock('../../../hooks/useTauri', () => ({
  useTauri: () => ({
    fetchRemotePr: vi.fn(),
  }),
}));
vi.mock('../../../hooks/useAgents', () => ({
  useAgents: () => ({ data: [] }),
}));
vi.mock('../../../hooks/useRepos', () => ({
  useRepos: () => ({ data: [], addRepo: { isPending: false }, cloneRepo: { isPending: false } }),
}));
vi.mock('../../../contexts/useGeneration', () => ({
  useGeneration: () => ({
    startGeneration: vi.fn(),
    stopGeneration: vi.fn(),
  }),
}));

// Mock components that might be complex or unnecessary for this test
vi.mock('../DiffEditorPanel', () => ({
  DiffEditorPanel: () => <div data-testid="diff-editor-panel" />,
  countAdditions: () => 0,
  countDeletions: () => 0,
}));
vi.mock('../AgentConfigPanel', () => ({
  AgentConfigPanel: () => <div data-testid="agent-config-panel" />,
}));
vi.mock('../PlanOverview', () => ({
  PlanOverview: ({ items, isExpanded, onToggle }: any) => (
    <div data-testid="plan-overview">
      <span data-testid="plan-count">{items.length}</span>
      <span data-testid="plan-is-expanded">{isExpanded ? 'true' : 'false'}</span>
      <button data-testid="plan-toggle" onClick={onToggle}>
        Toggle
      </button>
    </div>
  ),
}));
vi.mock('../LiveActivityFeed', () => ({
  LiveActivityFeed: () => <div data-testid="live-activity-feed" />,
}));
vi.mock('../VcsInputCard', () => ({
  VcsInputCard: ({ onClear }: any) => (
    <div data-testid="vcs-input-card">
      <button data-testid="clear-button" onClick={onClear}>
        Clear
      </button>
    </div>
  ),
}));
vi.mock('../ViewModeToggle', () => ({
  ViewModeToggle: () => <div data-testid="view-mode-toggle" />,
}));
vi.mock('../DiffStats', () => ({
  DiffStats: () => <div data-testid="diff-stats" />,
}));

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { retry: false },
  },
});

const wrapper = ({ children }: { children: React.ReactNode }) => (
  <QueryClientProvider client={queryClient}>
    <GenerationProvider>{children}</GenerationProvider>
  </QueryClientProvider>
);

describe('GenerateView Plan Expansion', () => {
  let store: any;

  beforeEach(() => {
    vi.clearAllMocks();
    store = {
      diffText: '',
      setDiffText: vi.fn(),
      setParsedDiff: vi.fn(),
      isGenerating: false,
      plan: null,
      progressMessages: [],
      pendingSource: null,
      setPendingSource: vi.fn(),
      selectedRepoId: '',
      setSelectedRepoId: vi.fn(),
      prRef: '',
      setPrRef: vi.fn(),
      viewMode: 'raw',
      setViewMode: vi.fn(),
      planItems: [],
      setPlanItems: vi.fn(),
      isPlanExpanded: false,
      setIsPlanExpanded: vi.fn(),
      agentId: '',
      setAgentId: vi.fn(),
    };
    (useAppStore as any).mockImplementation((selector: any) => selector(store));
  });

  it('automatically expands the plan panel when the first plan item arrives', () => {
    const { rerender } = render(<GenerateView onNavigate={vi.fn()} />, { wrapper });

    expect(store.setIsPlanExpanded).not.toHaveBeenCalled();

    // Simulate first item arriving
    store.plan = { entries: [{ content: 'Task 1', status: 'pending' }] };
    rerender(<GenerateView onNavigate={vi.fn()} />);

    expect(store.setIsPlanExpanded).toHaveBeenLastCalledWith(true);
  });

  it('does not re-expand if items are added after user collapsed it', () => {
    const { rerender } = render(<GenerateView onNavigate={vi.fn()} />, { wrapper });

    // Initial expansion
    store.plan = { entries: [{ content: 'Task 1', status: 'pending' }] };
    rerender(<GenerateView onNavigate={vi.fn()} />);
    expect(store.setIsPlanExpanded).toHaveBeenCalledWith(true);

    // Reset call count for clarity
    store.setIsPlanExpanded.mockClear();

    // User collapses it
    store.isPlanExpanded = false;
    rerender(<GenerateView onNavigate={vi.fn()} />);

    // Simulate more items arriving
    store.plan = {
      entries: [
        { content: 'Task 1', status: 'pending' },
        { content: 'Task 2', status: 'pending' },
      ],
    };
    rerender(<GenerateView onNavigate={vi.fn()} />);

    // Expect NO new call to expand
    expect(store.setIsPlanExpanded).not.toHaveBeenCalled();
  });

  it('resets the auto-expand state when handleClear is called', () => {
    const { rerender } = render(<GenerateView onNavigate={vi.fn()} />, { wrapper });

    // Initial expansion
    store.plan = { entries: [{ content: 'Task 1', status: 'pending' }] };
    rerender(<GenerateView onNavigate={vi.fn()} />);
    expect(store.setIsPlanExpanded).toHaveBeenCalledWith(true);
    store.setIsPlanExpanded.mockClear();

    // Trigger clear
    const clearButton = screen.getByTestId('clear-button');
    fireEvent.click(clearButton);

    // Simulate what handleClear does to the store (the mock functions don't actually update our 'store' object)
    store.plan = null;
    store.planItems = [];
    rerender(<GenerateView onNavigate={vi.fn()} />);

    store.setIsPlanExpanded.mockClear();

    // Simulate new generation starting with items
    store.plan = { entries: [{ content: 'New Task 1', status: 'pending' }] };
    rerender(<GenerateView onNavigate={vi.fn()} />);

    // Should auto-expand again
    expect(store.setIsPlanExpanded).toHaveBeenCalledWith(true);
  });
});
