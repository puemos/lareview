import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { describe, expect, it, vi } from 'vitest';
import { TaskList } from './TaskList';
import { TaskDetail } from './TaskDetail';
import { FeedbackDetail } from './FeedbackDetail';
import { SelectionModal } from './SelectionModal';
import { mockNativeDialog } from '../../test/mockNativeDialog';
import type { ReviewTask, Feedback, DiffFile } from '../../types';
vi.mock('../../hooks/useTauri', () => ({
  useTauri: () => ({ getFeedbackDiffSnippet: async () => null, openUrl: vi.fn() }),
}));
vi.mock('../DiffViewer/DiffViewer', () => ({ DiffViewer: () => <div>Diff viewer</div> }));
vi.mock('../Common/Mermaid', () => ({ Mermaid: () => <div>Diagram</div> }));
mockNativeDialog();
const file: DiffFile = {
  name: 'a.ts',
  new_path: 'a.ts',
  hunks: [
    {
      old_start: 1,
      old_lines: 3,
      new_start: 1,
      new_lines: 3,
      content: ' first\n-old\n+new\n last',
    },
  ],
};
const task = {
  id: 't1',
  run_id: 'run',
  ai_generated: true,
  risk_level: 'high',
  title: 'Review expiration boundary and anonymous requests',
  description: 'Check callers.',
  stats: { risk: 'high', additions: 1, deletions: 1, tags: [] },
  files: ['a.ts'],
  diff_refs: [{ file: 'a.ts', hunks: file.hunks }],
  status: 'pending',
} as ReviewTask;
const feedback = {
  id: 'f1',
  title: 'Cover boundary',
  status: 'todo',
  impact: 'blocking',
  confidence: 0.9,
  anchor: null,
} as Feedback;
function provider(children: React.ReactNode) {
  return (
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      {children}
    </QueryClientProvider>
  );
}
describe('review workspace refinements', () => {
  it('keeps tasks visible during background refresh with selected semantics', () => {
    render(
      <TaskList
        tasks={[task]}
        selectedTaskId="t1"
        onSelectTask={vi.fn()}
        isLoading={false}
        isFetching
      />
    );
    expect(screen.getByRole('button', { name: /Review expiration/ })).toHaveAttribute(
      'aria-current',
      'true'
    );
  });
  it('handles absent task transitions, counts only changes, and opens related files', () => {
    const select = vi.fn(),
      tab = vi.fn();
    const props = {
      parsedDiff: { diff_text: '', files: [file], total_additions: 1, total_deletions: 1 },
      selectedFile: null,
      onSelectFile: select,
      highlightedHunks: [],
      activeTab: 'description' as const,
      onTabChange: tab,
    };
    const view = render(<TaskDetail {...props} task={null} />);
    view.rerender(<TaskDetail {...props} task={task} />);
    expect(screen.getByRole('button', { name: /Changes\s*\+1\/-1/ })).toBeVisible();
    expect(screen.queryByRole('button', { name: 'Diagram' })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'a.ts' }));
    expect(select).toHaveBeenCalledWith(file);
    expect(tab).toHaveBeenCalledWith('diff');
    view.rerender(<TaskDetail {...props} task={null} />);
  });
  it('replaces a stale selected file on task diff navigation', async () => {
    const select = vi.fn();
    render(
      <TaskDetail
        task={task}
        parsedDiff={{ diff_text: '', files: [file], total_additions: 1, total_deletions: 1 }}
        selectedFile={{ ...file, name: 'other.ts' }}
        onSelectFile={select}
        highlightedHunks={[]}
        activeTab="diff"
        onTabChange={vi.fn()}
      />
    );
    await waitFor(() => expect(select).toHaveBeenCalledWith(file));
  });
  it('preserves replies on failure and clears them only after success', async () => {
    const add = vi
      .fn()
      .mockRejectedValueOnce(new Error('Offline'))
      .mockResolvedValueOnce(undefined);
    render(
      provider(
        <FeedbackDetail
          feedback={feedback}
          rulesById={{}}
          categoriesById={{}}
          comments={[]}
          onUpdateStatus={vi.fn()}
          onUpdateImpact={vi.fn()}
          onDelete={vi.fn()}
          onAddComment={add}
          isUpdatingStatus={false}
          isUpdatingImpact={false}
          isAddingComment={false}
        />
      )
    );
    const input = screen.getByRole('textbox', { name: 'Reply to feedback' });
    fireEvent.change(input, { target: { value: 'Keep this draft' } });
    fireEvent.click(screen.getByRole('button', { name: 'Reply' }));
    await screen.findByText('Reply was not sent');
    expect(input).toHaveValue('Keep this draft');
    fireEvent.click(screen.getByRole('button', { name: 'Reply' }));
    await waitFor(() => expect(input).toHaveValue(''));
    expect(add).toHaveBeenCalledTimes(2);
  });
  it('preserves export selections during refetch and defaults to clipboard', async () => {
    const confirm = vi.fn().mockRejectedValue(new Error('Clipboard unavailable'));
    const props = {
      isOpen: true,
      onClose: vi.fn(),
      onConfirm: confirm,
      tasks: [task],
      feedbacks: [feedback],
      remoteProviderName: 'GitHub',
    };
    const view = render(<SelectionModal {...props} />);
    fireEvent.click(screen.getByRole('checkbox', { name: task.title }));
    view.rerender(
      <SelectionModal {...props} tasks={[{ ...task }]} feedbacks={[{ ...feedback }]} />
    );
    expect(screen.getByRole('checkbox', { name: task.title })).not.toBeChecked();
    fireEvent.click(screen.getByRole('button', { name: 'Copy Markdown' }));
    await screen.findByText('Could not export review');
    expect(confirm).toHaveBeenCalledWith('markdown', [], ['f1']);
  });
});
