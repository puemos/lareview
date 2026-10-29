import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { SettingsView } from './SettingsView';
import { mockNativeDialog } from '../../test/mockNativeDialog';

mockNativeDialog();
const mocks = vi.hoisted(() => ({
  vcs: vi.fn().mockResolvedValue([]),
  singleVcs: vi.fn(),
  cli: vi.fn().mockResolvedValue({ isInstalled: true, path: '/bin/lareview', version: '1.0' }),
  version: vi.fn().mockResolvedValue('1.0'),
  install: vi.fn(),
  copy: vi.fn().mockResolvedValue(undefined),
  editors: vi.fn().mockResolvedValue([{ id: 'code', label: 'VS Code', path: '/bin/code' }]),
  editorConfig: vi.fn().mockResolvedValue({ preferred_editor_id: null }),
  saveEditor: vi.fn().mockResolvedValue(undefined),
  agents: vi
    .fn()
    .mockResolvedValue([
      {
        id: 'copilot',
        name: 'Copilot',
        available: true,
        path: '/bin/copilot',
        args: ['--model', 'review model', ''],
        is_custom: false,
      },
    ]),
  saveAgent: vi.fn().mockResolvedValue(undefined),
  addAgent: vi.fn(),
  deleteAgent: vi.fn(),
  threshold: vi.fn(),
  timeout: vi.fn(),
}));
vi.mock('../../hooks/useTauri', () => ({
  useTauri: () => ({
    getVcsStatus: mocks.vcs,
    getSingleVcsStatus: mocks.singleVcs,
    getCliStatus: mocks.cli,
    getVersion: mocks.version,
    installCli: mocks.install,
    copyToClipboard: mocks.copy,
    getAvailableEditors: mocks.editors,
    getEditorConfig: mocks.editorConfig,
    updateEditorConfig: mocks.saveEditor,
    getAgents: mocks.agents,
    updateAgentConfig: mocks.saveAgent,
    addCustomAgent: mocks.addAgent,
    deleteCustomAgent: mocks.deleteAgent,
  }),
}));
vi.mock('../../hooks/useSettings', () => ({
  useFeedbackFilterConfig: () => ({
    config: { confidenceThreshold: null },
    updateThreshold: mocks.threshold,
    isUpdating: false,
    isLoading: false,
  }),
  useTimeoutConfig: () => ({
    config: { timeoutSecs: 1000 },
    updateTimeout: mocks.timeout,
    isUpdating: false,
    isLoading: false,
  }),
}));

describe('Settings', () => {
  beforeEach(() => vi.clearAllMocks());
  it('keeps threshold adjustments as drafts until explicitly applied', async () => {
    render(<SettingsView onNavigate={vi.fn()} />);
    fireEvent.click(screen.getByRole('button', { name: 'Feedback Filters' }));
    fireEvent.click(screen.getByRole('button', { name: /70% and above/ }));
    fireEvent.change(screen.getByRole('slider', { name: 'Minimum confidence' }), {
      target: { value: '71' },
    });
    expect(mocks.threshold).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: 'Apply changes' }));
    expect(mocks.threshold).toHaveBeenCalledWith(0.71, expect.any(Object));
    fireEvent.change(screen.getByRole('spinbutton', { name: 'Custom threshold (%)' }), {
      target: { value: '101' },
    });
    expect(screen.getByRole('button', { name: 'Apply changes' })).toBeDisabled();
  });
  it('copies runnable command examples', async () => {
    render(<SettingsView onNavigate={vi.fn()} />);
    fireEvent.click(screen.getByRole('button', { name: 'Command line' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Copy lareview diff main HEAD' }));
    await waitFor(() => expect(mocks.copy).toHaveBeenCalledWith('lareview diff main HEAD'));
  });
  it('labels editor selection and saves only a discovered editor', async () => {
    render(<SettingsView onNavigate={vi.fn()} />);
    fireEvent.click(screen.getByRole('button', { name: 'Editor' }));
    const select = await screen.findByRole('combobox', { name: 'Default editor' });
    await waitFor(() => expect(select).toBeEnabled());
    fireEvent.change(select, { target: { value: 'code' } });
    await waitFor(() => expect(mocks.saveEditor).toHaveBeenCalledWith('code'));
  });
  it('preserves existing quoted arguments when an agent is saved', async () => {
    render(<SettingsView onNavigate={vi.fn()} />);
    fireEvent.click(screen.getByRole('button', { name: 'Agents' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Edit Copilot' }));
    expect(screen.getByRole('textbox', { name: 'Arguments' })).toHaveValue(
      `--model 'review model' ''`
    );
    fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));
    await waitFor(() =>
      expect(mocks.saveAgent).toHaveBeenCalledWith('copilot', '/bin/copilot', [
        '--model',
        'review model',
        '',
      ])
    );
  });
  it('rejects fractional timeouts and duplicate custom agent IDs', async () => {
    render(<SettingsView onNavigate={vi.fn()} />);
    fireEvent.click(screen.getByRole('button', { name: 'Agents' }));
    await screen.findByRole('button', { name: 'Edit Copilot' });
    fireEvent.change(screen.getByRole('spinbutton', { name: 'Review timeout' }), {
      target: { value: '60.5' },
    });
    expect(screen.getByRole('button', { name: 'Save timeout' })).toBeDisabled();
    fireEvent.click(screen.getByRole('button', { name: 'Add custom agent' }));
    const dialog = screen.getByRole('dialog', { name: 'Add custom agent' });
    fireEvent.change(within(dialog).getByLabelText('Display name'), {
      target: { value: 'Team reviewer' },
    });
    fireEvent.change(within(dialog).getByLabelText('Agent ID'), { target: { value: 'copilot' } });
    fireEvent.change(within(dialog).getByLabelText('Command'), { target: { value: 'copilot' } });
    expect(within(dialog).getByRole('button', { name: 'Add agent' })).toBeDisabled();
    expect(mocks.addAgent).not.toHaveBeenCalled();
  });
});
