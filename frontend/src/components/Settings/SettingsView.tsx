import React, { useState, useEffect, useCallback } from 'react';
import {
  Gear,
  User,
  Terminal,
  Database,
  Asterisk,
  CaretRight,
  CheckCircle,
  Warning,
  FloppyDisk,
  ArrowsClockwise,
  Check,
  X,
  GithubLogo,
  GitlabLogo,
  GitDiff,
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
import { useTauri } from '../../hooks/useTauri';
import { useDelayedLoading } from '../../hooks/useDelayedLoading';
import { VcsSkeleton, CliSkeleton, EditorSkeleton, AgentsSkeleton } from './SettingsSkeleton';

interface SettingsViewProps {
  onNavigate: (view: ViewType) => void;
}

export const SettingsView: React.FC<SettingsViewProps> = () => {
  const [activeTab, setActiveTab] = useState<'vcs' | 'cli' | 'editor' | 'agents'>('vcs');

  return (
    <div className="bg-bg-primary flex h-full flex-col">
      {/* Header */}
      <div className="border-border flex h-12 shrink-0 items-center gap-3 border-b px-6">
        <Asterisk size={18} weight="fill" className="text-brand" />
        <h1 className="font-display text-text-primary text-sm font-medium tracking-wide">
          Settings
        </h1>
      </div>

      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar */}
        <div className="border-border bg-bg-secondary/30 flex w-[240px] flex-col border-r pt-4">
          <nav className="flex-1 space-y-1 px-3">
            <TabButton
              icon={<Gear size={14} />}
              label="VCS Integration"
              isActive={activeTab === 'vcs'}
              onClick={() => setActiveTab('vcs')}
            />
            <TabButton
              icon={<Terminal size={14} />}
              label="CLI Tools"
              isActive={activeTab === 'cli'}
              onClick={() => setActiveTab('cli')}
            />

            <TabButton
              icon={<Database size={14} />}
              label="Editor"
              isActive={activeTab === 'editor'}
              onClick={() => setActiveTab('editor')}
            />
            <div className="pt-3 pb-1">
              <div className="bg-border/50 mx-2 h-px" />
            </div>
            <TabButton
              icon={<User size={14} />}
              label="Agents"
              isActive={activeTab === 'agents'}
              onClick={() => setActiveTab('agents')}
            />
          </nav>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto">
          <div className="animate-fade-in max-w-3xl space-y-8 p-8 md:p-12">
            {activeTab === 'vcs' && <VcsSettings />}
            {activeTab === 'cli' && <CliSettings />}

            {activeTab === 'editor' && <EditorSettings />}
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
    className={`group relative flex w-full items-center gap-3 rounded-md px-3 py-2 text-xs font-medium transition-all ${
      isActive
        ? 'bg-bg-tertiary text-text-primary shadow-sm'
        : 'text-text-secondary hover:text-text-primary hover:bg-bg-secondary'
    }`}
  >
    {isActive && (
      <div className="bg-brand absolute top-1.5 bottom-1.5 left-0 w-[2px] rounded-r-full" />
    )}
    <span
      className={isActive ? 'text-brand' : 'text-text-tertiary group-hover:text-text-secondary'}
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
  <div className="mb-6">
    <h2 className="text-text-primary mb-1 text-lg font-medium">{title}</h2>
    <p className="text-text-tertiary text-xs leading-relaxed">{description}</p>
  </div>
);

const VcsSettings: React.FC = () => {
  const { getVcsStatus } = useTauri();
  const [status, setStatus] = useState<VcsStatusType[]>([]);
  const [isChecking, setIsChecking] = useState(true);

  const checkStatus = useCallback(
    async (manual = false) => {
      setIsChecking(true);
      try {
        const result = await getVcsStatus();
        setStatus(result);
        if (manual === true) {
          toast('Status Refreshed', {
            description: 'VCS connection status updated.',
          });
        }
      } catch (error) {
        console.error('Failed to check VCS status:', error);
        toast('Failed to refresh status', {
          description: error instanceof Error ? error.message : String(error),
        });
      } finally {
        setIsChecking(false);
      }
    },
    [getVcsStatus]
  );

  useEffect(() => {
    checkStatus();
  }, [checkStatus]);

  const shouldShowSkeleton = useDelayedLoading(isChecking && status.length === 0);

  return (
    <div>
      <div className="mb-6 flex items-center justify-between">
        <SectionHeader
          title="VCS Integration"
          description="Connect your VCS accounts to enable remote reviews and feedback synchronization."
        />
        <div className="flex items-center gap-3">
          {shouldShowSkeleton ? (
            <div className="bg-bg-secondary h-6 w-16 animate-pulse rounded" />
          ) : isChecking ? (
            <span className="text-text-tertiary bg-bg-secondary border-border flex items-center gap-1.5 rounded-md border px-2 py-1 text-[10px] font-medium">
              <ArrowsClockwise size={12} className="animate-spin" />
              Checking...
            </span>
          ) : status.some(item => item.login && !item.error) ? (
            <span className="text-status-done bg-status-done/10 border-status-done/20 flex items-center gap-1.5 rounded-md border px-2 py-1 text-[10px] font-medium">
              <Check size={12} weight="bold" />
              Ready
            </span>
          ) : (
            <span className="text-status-in_progress bg-status-in_progress/10 border-status-in_progress/20 flex items-center gap-1.5 rounded-md border px-2 py-1 text-[10px] font-medium">
              <X size={12} weight="bold" />
              Disconnected
            </span>
          )}
        </div>
      </div>

      {shouldShowSkeleton ? (
        <VcsSkeleton />
      ) : (
        <div className="space-y-6">
          {status.map(item => {
            const isReady = item.login && !item.error;
            return (
              <div
                key={item.id}
                className="bg-bg-secondary/40 border-border space-y-6 rounded-lg border p-6"
              >
                <div className="flex items-center gap-4 border-b border-border pb-4 mb-4">
                  <div className="bg-bg-tertiary border-border text-text-primary flex h-10 w-10 items-center justify-center rounded-lg border">
                    {item.id === 'github' ? (
                      <GithubLogo size={20} weight="fill" />
                    ) : item.id === 'gitlab' ? (
                      <GitlabLogo size={20} weight="fill" />
                    ) : (
                      <GitDiff size={20} weight="fill" />
                    )}
                  </div>
                  <div>
                    <h3 className="text-text-primary text-sm font-medium">{item.name}</h3>
                    <p className="text-text-tertiary text-xs">
                      {item.id === 'github' && 'Integration via gh CLI'}
                      {item.id === 'gitlab' && 'Integration via glab dynamic CLI'}
                      {item.id !== 'github' && item.id !== 'gitlab' && 'VCS Provider'}
                    </p>
                  </div>
                </div>

                <div className="grid grid-cols-[120px_1fr] items-center gap-x-8 gap-y-4">
                  <span className="text-text-disabled text-[10px] font-bold tracking-wider uppercase">
                    Connection
                  </span>
                  <div className="flex items-center gap-2">
                    {isChecking ? (
                      <div className="text-text-tertiary flex items-center gap-2">
                        <ArrowsClockwise size={14} className="animate-spin" />
                        <span className="text-xs">Checking status...</span>
                      </div>
                    ) : item.login ? (
                      <div className="flex items-center gap-2">
                        <span className="text-status-done text-xs font-medium">Connected</span>
                        <span className="text-text-tertiary font-mono text-xs">
                          (@{item.login})
                        </span>
                      </div>
                    ) : (
                      <div className="flex items-center gap-2">
                        <span className="text-status-in_progress text-xs font-medium">
                          Disconnected
                        </span>
                        {item.error && (
                          <span className="text-text-tertiary text-[10px]">
                            (Error: {item.error})
                          </span>
                        )}
                      </div>
                    )}
                  </div>

                  {item.cliPath && (
                    <>
                      <span className="text-text-disabled text-[10px] font-bold tracking-wider uppercase">
                        Executable
                      </span>
                      <span className="text-text-secondary truncate font-mono text-xs">
                        {item.cliPath}
                      </span>
                    </>
                  )}

                  <span className="text-text-disabled text-[10px] font-bold tracking-wider uppercase">
                    Status
                  </span>
                  <span
                    className={`text-xs font-medium ${
                      isReady ? 'text-status-done' : 'text-status-in_progress'
                    }`}
                  >
                    {isReady ? 'Ready' : 'Needs setup'}
                  </span>
                </div>

                <div className="pt-2">
                  <button
                    onClick={() => checkStatus(true)}
                    className="bg-bg-tertiary hover:bg-bg-secondary flex items-center gap-2 rounded-md px-4 py-2 text-xs font-medium transition-colors"
                  >
                    <ArrowsClockwise size={14} />
                    Refresh Status
                  </button>
                </div>
              </div>
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
  const [appVersion, setAppVersion] = useState<string>('');
  const [isInstalling, setIsInstalling] = useState(false);
  const [installError, setInstallError] = useState<string | null>(null);

  const fetchStatus = useCallback(async () => {
    try {
      const [cliStatus, version] = await Promise.all([getCliStatus(), getVersion()]);
      setStatus(cliStatus);
      setAppVersion(version);
    } catch (error) {
      console.error('Failed to fetch CLI status:', error);
    }
  }, [getCliStatus, getVersion]);

  useEffect(() => {
    fetchStatus();
  }, [fetchStatus]);

  const handleInstall = async () => {
    setIsInstalling(true);
    setInstallError(null);
    try {
      await installCli();
      await fetchStatus();
      toast('CLI Tools Installed', {
        description: 'The lareview command is now available in your terminal.',
      });
    } catch (error) {
      setInstallError(error as string);
      toast('Installation Failed', {
        description: String(error),
      });
    } finally {
      setIsInstalling(false);
    }
  };

  const isInstalled = status?.isInstalled;
  const version = status?.version || appVersion;
  const shouldShowSkeleton = useDelayedLoading(!status && !appVersion);

  return (
    <div>
      <SectionHeader
        title="CLI Configuration"
        description="Install command-line tools to generate reviews directly from your terminal."
      />

      {shouldShowSkeleton ? (
        <CliSkeleton />
      ) : (
        <div className="space-y-4">
          <div className="bg-bg-secondary/40 border-border flex items-center justify-between rounded-lg border p-6">
            <div className="flex items-center gap-4">
              <div className="bg-bg-tertiary border-border text-brand flex h-10 w-10 items-center justify-center rounded-lg border">
                <Terminal size={20} weight="fill" />
              </div>
              <div>
                <h3 className="text-text-primary text-sm font-medium">LaReview CLI</h3>
                <p className="text-text-tertiary mt-0.5 text-xs">Version {version}</p>
              </div>
            </div>
            {isInstalled ? (
              <div className="bg-status-done/10 text-status-done border-status-done/20 flex items-center gap-2 rounded-md border px-3 py-1.5 text-xs font-medium">
                <span className="bg-status-done h-1.5 w-1.5 rounded-full" />
                Installed
              </div>
            ) : (
              <button
                onClick={handleInstall}
                disabled={isInstalling}
                className="bg-bg-tertiary text-text-primary hover:bg-bg-secondary border-border rounded-md border px-4 py-2 text-xs font-medium shadow-sm transition-colors disabled:opacity-50"
              >
                {isInstalling ? 'Installing...' : 'Install Tools'}
              </button>
            )}
          </div>

          {installError && (
            <div className="bg-status-in_progress/10 border-status-in_progress/20 text-status-in_progress flex items-center gap-2 rounded-md border px-4 py-2 text-xs">
              <Warning size={14} />
              {installError}
            </div>
          )}

          {status?.path && (
            <div className="text-text-tertiary flex items-center gap-1 text-[10px]">
              <Check size={10} className="text-status-done" />
              Active binary at: <span className="text-text-secondary font-mono">{status.path}</span>
            </div>
          )}

          <div className="border-border bg-bg-primary overflow-hidden rounded-lg border">
            <div className="bg-bg-secondary border-border flex items-center gap-2 border-b px-3 py-1.5">
              <div className="flex gap-1.5">
                <div className="bg-status-ignored h-2.5 w-2.5 rounded-full" />
                <div className="bg-status-in_progress h-2.5 w-2.5 rounded-full" />
                <div className="bg-status-done h-2.5 w-2.5 rounded-full" />
              </div>
              <span className="text-text-disabled ml-2 font-mono text-[10px]">Example Usage</span>
            </div>
            <div className="bg-bg-primary text-text-secondary space-y-2 p-4 font-mono text-xs">
              <CommandLine cmd="lareview diff <from> <to>" desc="Review changes between commits" />
              <CommandLine cmd="lareview pr <owner/repo#number>" desc="Review a GitHub change" />
              <CommandLine cmd="lareview pr <group/project!number>" desc="Review a GitLab change" />
              <CommandLine cmd="lareview status" desc="Review uncommitted changes" />
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

const CommandLine: React.FC<{ cmd: string; desc: string }> = ({ cmd, desc }) => (
  <div className="flex items-center gap-4">
    <span className="text-brand shrink-0">$</span>
    <span className="text-text-primary">{cmd}</span>
    <span className="text-text-disabled ml-auto text-[10px]"># {desc}</span>
  </div>
);

const EditorSettings: React.FC = () => {
  const { getAvailableEditors, getEditorConfig, updateEditorConfig } = useTauri();
  const [editors, setEditors] = useState<EditorCandidate[]>([]);
  const [config, setConfig] = useState<EditorConfig | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  const fetchData = useCallback(async () => {
    setIsLoading(true);
    try {
      const [availableEditors, currentConfig] = await Promise.all([
        getAvailableEditors(),
        getEditorConfig(),
      ]);
      setEditors(availableEditors);
      setConfig(currentConfig);
    } catch (error) {
      console.error('Failed to fetch editor settings:', error);
    } finally {
      setIsLoading(false);
    }
  }, [getAvailableEditors, getEditorConfig]);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const handleEditorChange = async (editorId: string) => {
    try {
      await updateEditorConfig(editorId);
      setConfig({ preferred_editor_id: editorId });
      const editorLabel = editors.find(e => e.id === editorId)?.label;
      toast('Editor Updated', {
        description: editorLabel ? `${editorLabel} set as default.` : 'Default editor updated.',
      });
    } catch (error) {
      console.error('Failed to update editor preference:', error);
      toast('Failed to update editor', {
        description: error instanceof Error ? error.message : String(error),
      });
    }
  };

  const shouldShowSkeleton = useDelayedLoading(isLoading && editors.length === 0);

  return (
    <div>
      <SectionHeader
        title="Editor Configuration"
        description="Choose your preferred editor for opening files from reviews. Available editors are discovered automatically."
      />
      {shouldShowSkeleton ? (
        <EditorSkeleton />
      ) : (
        <div className="bg-bg-secondary/40 border-border rounded-lg border p-6">
          {isLoading ? (
            <div className="text-text-tertiary flex items-center gap-2 text-xs">
              <ArrowsClockwise size={14} className="animate-spin" />
              Discovering editors...
            </div>
          ) : (
            <div>
              <label className="text-text-disabled mb-2 block text-[10px] font-bold tracking-wider uppercase">
                Default Editor
              </label>
              <div className="group relative max-w-xs">
                <select
                  value={config?.preferred_editor_id || ''}
                  onChange={e => handleEditorChange(e.target.value)}
                  className="bg-bg-tertiary border-border text-text-primary focus:border-brand focus:ring-brand/20 w-full cursor-pointer appearance-none rounded-md border py-2 pr-8 pl-3 text-xs transition-all focus:ring-1 focus:outline-none"
                >
                  <option value="" disabled>
                    Select an editor
                  </option>
                  {editors.map(editor => (
                    <option key={editor.id} value={editor.id}>
                      {editor.label}
                    </option>
                  ))}
                </select>
                <div className="text-text-disabled group-hover:text-text-secondary pointer-events-none absolute top-1/2 right-2.5 -translate-y-1/2">
                  <CaretRight size={12} className="rotate-90" />
                </div>
              </div>
              {editors.length === 0 && (
                <p className="text-status-in_progress mt-3 flex items-center gap-1.5 text-xs">
                  <Warning size={14} />
                  No supported editors found in your PATH.
                </p>
              )}
              {config?.preferred_editor_id && (
                <p className="text-text-tertiary mt-3 text-[10px]">
                  Selected:{' '}
                  <span className="text-text-secondary font-mono">
                    {editors.find(e => e.id === config.preferred_editor_id)?.path}
                  </span>
                </p>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
};

const AgentsSettings: React.FC = () => {
  const { getAgents, updateAgentConfig } = useTauri();
  const [agents, setAgents] = useState<Agent[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editState, setEditState] = useState<{ path: string; args: string }>({
    path: '',
    args: '',
  });
  const [savingId, setSavingId] = useState<string | null>(null);
  const [savedId, setSavedId] = useState<string | null>(null);

  const fetchAgents = useCallback(async () => {
    setIsLoading(true);
    try {
      const data = await getAgents();
      setAgents(data);
    } catch (error) {
      console.error('Failed to fetch agents:', error);
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
    setEditingId(agent.id);
    setEditState({
      path: agent.path || '',
      args: (agent.args || []).join(' '),
    });
  };

  const handleCancel = () => {
    setEditingId(null);
    setEditState({ path: '', args: '' });
  };

  const handleSave = async (agentId: string) => {
    setSavingId(agentId);
    try {
      const argsArray = editState.args
        .split(' ')
        .map(s => s.trim())
        .filter(s => s.length > 0);

      await updateAgentConfig(agentId, editState.path, argsArray);
      await fetchAgents();
      setEditingId(null);
      setSavedId(agentId);
      toast('Agent Updated', {
        description: 'Configuration saved successfully.',
      });
    } catch (error) {
      console.error('Failed to update agent:', error);
      toast('Failed to update agent', {
        description: error instanceof Error ? error.message : String(error),
      });
    } finally {
      setSavingId(null);
    }
  };

  const shouldShowSkeleton = useDelayedLoading(isLoading && agents.length === 0);

  return (
    <div>
      <div className="mb-6 flex items-center justify-between">
        <SectionHeader
          title="Review Agents"
          description="Manage and configure the AI agents available for code reviews. Built-in agents can be configured with custom executable paths."
        />
        <button
          onClick={fetchAgents}
          className="text-text-tertiary hover:text-text-primary bg-bg-secondary rounded-md p-2 transition-colors"
          title="Refresh agents"
        >
          <ArrowsClockwise size={16} className={isLoading ? 'animate-spin' : ''} />
        </button>
      </div>

      {shouldShowSkeleton ? (
        <AgentsSkeleton />
      ) : (
        <div className="space-y-4">
          {agents.map(agent => (
            <div
              key={agent.id}
              className={`bg-bg-secondary/40 border-border rounded-lg border p-5 transition-all ${
                editingId === agent.id ? 'ring-brand/20 ring-2' : 'hover:border-brand/30'
              }`}
            >
              <div className="mb-4 flex items-start justify-between">
                <div className="flex items-start gap-4">
                  <div className="bg-bg-tertiary border-border mt-1 flex h-10 w-10 items-center justify-center overflow-hidden rounded-lg border">
                    {agent.logo ? (
                      <img src={agent.logo} alt={agent.name} className="h-6 w-6 object-contain" />
                    ) : (
                      <User size={20} weight="fill" className="text-brand" />
                    )}
                  </div>
                  <div>
                    <h3 className="text-text-primary flex items-center gap-2 text-sm font-semibold">
                      {agent.name}
                      {agent.available ? (
                        <span className="text-status-done flex items-center gap-1 text-[10px] font-medium">
                          <CheckCircle size={12} weight="fill" />
                          Available
                        </span>
                      ) : (
                        <span className="text-status-in_progress flex items-center gap-1 text-[10px] font-medium">
                          <Warning size={12} weight="fill" />
                          Not found
                        </span>
                      )}
                    </h3>
                    <p className="text-text-tertiary mt-1 text-xs">
                      {agent.id === 'default'
                        ? 'Standard review agent'
                        : agent.description || `${agent.name} ACP Agent`}
                    </p>
                  </div>
                </div>
                {editingId !== agent.id && (
                  <div className="flex items-center gap-2">
                    {savedId === agent.id && (
                      <span className="text-status-done animate-in fade-in zoom-in text-xs font-medium duration-300">
                        Saved!
                      </span>
                    )}
                    <button
                      onClick={() => handleEdit(agent)}
                      className="text-text-secondary hover:text-text-primary hover:bg-bg-tertiary rounded px-3 py-1.5 text-xs font-medium transition-colors"
                    >
                      Edit
                    </button>
                  </div>
                )}
              </div>

              <div className="space-y-3">
                {editingId === agent.id ? (
                  <div className="animate-in fade-in slide-in-from-top-1 duration-200">
                    <div className="grid gap-3">
                      <div>
                        <label className="text-text-disabled mb-1.5 block text-[10px] font-bold tracking-wider uppercase">
                          Executable Path / Command
                        </label>
                        <input
                          type="text"
                          value={editState.path}
                          onChange={e => setEditState(prev => ({ ...prev, path: e.target.value }))}
                          placeholder="e.g. /usr/local/bin/agent-bin"
                          className="bg-bg-tertiary border-border text-text-primary placeholder-text-disabled focus:border-brand w-full rounded-md border px-3 py-2 font-mono text-xs transition-all focus:outline-none"
                          autoFocus
                        />
                      </div>

                      <div>
                        <label className="text-text-disabled mb-1.5 block text-[10px] font-bold tracking-wider uppercase">
                          Arguments
                        </label>
                        <input
                          type="text"
                          value={editState.args}
                          onChange={e => setEditState(prev => ({ ...prev, args: e.target.value }))}
                          placeholder="e.g. --model=gpt-4 --temperature=0.7"
                          className="bg-bg-tertiary border-border text-text-primary placeholder-text-disabled focus:border-brand w-full rounded-md border px-3 py-2 font-mono text-xs transition-all focus:outline-none"
                        />
                        <p className="text-text-tertiary mt-1 text-[10px]">
                          Space-separated arguments passed to the agent command.
                        </p>
                      </div>
                    </div>

                    <div className="flex justify-end gap-2 pt-4">
                      <button
                        onClick={handleCancel}
                        disabled={savingId === agent.id}
                        className="text-text-secondary hover:text-text-primary hover:bg-bg-tertiary rounded px-3 py-1.5 text-xs font-medium transition-colors disabled:opacity-50"
                      >
                        Cancel
                      </button>
                      <button
                        onClick={() => handleSave(agent.id)}
                        disabled={savingId === agent.id}
                        className="bg-brand text-bg-primary shadow-custom flex items-center gap-2 rounded-md px-4 py-2 text-xs font-bold transition-all hover:brightness-110 disabled:opacity-50"
                      >
                        {savingId === agent.id ? (
                          <ArrowsClockwise size={14} className="animate-spin" />
                        ) : (
                          <FloppyDisk size={14} weight="fill" />
                        )}
                        Save Changes
                      </button>
                    </div>
                  </div>
                ) : (
                  <div className="space-y-4">
                    <div>
                      <span className="text-text-disabled mb-1 block text-[10px] font-bold tracking-wider uppercase">
                        Path
                      </span>
                      <code className="bg-bg-tertiary text-text-secondary block truncate rounded px-2 py-1 font-mono text-xs">
                        {agent.path || '(Default)'}
                      </code>
                    </div>
                    <div>
                      <span className="text-text-disabled mb-1 block text-[10px] font-bold tracking-wider uppercase">
                        Args
                      </span>
                      {agent.args && agent.args.length > 0 ? (
                        <div className="flex flex-wrap gap-1">
                          {agent.args.map((arg, i) => (
                            <span
                              key={i}
                              className="bg-bg-tertiary border-border text-text-secondary rounded border px-1.5 py-0.5 font-mono text-[10px]"
                            >
                              {arg}
                            </span>
                          ))}
                        </div>
                      ) : (
                        <span className="text-text-tertiary text-xs italic">None</span>
                      )}
                    </div>
                  </div>
                )}
              </div>
            </div>
          ))}

          {agents.length === 0 && !isLoading && (
            <div className="border-border rounded-lg border border-dashed py-12 text-center">
              <User size={32} weight="thin" className="text-text-disabled mx-auto mb-3" />
              <p className="text-text-tertiary text-sm">No agents discovered</p>
            </div>
          )}
        </div>
      )}
    </div>
  );
};
