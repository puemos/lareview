import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, within } from '@testing-library/react';
import { LearningView } from './LearningView';
import { mockNativeDialog } from '../../test/mockNativeDialog';

mockNativeDialog();
const mocks = vi.hoisted(() => ({
  create: vi.fn(),
  update: vi.fn(),
  remove: vi.fn(),
  toggle: vi.fn(),
  compact: vi.fn(),
  reset: vi.fn(),
  refetch: vi.fn(),
}));
const pattern = {
  id: 'pattern-1',
  pattern_text: 'Prefer actionable findings',
  category: 'style',
  file_extension: 'ts',
  source_count: 0,
  is_edited: true,
  enabled: true,
  created_at: '2026-10-29T12:00:00Z',
  updated_at: '2026-10-29T12:00:00Z',
};
let pending = 0;
let queryError: Error | null = null;
vi.mock('../../hooks/useAgents', () => ({
  useAgents: () => ({
    data: [{ id: 'copilot', name: 'Copilot', available: true }],
    isLoading: false,
  }),
}));
vi.mock('../../hooks/useLearnedPatterns', () => ({
  useLearnedPatterns: () => ({
    patterns: {
      data: queryError ? [] : [pattern],
      isLoading: false,
      isError: Boolean(queryError),
      error: queryError,
      refetch: mocks.refetch,
    },
    status: {
      data: { pattern_count: 1, enabled_pattern_count: 1, pending_rejections: pending },
      refetch: mocks.refetch,
    },
    create: { mutate: mocks.create, isPending: false },
    update: { mutate: mocks.update, isPending: false },
    remove: { mutate: mocks.remove, reset: mocks.reset, isPending: false },
    toggle: { mutate: mocks.toggle, isPending: false },
    compact: { mutate: mocks.compact, isPending: false, isSuccess: false },
  }),
}));

describe('Learning', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    pending = 0;
    queryError = null;
  });
  it('searches patterns and exposes enabled states as switches', () => {
    render(<LearningView />);
    fireEvent.click(screen.getByRole('switch', { name: 'Pattern: Prefer actionable findings' }));
    expect(mocks.toggle).toHaveBeenCalledWith({ id: 'pattern-1', enabled: false });
    fireEvent.change(screen.getByRole('searchbox', { name: 'Search patterns' }), {
      target: { value: 'missing' },
    });
    expect(screen.getByText('No matching patterns')).toBeInTheDocument();
  });
  it('submits trimmed pattern content from a labeled form', () => {
    render(<LearningView />);
    fireEvent.click(screen.getByRole('button', { name: 'New pattern' }));
    const dialog = screen.getByRole('dialog', { name: 'New pattern' });
    const field = within(dialog).getByRole('textbox', { name: 'Pattern' });
    expect(field).toHaveFocus();
    expect(within(dialog).getByRole('button', { name: 'Add pattern' })).toBeDisabled();
    fireEvent.change(field, { target: { value: '  Avoid cosmetic findings  ' } });
    fireEvent.change(within(dialog).getByLabelText('File extension (optional)'), {
      target: { value: 'ts' },
    });
    fireEvent.submit(field.closest('form')!);
    expect(mocks.create).toHaveBeenCalledWith(
      {
        pattern_text: 'Avoid cosmetic findings',
        category: null,
        file_extension: 'ts',
        enabled: true,
      },
      expect.any(Object)
    );
  });
  it('explains zero pending items and enables analysis only when there is work', () => {
    const { rerender } = render(<LearningView />);
    fireEvent.click(screen.getByRole('button', { name: 'Analyze rejections' }));
    expect(screen.getByRole('button', { name: 'Analyze' })).toBeDisabled();
    pending = 1;
    rerender(<LearningView />);
    fireEvent.click(screen.getByRole('button', { name: 'Analyze' }));
    expect(mocks.compact).toHaveBeenCalledWith('copilot', expect.any(Object));
  });
  it('cancels deletion and recovers query errors without showing an empty collection', () => {
    const { rerender } = render(<LearningView />);
    fireEvent.click(
      screen.getByRole('button', { name: 'Delete pattern: Prefer actionable findings' })
    );
    expect(mocks.remove).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    queryError = new Error('Database unavailable');
    rerender(<LearningView />);
    expect(screen.getByRole('alert')).toHaveTextContent('Database unavailable');
    expect(screen.queryByText('No learned patterns yet')).not.toBeInTheDocument();
  });
});
