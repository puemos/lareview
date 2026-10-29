import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, within } from '@testing-library/react';
import { RulesView } from './RulesView';

const mocks = vi.hoisted(() => ({
  create: vi.fn(),
  update: vi.fn(),
  remove: vi.fn(),
  reset: vi.fn(),
}));
const rule = {
  id: 'rule-1',
  scope: 'global',
  text: 'Verify boundary inputs',
  category: 'testing',
  glob: null,
  repo_id: null,
  enabled: true,
  created_at: '2026-10-29T12:00:00Z',
  updated_at: '2026-10-29T12:00:00Z',
};
vi.mock('../../hooks/useRules', () => ({
  useRules: () => ({
    data: [rule],
    isLoading: false,
    isError: false,
    createRule: { mutate: mocks.create, isPending: false },
    updateRule: { mutate: mocks.update, isPending: false },
    removeRule: { mutate: mocks.remove, reset: mocks.reset, isPending: false },
  }),
  useRuleRejectionStats: () => ({ data: [] }),
}));
vi.mock('../../hooks/useRepos', () => ({
  useRepos: () => ({ data: [{ id: 'repo-1', name: 'Internal project', remotes: [] }] }),
}));
vi.mock('../../hooks/useRuleLibrary', () => ({
  useRuleLibrary: () => ({ allRules: { data: [] }, addFromLibrary: { isPending: false } }),
}));

const originalShowModal = Object.getOwnPropertyDescriptor(HTMLDialogElement.prototype, 'showModal');
const originalClose = Object.getOwnPropertyDescriptor(HTMLDialogElement.prototype, 'close');
beforeAll(() => {
  Object.defineProperty(HTMLDialogElement.prototype, 'showModal', {
    configurable: true,
    value() {
      this.setAttribute('open', '');
    },
  });
  Object.defineProperty(HTMLDialogElement.prototype, 'close', {
    configurable: true,
    value() {
      this.removeAttribute('open');
    },
  });
});
afterAll(() => {
  if (originalShowModal)
    Object.defineProperty(HTMLDialogElement.prototype, 'showModal', originalShowModal);
  else delete (HTMLDialogElement.prototype as Partial<HTMLDialogElement>).showModal;
  if (originalClose) Object.defineProperty(HTMLDialogElement.prototype, 'close', originalClose);
  else delete (HTMLDialogElement.prototype as Partial<HTMLDialogElement>).close;
});

describe('Rules', () => {
  beforeEach(() => vi.clearAllMocks());
  it('searches rule content and exposes the enabled state as a switch', () => {
    render(<RulesView />);
    fireEvent.click(screen.getByRole('switch', { name: 'Rule: Verify boundary inputs' }));
    expect(mocks.update).toHaveBeenCalledWith(
      expect.objectContaining({ id: 'rule-1', input: expect.objectContaining({ enabled: false }) })
    );
    fireEvent.change(screen.getByRole('searchbox', { name: 'Search rules' }), {
      target: { value: 'missing' },
    });
    expect(screen.getByText('No matching rules')).toBeInTheDocument();
  });
  it('requires repository selection for scoped rules and submits trimmed form content', () => {
    render(<RulesView />);
    fireEvent.click(screen.getByRole('button', { name: 'Add rule' }));
    const dialog = screen.getByRole('dialog', { name: 'Add rule' });
    const field = within(dialog).getByRole('textbox', { name: 'Rule text' });
    expect(field).toHaveFocus();
    fireEvent.change(field, { target: { value: '  Check retries  ' } });
    fireEvent.change(within(dialog).getByLabelText('Scope'), { target: { value: 'repo' } });
    expect(within(dialog).getByRole('button', { name: 'Add rule' })).toBeDisabled();
    fireEvent.change(within(dialog).getByLabelText('Repository'), { target: { value: 'repo-1' } });
    fireEvent.submit(field.closest('form')!);
    expect(mocks.create).toHaveBeenCalledWith(
      {
        scope: 'repo',
        repo_id: 'repo-1',
        text: 'Check retries',
        enabled: true,
        glob: null,
        category: null,
      },
      expect.any(Object)
    );
  });
  it('confirms deletion and lets Escape cancel without deleting', () => {
    render(<RulesView />);
    const trigger = screen.getByRole('button', { name: 'Delete rule: Verify boundary inputs' });
    trigger.focus();
    fireEvent.click(trigger);
    const dialog = screen.getByRole('dialog', { name: 'Delete rule' });
    expect(mocks.remove).not.toHaveBeenCalled();
    fireEvent(dialog, new Event('cancel', { cancelable: true }));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(trigger).toHaveFocus();
    fireEvent.click(trigger);
    fireEvent.click(
      within(screen.getByRole('dialog')).getByRole('button', { name: 'Delete rule' })
    );
    expect(mocks.remove).toHaveBeenCalledWith('rule-1', expect.any(Object));
  });
});
