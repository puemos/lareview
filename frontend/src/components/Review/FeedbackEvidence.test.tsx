import { fireEvent, render, screen } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { describe, expect, it, vi } from 'vitest';
import { FeedbackDetail } from './FeedbackDetail';
import { mockNativeDialog } from '../../test/mockNativeDialog';
import type { Comment, DiffFile, Feedback } from '../../types';

mockNativeDialog();
const snippet = vi.hoisted(() =>
  vi.fn().mockResolvedValue({
    file_path: 'src/a.ts',
    hunk_header: '@@ -40,1 +40,1 @@',
    highlighted_line: 40,
    lines: [
      { line_number: 40, content: 'safe();', prefix: '+', is_addition: true, is_deletion: false },
    ],
  })
);
vi.mock('../../hooks/useTauri', () => ({ useTauri: () => ({ getFeedbackDiffSnippet: snippet }) }));
vi.mock('../Common/Mermaid', () => ({ Mermaid: () => null }));
vi.mock('../DiffViewer/DiffViewer', () => ({
  DiffViewer: ({ focusLocation }: { focusLocation: { line: number; side: string } }) => (
    <div>
      Source line {focusLocation.line} ({focusLocation.side})
    </div>
  ),
}));
const file: DiffFile = {
  name: 'src/a.ts',
  new_path: 'src/a.ts',
  old_path: 'src/a.ts',
  hunks: [
    { old_start: 40, old_lines: 1, new_start: 40, new_lines: 1, content: '-unsafe();\n+safe();' },
  ],
};
const feedback = {
  id: 'f1',
  title: 'Avoid unsafe input',
  status: 'todo',
  impact: 'blocking',
  confidence: 1,
  anchor: { file_path: 'src/a.ts', line_number: 40, side: 'new' },
} as Feedback;
const comments = [
  {
    id: 'c1',
    author: 'agent:system',
    created_at: '2026-10-31',
    body: 'See `src/a.ts:40`.\n\n**Evidence:**\n\n```ts\nsafe();\n```',
  },
] as Comment[];

describe('feedback evidence', () => {
  it('shows the excerpt by default and opens code links without losing a reply draft', async () => {
    render(
      <QueryClientProvider
        client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
      >
        <FeedbackDetail
          feedback={feedback}
          files={[file]}
          comments={comments}
          rulesById={{}}
          categoriesById={{}}
          onUpdateStatus={vi.fn()}
          onUpdateImpact={vi.fn()}
          onDelete={vi.fn()}
          onAddComment={vi.fn()}
          isUpdatingStatus={false}
          isUpdatingImpact={false}
          isAddingComment={false}
        />
      </QueryClientProvider>
    );
    expect(await screen.findByText('@@ -40,1 +40,1 @@')).toBeVisible();
    expect(snippet).toHaveBeenCalledWith('f1', 3);
    const reply = screen.getByRole('textbox', { name: 'Reply to feedback' });
    fireEvent.change(reply, { target: { value: 'Keep this thought while I check the source' } });
    fireEvent.click(screen.getAllByRole('link', { name: 'src/a.ts:40' })[0]);
    expect(screen.getByRole('dialog', { name: 'Referenced code' })).toBeVisible();
    expect(screen.getByText('Source line 40 (new)')).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Close referenced code' }));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(reply).toHaveValue('Keep this thought while I check the source');
    fireEvent.click(screen.getByRole('link', { name: 'View src/a.ts:40 in changes' }));
    expect(screen.getByRole('dialog', { name: 'Referenced code' })).toBeVisible();
  });
});
