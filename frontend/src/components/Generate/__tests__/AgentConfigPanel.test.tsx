import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { TooltipProvider } from '../../Common/Tooltip';
import { useAppStore } from '../../../store';
import { AgentConfigPanel } from '../AgentConfigPanel';

vi.mock('../../../hooks/useAgentSessionConfig', () => ({
  useAgentSessionConfig: () => ({
    data: [
      {
        id: 'model',
        name: 'Model',
        category: 'model',
        type: 'select',
        currentValue: 'gpt-5.6-sol',
        options: [{ value: 'gpt-5.6-sol', name: 'GPT-5.6-Sol' }],
      },
      {
        id: 'effort',
        name: 'Effort',
        category: 'thought_level',
        type: 'select',
        currentValue: 'xhigh',
        options: [{ value: 'xhigh', name: 'Xhigh' }],
      },
    ],
    preferences: [],
    isPending: false,
    isFetching: false,
    error: null,
  }),
}));

describe('AgentConfigPanel', () => {
  beforeEach(() => {
    useAppStore.getState().reset();
  });

  it('uses one field rhythm across the compact configuration strip', () => {
    const { container } = render(
      <TooltipProvider>
        <AgentConfigPanel
          agents={[{ id: 'codex', name: 'Codex', available: true }]}
          repos={[
            {
              id: 'repo-1',
              name: 'lareview',
              path: '/tmp/lareview',
              remotes: [],
              allow_snapshot_access: true,
            },
          ]}
          selectedAgentId="codex"
          selectedRepoId=""
          onAgentSelect={vi.fn()}
          onRepoSelect={vi.fn()}
          isStarting={false}
          onGenerate={vi.fn()}
          isDiffValid={false}
        />
      </TooltipProvider>
    );

    for (const label of ['Agent', 'Model', 'Effort', 'Repository']) {
      expect(screen.getByText(label)).toHaveClass('h-3', 'leading-3');
    }

    for (const name of ['Agent', 'Model', 'Effort', 'Repository']) {
      expect(screen.getByRole('combobox', { name })).toHaveClass('h-9', 'rounded-md');
    }

    const modelControl = screen.getByRole('combobox', { name: 'Model' });
    expect(modelControl.parentElement?.parentElement).toHaveClass('gap-3');
    expect(screen.getByRole('button', { name: 'Start Review' })).toBeDisabled();
    expect(container.querySelector('[data-window-drag-region]')).toHaveClass('h-6');
  });
});
