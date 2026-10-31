import React, { useState, useEffect, useCallback } from 'react';
import {
  Gear,
  Terminal,
  Code,
  Copy,
  CaretRight,
  CaretDown,
  CheckCircle,
  Warning,
  FloppyDisk,
  ArrowsClockwise,
  Check,
  GithubLogo,
  GitlabLogo,
  GitDiff,
  Funnel,
  Plus,
  Trash,
  Robot,
  Timer,
  TextAa,
} from '@phosphor-icons/react';
import type {
  ViewType,
  Agent,
  VcsStatus as VcsStatusType,
  EditorCandidate,
  EditorConfig,
  CliStatus,
} from '../../types';
import { toast } from 'sonner';
import { PageHeader } from '../Common/PageHeader';
import { InlineError } from '../Common/InlineError';
import { SearchField } from '../Common/SearchField';
import { EmptyPanel } from '../Common/EmptyPanel';
import { Dialog } from '../Common/Dialog';
import {
  agentArgumentError,
  formatAgentArguments,
  parseAgentArguments,
} from '../../lib/agent-arguments';
import { useTauri } from '../../hooks/useTauri';
import { useDelayedLoading } from '../../hooks/useDelayedLoading';
import { useFeedbackFilterConfig, useTimeoutConfig } from '../../hooks/useSettings';
import { VcsSkeleton, CliSkeleton, EditorSkeleton, AgentsSkeleton } from './SettingsSkeleton';
import { AppearanceSettings } from './AppearanceSettings';

interface SettingsViewProps {
  onNavigate: (view: ViewType) => void;
}

export const SettingsView: React.FC<SettingsViewProps> = () => {
  const [activeTab, setActiveTab] = useState<
    'vcs' | 'cli' | 'editor' | 'feedback' | 'agents' | 'appearance'
  >('vcs');

  return (
    <div className="bg-bg-primary flex h-full flex-col">
      <PageHeader title="Settings" description="Connect your tools and tune review behavior." />

      <div className="settings-layout flex min-h-0 flex-1 overflow-hidden">
        {/* Sidebar */}
        <div className="settings-navigation">
          <nav aria-label="Settings sections">
            <TabButton
              icon={<TextAa size={14} />}
              label="Appearance"
              isActive={activeTab === 'appearance'}
              onClick={() => setActiveTab('appearance')}
            />
            <TabButton
              icon={<Gear size={14} />}
              label="Git providers"
              isActive={activeTab === 'vcs'}
              onClick={() => setActiveTab('vcs')}
            />
            <TabButton
              icon={<Terminal size={14} />}
              label="Command line"
              isActive={activeTab === 'cli'}
              onClick={() => setActiveTab('cli')}
            />

            <TabButton
              icon={<Code size={14} />}
              label="Editor"
              isActive={activeTab === 'editor'}
              onClick={() => setActiveTab('editor')}
            />
            <TabButton
              icon={<Funnel size={14} />}
              label="Feedback Filters"
              isActive={activeTab === 'feedback'}
              onClick={() => setActiveTab('feedback')}
            />
            <div className="settings-nav-divider pt-3 pb-1">
              <div className="bg-border/50 mx-2 h-px" />
            </div>
            <TabButton
              icon={<Robot size={14} />}
              label="Agents"
              isActive={activeTab === 'agents'}
              onClick={() => setActiveTab('agents')}
            />
          </nav>
        </div>

        {/* Content */}
        <div className="settings-content" key={activeTab}>
          <div className="settings-section space-y-8">
            {activeTab === 'appearance' && <AppearanceSettings />}
            {activeTab === 'vcs' && <VcsSettings />}
            {activeTab === 'cli' && <CliSettings />}
            {activeTab === 'editor' && <EditorSettings />}
            {activeTab === 'feedback' && <FeedbackFilterSettings />}
            {activeTab === 'agents' && <AgentsSettings />}
          </div>
        </div>
      </div>
    </div>
  );
};

interface TabButtonProps {
  icon: React.ReactNode;
  label: string;
  isActive: boolean;
  onClick: () => void;
}

const TabButton: React.FC<TabButtonProps> = ({ icon, label, isActive, onClick }) => (
  <button
    onClick={onClick}
    aria-current={isActive ? 'page' : undefined}
    className={`group relative flex min-h-9 w-full shrink-0 items-center gap-2 rounded-md px-3 py-2 text-xs font-medium whitespace-nowrap transition-colors ${
      isActive
        ? 'bg-bg-tertiary text-text-primary'
        : 'text-text-secondary hover:text-text-primary hover:bg-bg-secondary'
    }`}
  >
    <span
      className={
        isActive ? 'text-text-secondary' : 'text-text-tertiary group-hover:text-text-secondary'
      }
    >
      {icon}
    </span>
    <span>{label}</span>
    {isActive && <CaretRight size={12} className="text-text-tertiary ml-auto" />}
  </button>
);

const SectionHeader: React.FC<{ title: string; description: string }> = ({
  title,
  description,
}) => (
  <div className="settings-section-heading">
    <h2 className="text-text-primary mb-2 text-lg font-semibold tracking-[-0.015em]">{title}</h2>
    <p className="text-text-tertiary text-[0.8125rem] leading-6">{description}</p>
  </div>
);

const VcsSettings: React.FC = () => {
  const { getVcsStatus, getSingleVcsStatus } = useTauri();
  const [error, setError] = useState<string | null>(null);
  const [refreshErrors, setRefreshErrors] = useState<Record<string, string | undefined>>({});
  const [status, setStatus] = useState<VcsStatusType[]>([]);
  const [isChecking, setIsChecking] = useState(true);
  const [loadingMap, setLoadingMap] = useState<Record<string, boolean>>({});

  const checkStatus = useCallback(async () => {
    setIsChecking(true);
    setError(null);
    try {
      const result = await getVcsStatus();
      setStatus(result);
    } catch (error) {
      setError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsChecking(false);
    }
  }, [getVcsStatus]);

  const refreshSingle = async (id: string, name: string) => {
    setLoadingMap(prev => ({ ...prev, [id]: true }));
    setRefreshErrors(prev => ({ ...prev, [id]: undefined }));
    try {
      const updatedStatus = await getSingleVcsStatus(id);
      setStatus(prev => prev.map(item => (item.id === id ? updatedStatus : item)));
      toast('Status Refreshed', {
        description: `${name} connection status updated.`,
      });
    } catch (error) {
      setRefreshErrors(prev => ({
        ...prev,
        [id]: error instanceof Error ? error.message : String(error),
      }));
    } finally {
      setLoadingMap(prev => ({ ...prev, [id]: false }));
    }
  };

  useEffect(() => {
    checkStatus();
  }, [checkStatus]);

  const shouldShowSkeleton = useDelayedLoading(isChecking && status.length === 0);

  return (
    <div className="space-y-6">
      <SectionHeader
        title="Git providers"
        description="Connect GitHub or GitLab to load pull requests and share review feedback."
      />
      {error && (
        <InlineError
          title="Could not load Git connections"
          message={error}
          onRetry={() => {
            void checkStatus();
          }}
          isRetrying={isChecking}
        />
      )}
      {shouldShowSkeleton ? (
        <VcsSkeleton />
      ) : (
        <div className="space-y-4" aria-busy={isChecking}>
          {status.map(item => {
            const isReady = Boolean(item.login && !item.error);
            const isLoading = loadingMap[item.id] || false;
            return (
              <section
                key={item.id}
                aria-label={`${item.name} connection`}
                className="settings-card"
              >
                <div className="flex items-start justify-between gap-4">
                  <div className="flex min-w-0 items-center gap-3">
                    <div className="bg-bg-tertiary/60 text-text-secondary flex h-9 w-9 shrink-0 items-center justify-center rounded-md">
                      {item.id === 'github' ? (
                        <GithubLogo size={20} />
                      ) : item.id === 'gitlab' ? (
                        <GitlabLogo size={20} />
                      ) : (
                        <GitDiff size={20} />
                      )}
                    </div>
                    <div className="min-w-0">
                      <h3 className="text-text-primary text-sm font-medium">{item.name}</h3>
                      <p
                        className={`mt-1 text-xs ${isReady ? 'text-status-done' : 'text-text-tertiary'}`}
                      >
                        {isReady ? `Connected as @${item.login}` : 'Not connected'}
                      </p>
                    </div>
                  </div>
                  <button
                    type="button"
                    onClick={() => refreshSingle(item.id, item.name)}
                    disabled={isLoading}
                    aria-label={`Refresh ${item.name} status`}
                    aria-busy={isLoading}
                    className="text-text-secondary hover:text-text-primary border-border flex h-8 items-center gap-2 rounded-md border px-3 text-xs disabled:opacity-50"
                  >
                    <ArrowsClockwise size={13} className={isLoading ? 'animate-spin' : ''} />
                    {isLoading ? 'Checking…' : 'Refresh'}
                  </button>
                </div>
                {item.cliPath && (
                  <dl className="border-border/50 mt-4 flex flex-wrap items-baseline gap-x-4 gap-y-1 border-t pt-4 text-xs">
                    <dt className="text-text-disabled">Executable</dt>
                    <dd className="text-text-secondary min-w-0 font-mono text-[0.6875rem] break-all select-text">
                      {item.cliPath}
                    </dd>
                  </dl>
                )}
                {!isReady && (
                  <p className="text-text-tertiary mt-3 text-xs leading-5">
                    Sign in with your provider’s CLI, then refresh the connection.
                  </p>
                )}
                {(refreshErrors[item.id] || item.error) && (
                  <div className="mt-3">
                    <InlineError
                      title={`${item.name} connection needs attention`}
                      message={refreshErrors[item.id] ?? item.error ?? undefined}
                    />
                  </div>
                )}
              </section>
            );
          })}
        </div>
      )}
    </div>
  );
};

const CliSettings: React.FC = () => {
  const { getCliStatus, installCli, getVersion } = useTauri();
  const [status, setStatus] = useState<CliStatus | null>(null);
  const [appVersion, setAppVersion] = useState('');
  const [isLoading, setIsLoading] = useState(true);
  const [isInstalling, setIsInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const fetchStatus = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const [next, version] = await Promise.all([getCliStatus(), getVersion()]);
      setStatus(next);
      setAppVersion(version);
    } catch (error) {
      setError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsLoading(false);
    }
  }, [getCliStatus, getVersion]);
  useEffect(() => {
    void fetchStatus();
  }, [fetchStatus]);
  const handleInstall = async () => {
    if (isInstalling) return;
    setIsInstalling(true);
    setError(null);
    try {
      await installCli();
      await fetchStatus();
      toast('Command installed', { description: 'Open a new terminal to use lareview.' });
    } catch (error) {
      setError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsInstalling(false);
    }
  };
  const showSkeleton = useDelayedLoading(isLoading && !status);
  return (
    <div className="space-y-6">
      <SectionHeader
        title="Command line"
        description="Start a review from the terminal, then continue in the app."
      />
      {error && (
        <InlineError
          title="Could not update command-line tools"
          message={error}
          onRetry={() => {
            void fetchStatus();
          }}
          isRetrying={isLoading}
        />
      )}
      {showSkeleton ? (
        <CliSkeleton />
      ) : (
        status && (
          <section className="settings-card">
            <div className="flex flex-wrap items-center justify-between gap-4">
              <div className="flex items-center gap-3">
                <Terminal size={21} className="text-text-tertiary" />
                <div>
                  <h3 className="text-text-primary text-sm font-medium">LaReview command</h3>
                  <p className="text-text-tertiary mt-1 text-xs">
                    {(status.version ?? appVersion)
                      ? `Version ${status.version ?? appVersion}`
                      : 'Version unavailable'}
                  </p>
                </div>
              </div>
              {status.isInstalled ? (
                <span className="text-status-done flex items-center gap-1.5 text-xs">
                  <Check size={14} /> Installed
                </span>
              ) : (
                <button
                  type="button"
                  onClick={handleInstall}
                  disabled={isInstalling || isLoading}
                  aria-busy={isInstalling}
                  className="bg-brand text-brand-fg flex h-9 items-center gap-2 rounded-md px-3 text-xs font-medium"
                >
                  {isInstalling && <ArrowsClockwise size={13} className="animate-spin" />}
                  {isInstalling ? 'Installing…' : 'Install command'}
                </button>
              )}
            </div>
            {status.path && (
              <p className="text-text-tertiary border-border/50 mt-4 border-t pt-4 font-mono text-[0.6875rem] break-all select-text">
                {status.path}
              </p>
            )}
          </section>
        )
      )}
      <section className="settings-card space-y-4">
        <div>
          <h3 className="text-text-primary text-sm font-medium">Common commands</h3>
          <p className="text-text-tertiary mt-2 text-xs leading-5">
            Replace the example refs and repositories with your own.
          </p>
        </div>
        <CommandLine cmd="lareview diff main HEAD" desc="Compare two commits" />
        <CommandLine cmd="lareview pr owner/repo#123" desc="Review a GitHub pull request" />
        <CommandLine cmd="lareview pr 'group/project!123'" desc="Review a GitLab merge request" />
        <CommandLine cmd="lareview status" desc="Review uncommitted changes" />
      </section>
    </div>
  );
};

const CommandLine: React.FC<{ cmd: string; desc: string }> = ({ cmd, desc }) => {
  const { copyToClipboard } = useTauri();
  const [copied, setCopied] = useState(false);
  useEffect(() => {
    if (!copied) return;
    const timer = setTimeout(() => setCopied(false), 1500);
    return () => clearTimeout(timer);
  }, [copied]);
  const copy = async () => {
    try {
      await copyToClipboard(cmd);
      setCopied(true);
    } catch (error) {
      toast.error('Could not copy command', { description: String(error) });
    }
  };
  return (
    <div className="border-border/40 flex items-start justify-between gap-4 border-t pt-4">
      <div className="min-w-0">
        <p className="text-text-tertiary mb-1.5 text-xs">{desc}</p>
        <code className="text-text-primary font-mono text-xs leading-5 break-all select-text">
          {cmd}
        </code>
      </div>
      <button
        type="button"
        aria-label={`Copy ${cmd}`}
        title={copied ? 'Copied' : 'Copy command'}
        onClick={() => {
          void copy();
        }}
        className="text-text-tertiary hover:text-text-primary hover:bg-bg-tertiary flex h-8 w-8 shrink-0 items-center justify-center rounded-md"
      >
        {copied ? <Check size={14} /> : <Copy size={14} />}
      </button>
      <span className="sr-only" role="status">
        {copied ? 'Command copied' : ''}
      </span>
    </div>
  );
};

const EditorSettings: React.FC = () => {
  const { getAvailableEditors, getEditorConfig, updateEditorConfig } = useTauri();
  const [editors, setEditors] = useState<EditorCandidate[]>([]);
  const [config, setConfig] = useState<EditorConfig | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const fieldId = React.useId();
  const fetchData = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const [found, current] = await Promise.all([getAvailableEditors(), getEditorConfig()]);
      setEditors(found);
      setConfig(current);
    } catch (error) {
      setError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsLoading(false);
    }
  }, [getAvailableEditors, getEditorConfig]);
  useEffect(() => {
    void fetchData();
  }, [fetchData]);
  const save = async (editorId: string) => {
    if (isSaving || !editors.some(editor => editor.id === editorId)) return;
    setIsSaving(true);
    setError(null);
    try {
      await updateEditorConfig(editorId);
      setConfig({ preferred_editor_id: editorId });
      toast('Editor saved');
    } catch (error) {
      setError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsSaving(false);
    }
  };
  const selected = editors.find(editor => editor.id === config?.preferred_editor_id);
  const showSkeleton = useDelayedLoading(isLoading && !config);
  return (
    <div className="space-y-6">
      <SectionHeader
        title="Editor"
        description="Choose where files open when you jump from a review into code."
      />
      {error && (
        <InlineError
          title="Could not update editor settings"
          message={error}
          onRetry={() => {
            void fetchData();
          }}
          isRetrying={isLoading}
        />
      )}
      {showSkeleton ? (
        <EditorSkeleton />
      ) : (
        <div className="settings-card space-y-4">
          <div className="product-form">
            <label htmlFor={fieldId} className="text-text-secondary mb-2 block text-xs font-medium">
              Default editor
            </label>
            <select
              id={fieldId}
              value={config?.preferred_editor_id ?? ''}
              onChange={event => {
                void save(event.target.value);
              }}
              disabled={isLoading || isSaving || editors.length === 0}
              className="text-text-primary border-border w-full rounded-md border"
            >
              <option value="" disabled>
                Choose an editor
              </option>
              {config?.preferred_editor_id && !selected && (
                <option value={config.preferred_editor_id} disabled>
                  {config.preferred_editor_id} (unavailable)
                </option>
              )}
              {editors.map(editor => (
                <option key={editor.id} value={editor.id}>
                  {editor.label}
                </option>
              ))}
            </select>
          </div>
          {isSaving && (
            <p role="status" className="text-text-tertiary text-xs">
              Saving editor preference…
            </p>
          )}
          {selected && (
            <p className="text-text-tertiary font-mono text-[0.6875rem] leading-5 break-all select-text">
              {selected.path}
            </p>
          )}
          {!isLoading && !error && editors.length === 0 && (
            <p className="text-text-tertiary text-xs leading-5">
              No supported editors were found. Install an editor, then refresh this list.
            </p>
          )}
          {config?.preferred_editor_id && !selected && !isLoading && (
            <p className="text-status-in_progress text-xs leading-5">
              Your configured editor is unavailable. Choose an installed editor or restore its
              executable.
            </p>
          )}
          <button
            type="button"
            onClick={() => {
              void fetchData();
            }}
            disabled={isLoading || isSaving}
            className="text-text-secondary hover:text-text-primary border-border flex h-8 items-center gap-2 rounded-md border px-3 text-xs"
          >
            <ArrowsClockwise size={13} className={isLoading ? 'animate-spin' : ''} />
            {isLoading ? 'Discovering…' : 'Refresh editors'}
          </button>
        </div>
      )}
    </div>
  );
};

const FeedbackFilterSettings: React.FC = () => {
  const { config, updateThreshold, isUpdating, isLoading, error, refetch } =
    useFeedbackFilterConfig();
  const [draft, setDraft] = useState('');
  const [saveError, setSaveError] = useState<string | null>(null);
  const inputId = React.useId();
  useEffect(() => {
    setDraft(
      config.confidenceThreshold == null ? '' : String(Math.round(config.confidenceThreshold * 100))
    );
  }, [config.confidenceThreshold]);
  const numeric = draft.trim() === '' ? null : Number(draft);
  const valid = numeric === null || (Number.isFinite(numeric) && numeric >= 0 && numeric <= 100);
  const next = numeric === null ? null : numeric / 100;
  const dirty = next !== config.confidenceThreshold;
  const busy = isUpdating || isLoading || Boolean(error);
  const change = (value: string) => {
    setDraft(value);
    setSaveError(null);
  };
  const save = () => {
    if (!valid || !dirty || busy) return;
    setSaveError(null);
    updateThreshold(next, {
      onSuccess: () => toast('Feedback filter saved'),
      onError: (error: Error) => setSaveError(error.message),
    });
  };
  return (
    <div className="space-y-6">
      <SectionHeader
        title="Feedback filters"
        description="Choose a minimum confidence level. Higher thresholds reduce noise and can hide useful findings."
      />
      {error && (
        <InlineError
          title="Could not load feedback preferences"
          message={String(error)}
          onRetry={() => {
            void refetch();
          }}
          isRetrying={isLoading}
        />
      )}
      <form
        className="settings-card space-y-5"
        onSubmit={event => {
          event.preventDefault();
          save();
        }}
      >
        <p className="text-text-secondary text-sm">
          Choose a preset or adjust the value, then apply your changes.
        </p>
        <div className="settings-presets" role="group" aria-label="Confidence presets">
          {[
            { label: 'All feedback', value: '', description: 'No confidence filter' },
            { label: '70% and above', value: '70', description: 'Hide lower confidence' },
            { label: '90% and above', value: '90', description: 'Keep high confidence' },
          ].map(preset => (
            <button
              type="button"
              key={preset.label}
              aria-pressed={draft === preset.value}
              disabled={busy}
              onClick={() => change(preset.value)}
              className={`rounded-md border p-3 text-left transition-colors ${draft === preset.value ? 'border-brand/70 bg-brand/5 text-text-primary' : 'border-border text-text-secondary hover:bg-bg-tertiary'}`}
            >
              <span className="block text-xs font-medium">{preset.label}</span>
              <span className="text-text-tertiary mt-1 block text-[0.6875rem] leading-5">
                {preset.description}
              </span>
            </button>
          ))}
        </div>
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <label htmlFor={`${inputId}-slider`} className="text-text-secondary text-xs">
              Minimum confidence
            </label>
            <output className="text-text-primary font-mono text-xs">
              {valid ? (numeric ?? 0) : '—'}%
            </output>
          </div>
          <input
            id={`${inputId}-slider`}
            type="range"
            aria-valuetext={`${valid ? (numeric ?? 0) : 0}% confidence`}
            min="0"
            max="100"
            step="1"
            value={valid ? (numeric ?? 0) : 0}
            onChange={event => change(event.target.value)}
            disabled={busy}
            className="accent-brand w-full"
          />
        </div>
        <div className="product-form">
          <label htmlFor={inputId} className="text-text-secondary mb-2 block text-xs">
            Custom threshold (%)
          </label>
          <input
            id={inputId}
            type="number"
            min="0"
            max="100"
            step="any"
            placeholder="No filter"
            value={draft}
            onChange={event => change(event.target.value)}
            disabled={busy}
            aria-invalid={!valid}
            aria-describedby={`${inputId}-help`}
            className="text-text-primary border-border w-32 rounded-md border"
          />
          <p
            id={`${inputId}-help`}
            className={`mt-2 text-xs leading-5 ${valid ? 'text-text-tertiary' : 'text-red-400'}`}
          >
            {valid
              ? 'Enter a percentage from 0 to 100, or leave blank to show all feedback.'
              : 'Enter a valid percentage from 0 to 100.'}
          </p>
        </div>
        {saveError && <InlineError title="Could not save feedback filter" message={saveError} />}
        <div className="border-border/50 flex flex-wrap items-center justify-between gap-3 border-t pt-4">
          <p className="text-text-tertiary text-xs">
            Current:{' '}
            {config.confidenceThreshold == null
              ? 'All feedback'
              : `${Math.round(config.confidenceThreshold * 100)}% and above`}
          </p>
          <button
            type="submit"
            disabled={busy || !valid || !dirty}
            aria-busy={isUpdating}
            className="bg-brand text-brand-fg disabled:bg-bg-tertiary disabled:text-text-disabled flex h-9 items-center gap-2 rounded-md px-4 text-xs font-medium"
          >
            {isUpdating && <ArrowsClockwise size={13} className="animate-spin" />}
            {isUpdating ? 'Applying…' : 'Apply changes'}
          </button>
        </div>
      </form>
    </div>
  );
};

const AgentsSettings: React.FC = () => {
  const { getAgents, updateAgentConfig, addCustomAgent, deleteCustomAgent } = useTauri();
  const {
    config: timeoutConfig,
    updateTimeout,
    isUpdating: isUpdatingTimeout,
    isLoading: isLoadingTimeout,
    error: timeoutLoadError,
  } = useTimeoutConfig();
  const [editError, setEditError] = useState<string | null>(null);
  const [addError, setAddError] = useState<string | null>(null);
  const customId = React.useId();
  const [searchQuery, setSearchQuery] = useState('');
  const [availableOnly, setAvailableOnly] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [timeoutError, setTimeoutError] = useState<string | null>(null);
  const timeoutId = React.useId();
  const [agents, setAgents] = useState<Agent[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editState, setEditState] = useState<{ path: string; args: string }>({
    path: '',
    args: '',
  });
  const [savingId, setSavingId] = useState<string | null>(null);
  const [savedId, setSavedId] = useState<string | null>(null);
  const [showAddForm, setShowAddForm] = useState(false);
  const [addForm, setAddForm] = useState({ id: '', label: '', command: '', args: '' });
  const [isAdding, setIsAdding] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);
  const DEFAULT_TIMEOUT_SECS = 1000;
  const [localTimeout, setLocalTimeout] = useState<string>('');

  useEffect(() => {
    setLocalTimeout(String(timeoutConfig.timeoutSecs ?? DEFAULT_TIMEOUT_SECS));
  }, [timeoutConfig.timeoutSecs]);

  const handleTimeoutSave = () => {
    const secs = Number(localTimeout);
    if (
      !Number.isInteger(secs) ||
      secs < 60 ||
      secs > 7200 ||
      isUpdatingTimeout ||
      isLoadingTimeout ||
      timeoutLoadError
    )
      return;
    setTimeoutError(null);
    updateTimeout(secs, {
      onSuccess: () => {
        toast('Timeout Updated', {
          description: `Review timeout set to ${secs}s (~${Math.round(secs / 60)} min).`,
        });
      },
      onError: (error: Error) => {
        setTimeoutError(error.message);
      },
    });
  };

  const handleTimeoutReset = () => {
    if (isUpdatingTimeout || isLoadingTimeout || timeoutLoadError) return;
    setTimeoutError(null);
    setLocalTimeout(String(DEFAULT_TIMEOUT_SECS));
    updateTimeout(null, {
      onSuccess: () => {
        toast('Timeout reset');
      },
      onError: (error: Error) => setTimeoutError(error.message),
    });
  };

  const fetchAgents = useCallback(async () => {
    setIsLoading(true);
    setLoadError(null);
    try {
      const data = await getAgents();
      setAgents(data);
    } catch (error) {
      setLoadError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsLoading(false);
    }
  }, [getAgents]);

  useEffect(() => {
    fetchAgents();
  }, [fetchAgents]);

  useEffect(() => {
    if (savedId) {
      const timer = setTimeout(() => setSavedId(null), 2000);
      return () => clearTimeout(timer);
    }
  }, [savedId]);

  const handleEdit = (agent: Agent) => {
    setEditError(null);
    setEditingId(agent.id);
    setEditState({
      path: agent.path || '',
      args: formatAgentArguments(agent.args || []),
    });
  };

  const handleCancel = () => {
    setEditingId(null);
    setEditState({ path: '', args: '' });
  };

  const handleSave = async (agentId: string) => {
    if (savingId || !editState.path.trim() || agentArgumentError(editState.args)) return;
    setEditError(null);
    setSavingId(agentId);
    try {
      const argsArray = parseAgentArguments(editState.args);

      await updateAgentConfig(agentId, editState.path.trim(), argsArray);
      await fetchAgents();
      setEditingId(null);
      setSavedId(agentId);
      toast('Agent Updated', {
        description: 'Configuration saved successfully.',
      });
    } catch (error) {
      setEditError(error instanceof Error ? error.message : String(error));
    } finally {
      setSavingId(null);
    }
  };

  const handleAddAgent = async () => {
    if (
      isAdding ||
      !addForm.id.trim() ||
      !addForm.label.trim() ||
      !addForm.command.trim() ||
      agentArgumentError(addForm.args) ||
      agents.some(agent => agent.id === addForm.id.trim())
    )
      return;
    setAddError(null);
    setIsAdding(true);
    try {
      const argsArray = parseAgentArguments(addForm.args);

      await addCustomAgent(
        addForm.id.trim(),
        addForm.label.trim(),
        addForm.command.trim(),
        argsArray
      );
      await fetchAgents();
      setShowAddForm(false);
      setAddForm({ id: '', label: '', command: '', args: '' });
      toast('Agent Added', {
        description: `Custom agent "${addForm.label}" has been added.`,
      });
    } catch (error) {
      setAddError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsAdding(false);
    }
  };

  const handleDeleteAgent = async (agentId: string) => {
    setDeletingId(agentId);
    try {
      await deleteCustomAgent(agentId);
      await fetchAgents();
      setConfirmDeleteId(null);
      toast('Agent Deleted', {
        description: 'Custom agent has been removed.',
      });
    } catch (error) {
      console.error('Failed to delete custom agent:', error);
      toast('Failed to delete agent', {
        description: error instanceof Error ? error.message : String(error),
      });
    } finally {
      setDeletingId(null);
    }
  };

  const visibleAgents = agents.filter(
    agent =>
      (!availableOnly || agent.available) &&
      [agent.id, agent.name, agent.path ?? ''].some(value =>
        value.toLowerCase().includes(searchQuery.trim().toLowerCase())
      )
  );
  const shouldShowSkeleton = useDelayedLoading(isLoading && agents.length === 0);

  return (
    <div>
      <div className="settings-agents-heading">
        <div className="max-w-md">
          <SectionHeader
            title="Review agents"
            description="Choose an installed agent or configure your own."
          />
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <button
            onClick={() => {
              setAddError(null);
              setAddForm({ id: '', label: '', command: '', args: '' });
              setShowAddForm(true);
            }}
            disabled={Boolean(savingId || deletingId || isAdding)}
            className="settings-secondary-action"
          >
            <Plus size={14} weight="bold" />
            Add custom agent
          </button>
          <button
            onClick={fetchAgents}
            disabled={isLoading || Boolean(savingId || deletingId || isAdding)}
            className="settings-secondary-action settings-icon-action"
            title="Refresh agents"
          >
            <ArrowsClockwise size={16} className={isLoading ? 'animate-spin' : ''} />
          </button>
        </div>
      </div>

      <details className="settings-timeout mb-6">
        <summary>
          <span className="flex items-center gap-2">
            <Timer size={15} />
            Review timeout
          </span>
          <span className="text-text-tertiary ml-auto text-xs">
            {timeoutConfig.timeoutSecs ?? DEFAULT_TIMEOUT_SECS} seconds
          </span>
          <CaretDown size={13} className="text-text-tertiary" />
        </summary>
        <form
          className="product-form space-y-4 pb-5"
          onSubmit={event => {
            event.preventDefault();
            handleTimeoutSave();
          }}
        >
          <div>
            <label
              htmlFor={timeoutId}
              className="text-text-primary flex items-center gap-2 text-sm font-medium"
            >
              <Timer size={14} />
              Review timeout
            </label>
            <p
              id={`${timeoutId}-help`}
              className={`mt-2 text-xs leading-5 ${Number.isInteger(Number(localTimeout)) && Number(localTimeout) >= 60 && Number(localTimeout) <= 7200 ? 'text-text-tertiary' : 'text-red-400'}`}
            >
              Stop a run after this many seconds. Enter a whole number from 60 to 7,200.
            </p>
          </div>
          <div className="flex flex-wrap items-center gap-3">
            <input
              id={timeoutId}
              type="number"
              min="60"
              max="7200"
              step="1"
              required
              value={localTimeout}
              onChange={event => {
                setLocalTimeout(event.target.value);
                setTimeoutError(null);
              }}
              disabled={isUpdatingTimeout || isLoadingTimeout || Boolean(timeoutLoadError)}
              aria-invalid={
                !Number.isInteger(Number(localTimeout)) ||
                Number(localTimeout) < 60 ||
                Number(localTimeout) > 7200
              }
              aria-describedby={`${timeoutId}-help`}
              className="text-text-primary border-border w-32 rounded-md border font-mono"
            />
            <span className="text-text-tertiary text-xs">seconds</span>
            <div className="ml-auto flex items-center gap-3">
              <button
                type="button"
                onClick={handleTimeoutReset}
                disabled={isUpdatingTimeout || isLoadingTimeout || Boolean(timeoutLoadError)}
                className="text-text-secondary rounded-md px-2 py-2 text-xs"
              >
                Use default
              </button>
              <button
                type="submit"
                disabled={
                  isUpdatingTimeout ||
                  isLoadingTimeout ||
                  Boolean(timeoutLoadError) ||
                  !Number.isInteger(Number(localTimeout)) ||
                  Number(localTimeout) < 60 ||
                  Number(localTimeout) > 7200 ||
                  Number(localTimeout) === (timeoutConfig.timeoutSecs ?? DEFAULT_TIMEOUT_SECS)
                }
                className="bg-brand text-brand-fg disabled:bg-bg-tertiary disabled:text-text-disabled rounded-md px-3 py-2 text-xs font-medium"
              >
                {isUpdatingTimeout ? 'Saving…' : 'Save timeout'}
              </button>
            </div>
          </div>
          {(timeoutError || timeoutLoadError) && (
            <InlineError
              title="Could not update review timeout"
              message={timeoutError ?? String(timeoutLoadError)}
            />
          )}
        </form>
      </details>
      {loadError && (
        <div className="mb-4">
          <InlineError
            title="Could not load agents"
            message={loadError}
            onRetry={() => {
              void fetchAgents();
            }}
            isRetrying={isLoading}
          />
        </div>
      )}

      <Dialog
        isOpen={showAddForm}
        onClose={() => setShowAddForm(false)}
        title="Add custom agent"
        description="Use an unused ID and a command that supports ACP."
        busy={isAdding}
      >
        <form
          className="product-form"
          onSubmit={event => {
            event.preventDefault();
            void handleAddAgent();
          }}
        >
          <fieldset disabled={isAdding} className="min-w-0 space-y-4">
            <div className="grid gap-4 md:grid-cols-2">
              <label className="block">
                <span>Display name</span>
                <input
                  data-autofocus
                  required
                  value={addForm.label}
                  onChange={event => setAddForm(prev => ({ ...prev, label: event.target.value }))}
                  placeholder="My agent"
                  className="text-text-primary border-border w-full rounded-md border"
                />
              </label>
              <label className="block">
                <span>Agent ID</span>
                <input
                  required
                  value={addForm.id}
                  onChange={event => setAddForm(prev => ({ ...prev, id: event.target.value }))}
                  aria-invalid={agents.some(agent => agent.id === addForm.id.trim())}
                  aria-describedby={`${customId}-help`}
                  placeholder="my-agent"
                  className="text-text-primary border-border w-full rounded-md border font-mono"
                />
              </label>
            </div>
            <p id={`${customId}-help`} className="text-text-tertiary text-xs">
              {agents.some(agent => agent.id === addForm.id.trim())
                ? 'That ID is already used. Choose a unique agent ID.'
                : 'The ID identifies this configuration and must be unique.'}
            </p>
            <AgentCommandFields
              command={addForm.command}
              args={addForm.args}
              onCommandChange={command => setAddForm(prev => ({ ...prev, command }))}
              onArgsChange={args => setAddForm(prev => ({ ...prev, args }))}
            />
            {addError && <InlineError title="Could not add agent" message={addError} />}
            <div className="flex justify-end gap-3">
              <button
                type="button"
                onClick={() => setShowAddForm(false)}
                className="text-text-secondary rounded-md px-3 py-2 text-xs"
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={
                  !addForm.id.trim() ||
                  !addForm.label.trim() ||
                  !addForm.command.trim() ||
                  Boolean(agentArgumentError(addForm.args)) ||
                  agents.some(agent => agent.id === addForm.id.trim()) ||
                  isAdding
                }
                className="bg-brand text-brand-fg disabled:bg-bg-tertiary disabled:text-text-disabled rounded-md px-4 py-2 text-xs font-medium"
              >
                {isAdding ? 'Adding…' : 'Add agent'}
              </button>
            </div>
          </fieldset>
        </form>
      </Dialog>

      {agents.length > 0 && (
        <div className="mb-4 space-y-3">
          <SearchField
            label="Search agents"
            value={searchQuery}
            onChange={setSearchQuery}
            placeholder="Agent name or command"
          />
          <div className="text-text-tertiary flex flex-wrap items-center justify-between gap-3 text-xs">
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={availableOnly}
                onChange={event => setAvailableOnly(event.target.checked)}
                className="accent-brand"
              />
              Available only
            </label>
            <span>
              {visibleAgents.length} {visibleAgents.length === 1 ? 'result' : 'results'} ·{' '}
              {agents.filter(agent => agent.available).length} of {agents.length} available
            </span>
          </div>
        </div>
      )}
      {!isLoading && agents.length > 0 && visibleAgents.length === 0 && (
        <EmptyPanel
          title="No matching agents"
          description="Try another name or show unavailable agents."
          action={
            <button
              type="button"
              onClick={() => {
                setSearchQuery('');
                setAvailableOnly(false);
              }}
              className="text-text-secondary border-border rounded-md border px-3 py-2 text-xs"
            >
              Clear filters
            </button>
          }
        />
      )}
      {shouldShowSkeleton ? (
        <AgentsSkeleton />
      ) : (
        <div className="space-y-4">
          {visibleAgents.map(agent => (
            <div key={agent.id} className="settings-agent-row">
              <div className="mb-3 flex flex-wrap items-start justify-between gap-3">
                <div className="flex min-w-0 items-start gap-3">
                  <div className="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center overflow-hidden">
                    {agent.logo ? (
                      <img
                        src={agent.logo}
                        alt=""
                        aria-hidden="true"
                        className="h-5 w-5 object-contain"
                      />
                    ) : (
                      <Robot size={20} weight="fill" className="text-brand" />
                    )}
                  </div>
                  <div>
                    <h3 className="text-text-primary flex flex-wrap items-center gap-x-3 gap-y-1 text-sm font-medium">
                      {agent.name}
                      {agent.available ? (
                        <span className="text-status-done flex items-center gap-1 text-[0.625rem] font-medium">
                          <CheckCircle size={12} weight="fill" />
                          Available
                        </span>
                      ) : (
                        <span className="text-status-in_progress flex items-center gap-1 text-[0.625rem] font-medium">
                          <Warning size={12} weight="fill" />
                          Not found
                        </span>
                      )}
                    </h3>
                    <p className="text-text-tertiary mt-1 text-xs">
                      {agent.id === 'default'
                        ? 'Standard review agent'
                        : agent.description ||
                          (agent.is_custom ? 'Custom agent' : 'Built-in agent')}
                    </p>
                  </div>
                </div>
                {editingId !== agent.id && (
                  <div className="flex items-center gap-2">
                    {savedId === agent.id && (
                      <span className="text-status-done text-xs font-medium">Saved</span>
                    )}
                    <button
                      onClick={() => handleEdit(agent)}
                      aria-label={`Edit ${agent.name}`}
                      disabled={Boolean(savingId || deletingId || isAdding)}
                      className="text-text-secondary hover:text-text-primary hover:bg-bg-tertiary rounded px-3 py-1.5 text-xs font-medium transition-colors"
                    >
                      Edit
                    </button>
                    {agent.is_custom &&
                      (confirmDeleteId === agent.id ? (
                        <div className="flex items-center gap-1">
                          <button
                            onClick={() => handleDeleteAgent(agent.id)}
                            disabled={deletingId === agent.id}
                            className="text-impact-blocking hover:bg-impact-blocking/10 rounded px-2 py-1.5 text-xs font-medium transition-colors disabled:opacity-50"
                          >
                            {deletingId === agent.id ? (
                              <ArrowsClockwise size={14} className="animate-spin" />
                            ) : (
                              'Delete agent'
                            )}
                          </button>
                          <button
                            onClick={() => setConfirmDeleteId(null)}
                            disabled={deletingId === agent.id}
                            className="text-text-secondary hover:text-text-primary rounded px-2 py-1.5 text-xs transition-colors disabled:opacity-50"
                          >
                            Cancel
                          </button>
                        </div>
                      ) : (
                        <button
                          onClick={() => setConfirmDeleteId(agent.id)}
                          className="text-text-tertiary hover:text-impact-blocking hover:bg-impact-blocking/10 rounded p-1.5 transition-colors"
                          title="Delete custom agent"
                        >
                          <Trash size={14} />
                        </button>
                      ))}
                  </div>
                )}
              </div>

              <div className="space-y-3">
                {editingId === agent.id ? (
                  <form
                    className="product-form"
                    onSubmit={event => {
                      event.preventDefault();
                      void handleSave(agent.id);
                    }}
                  >
                    <fieldset disabled={Boolean(savingId)} className="min-w-0 space-y-4">
                      <AgentCommandFields
                        command={editState.path}
                        args={editState.args}
                        onCommandChange={path => setEditState(prev => ({ ...prev, path }))}
                        onArgsChange={args => setEditState(prev => ({ ...prev, args }))}
                        autoFocus
                      />
                      {editError && (
                        <InlineError title="Could not save agent" message={editError} />
                      )}
                      <div className="flex justify-end gap-3">
                        <button
                          type="button"
                          onClick={handleCancel}
                          className="text-text-secondary rounded-md px-3 py-2 text-xs"
                        >
                          Cancel
                        </button>
                        <button
                          type="submit"
                          disabled={
                            Boolean(savingId) ||
                            !editState.path.trim() ||
                            Boolean(agentArgumentError(editState.args))
                          }
                          className="bg-brand text-brand-fg disabled:bg-bg-tertiary disabled:text-text-disabled flex items-center gap-2 rounded-md px-4 py-2 text-xs font-medium"
                        >
                          {savingId ? (
                            <ArrowsClockwise size={13} className="animate-spin" />
                          ) : (
                            <FloppyDisk size={13} />
                          )}
                          {savingId ? 'Saving…' : 'Save changes'}
                        </button>
                      </div>
                    </fieldset>
                  </form>
                ) : (
                  <details className="settings-agent-details">
                    <summary>Command details</summary>
                    <div className="space-y-4 pt-3">
                      <div>
                        <span className="text-text-tertiary mb-1 block text-xs font-medium">
                          Command
                        </span>
                        <code className="text-text-secondary block font-mono text-[0.6875rem] leading-5 break-all select-text">
                          {agent.path || '(Default)'}
                        </code>
                      </div>
                      <div>
                        <span className="text-text-tertiary mb-1 block text-xs font-medium">
                          Arguments
                        </span>
                        {agent.args && agent.args.length > 0 ? (
                          <code className="text-text-secondary block font-mono text-xs leading-6 break-all select-text">
                            {formatAgentArguments(agent.args)}
                          </code>
                        ) : (
                          <span className="text-text-tertiary text-xs italic">None</span>
                        )}
                      </div>
                    </div>
                  </details>
                )}
              </div>
            </div>
          ))}

          {agents.length === 0 && !isLoading && !loadError && (
            <div className="border-border rounded-lg border border-dashed py-12 text-center">
              <Robot size={32} weight="thin" className="text-text-disabled mx-auto mb-3" />
              <p className="text-text-tertiary text-sm">No agents discovered</p>
            </div>
          )}
        </div>
      )}
    </div>
  );
};

interface AgentCommandFieldsProps {
  command: string;
  args: string;
  onCommandChange: (value: string) => void;
  onArgsChange: (value: string) => void;
  autoFocus?: boolean;
}
function AgentCommandFields({
  command,
  args,
  onCommandChange,
  onArgsChange,
  autoFocus,
}: AgentCommandFieldsProps) {
  const id = React.useId();
  const error = agentArgumentError(args);
  return (
    <div className="space-y-4">
      <label className="block">
        <span>Command</span>
        <input
          required
          autoFocus={autoFocus}
          value={command}
          onChange={event => onCommandChange(event.target.value)}
          placeholder="Executable or full path"
          aria-describedby={`${id}-command`}
          className="text-text-primary border-border w-full rounded-md border font-mono"
        />
      </label>
      <p id={`${id}-command`} className="text-text-tertiary text-xs leading-5">
        Enter the command or executable path without quotes. Put flags in Arguments.
      </p>
      <label className="block">
        <span>Arguments</span>
        <textarea
          value={args}
          onChange={event => onArgsChange(event.target.value)}
          rows={3}
          placeholder="--acp --stdio"
          aria-invalid={Boolean(error)}
          aria-describedby={`${id}-args`}
          className="text-text-primary border-border w-full rounded-md border font-mono"
        />
      </label>
      <p
        id={`${id}-args`}
        className={`text-xs leading-5 ${error ? 'text-red-400' : 'text-text-tertiary'}`}
      >
        {error ??
          'Use quotes around values containing spaces. Arguments are passed literally to the agent.'}
      </p>
    </div>
  );
}
