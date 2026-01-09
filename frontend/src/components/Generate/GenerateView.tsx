import React, { useState, useEffect, useMemo, useCallback } from 'react';
import { Channel } from '@tauri-apps/api/core';
import { GithubLogo, Trash, X } from '@phosphor-icons/react';
import { useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { useTauri } from '../../hooks/useTauri';
import { useAppStore } from '../../store';
import { useAgents } from '../../hooks/useAgents';
import { useRepos } from '../../hooks/useRepos';
import type { ViewType } from '../../types';
import type { ProgressEventPayload } from '../../hooks/useTauri';
import { DiffEditorPanel } from './DiffEditorPanel';
import { AgentConfigPanel } from './AgentConfigPanel';
import { PlanOverview } from './PlanOverview';
import { LiveActivityFeed } from './LiveActivityFeed';
import { PrInput } from './PrInput';
import { ViewModeToggle } from './ViewModeToggle';
import { DiffStats } from './DiffStats';
import { countAdditions, countDeletions } from './DiffEditorPanel';

interface GenerateViewProps {
  onNavigate: (view: ViewType) => void;
}

export const GenerateView: React.FC<GenerateViewProps> = ({ onNavigate: _onNavigate }) => {
  const [diffText, setDiffText] = useState('');
  const [agentId, setAgentId] = useState('');
  const [selectedRepoId, setSelectedRepoId] = useState<string>('');
  const [isGenerating, setIsGenerating] = useState(false);
  const isGeneratingRef = React.useRef(false);
  const [viewMode, setViewMode] = useState<'raw' | 'diff'>('raw');
  const lastAutoSwitchedTextRef = React.useRef('');

  const [prRef, setPrRef] = useState('');
  const [isLoadingPr, setIsLoadingPr] = useState(false);

  const [planItems, setPlanItems] = useState<string[]>([]);
  const [validationError, setValidationError] = useState<string | null>(null);
  const [isPlanExpanded, setIsPlanExpanded] = useState(false);
  const currentTaskTitleRef = React.useRef<string | null>(null);

  const { generateReview, parseDiff, fetchGithubPR, stop_generation } = useTauri();
  const { data: agents = [] } = useAgents();
  const { data: repos = [] } = useRepos();
  const queryClient = useQueryClient();

  const setDiffTextStore = useAppStore(state => state.setDiffText);
  const setAgentIdStore = useAppStore(state => state.setAgentId);
  const setParsedDiff = useAppStore(state => state.setParsedDiff);
  const setIsGeneratingStore = useAppStore(state => state.setIsGenerating);
  const handleServerUpdate = useAppStore(state => state.handleServerUpdate);
  const updatePlanItemStatus = useAppStore(state => state.updatePlanItemStatus);
  const plan = useAppStore(state => state.plan);

  const addProgressMessage = useAppStore(state => state.addProgressMessage);
  const clearProgressMessages = useAppStore(state => state.clearProgressMessages);
  const progressMessages = useAppStore(state => state.progressMessages);
  const pendingSource = useAppStore(state => state.pendingSource);
  const setPendingSource = useAppStore(state => state.setPendingSource);

  const setReviewId = useAppStore(state => state.setReviewId);
  const setRunId = useAppStore(state => state.setRunId);
  const runId = useAppStore(state => state.runId);
  const setTasks = useAppStore(state => state.setTasks);

  const globalDiffText = useAppStore(state => state.diffText);

  useEffect(() => {
    if (globalDiffText) {
      setDiffText(globalDiffText);
    }
  }, [globalDiffText]);

  useEffect(() => {
    if (agents.length > 0 && !agentId) {
      setAgentId(agents[0].id);
    }
  }, [agents, agentId]);

  const validateDiff = useCallback((text: string): string | null => {
    const trimmed = text.trim();
    if (!trimmed) {
      return 'Please enter a diff to review';
    }
    if (trimmed.length < 10) {
      return 'Diff is too short. Please paste a valid git diff';
    }
    const lines = trimmed.split('\n');
    const hasHeader = lines.some(l => l.startsWith('---')) && lines.some(l => l.startsWith('+++'));
    if (!hasHeader && !trimmed.startsWith('diff ')) {
      return 'Invalid diff format. Expected a git diff with --- and +++ headers';
    }
    return null;
  }, []);

  const diffValidationError = useMemo(() => {
    if (!diffText.trim()) return null;
    return validateDiff(diffText);
  }, [diffText, validateDiff]);

  const isDiffValid = diffText.trim().length > 0 && !diffValidationError;

  // Auto-switch to diff mode on valid pasting
  useEffect(() => {
    if (
      viewMode === 'raw' &&
      isDiffValid &&
      diffText !== lastAutoSwitchedTextRef.current
    ) {
      setViewMode('diff');
      lastAutoSwitchedTextRef.current = diffText;
    }
  }, [diffText, viewMode, isDiffValid]);

  const handleGenerate = useCallback(async () => {
    setValidationError(null);

    const error = validateDiff(diffText);
    if (error) {
      setValidationError(error);
      return;
    }

    if (isGeneratingRef.current) return;

    isGeneratingRef.current = true;
    setIsGenerating(true);
    setIsGeneratingStore(true);
    clearProgressMessages();
    setPlanItems([]);

    setDiffTextStore(diffText);

    setAgentIdStore(agentId);
    try {
      const diff = await parseDiff(diffText);

      setParsedDiff(diff);

      const onProgress = new Channel<ProgressEventPayload>();

      // Generate runId early so we can stop it
      const currentRunId = crypto.randomUUID();
      setRunId(currentRunId);

      onProgress.onmessage = (payload: ProgressEventPayload) => {
        switch (payload.event) {
          case 'MessageDelta': {
            const data = payload.data as { id: string; delta: string };
            handleServerUpdate({
              sessionUpdate: 'agent_message_chunk',
              content: { type: 'text', text: data.delta },
              meta: { message_id: data.id },
            });
            break;
          }
          case 'ThoughtDelta': {
            const data = payload.data as { id: string; delta: string };
            handleServerUpdate({
              sessionUpdate: 'agent_thought_chunk',
              content: { type: 'text', text: data.delta },
              meta: { message_id: data.id },
            });
            break;
          }
          case 'ToolCallStarted': {
            const data = payload.data as {
              tool_call_id: string;
              title: string;
              kind: string;
            };
            addProgressMessage('tool_call', data.title, {
              toolCallId: { id: data.tool_call_id },
              status: 'running',
              kind: data.kind,
            });
            break;
          }
          case 'ToolCallComplete': {
            const data = payload.data as {
              tool_call_id: string;
              status: string;
              title: string;
              raw_input?: unknown;
              raw_output?: unknown;
            };
            const msgs = useAppStore.getState().progressMessages;
            const idx = msgs.findIndex(
              m => m.type === 'tool_call' && m.data?.toolCallId?.id === data.tool_call_id
            );
            if (idx >= 0) {
              const updated = [...msgs];
              updated[idx] = {
                ...updated[idx],
                data: {
                  ...updated[idx].data,
                  status: data.status,
                  raw_input: data.raw_input,
                  raw_output: data.raw_output,
                },
              };
              useAppStore.setState({ progressMessages: updated });
            }
            break;
          }
          case 'Plan': {
            const planData = payload.data as {
              entries: Array<{
                content: string;
                priority: string;
                status: string;
              }>;
            };
            handleServerUpdate({ sessionUpdate: 'plan', ...planData });
            break;
          }
          case 'Log':
            addProgressMessage('log', payload.data as string);
            break;
          case 'TaskStarted': {
            const title = (payload.data as { title: string }).title;
            currentTaskTitleRef.current = title;
            addProgressMessage('task_started', title);
            updatePlanItemStatus(title, 'in_progress');
            setPlanItems(prev => (prev.includes(title) ? prev : [...prev, title]));
            break;
          }
          case 'TaskCompleted':
            addProgressMessage('task_added', `Task completed`);
            if (currentTaskTitleRef.current) {
              updatePlanItemStatus(currentTaskTitleRef.current, 'completed');
            }
            break;
          case 'Completed':
            addProgressMessage('completed', 'Review generation complete!');
            queryClient.invalidateQueries({ queryKey: ['reviews'] });
            break;
          case 'Error':
            addProgressMessage('error', (payload.data as { message: string }).message);
            setIsGenerating(false);
            isGeneratingRef.current = false;
            setIsGeneratingStore(false);
            break;
          default:
            console.warn('[Progress] Unknown event type:', payload.event);
        }
      };

      const result = await generateReview(
        diffText,
        agentId,
        currentRunId,
        pendingSource || undefined,
        onProgress
      );

      setReviewId(result.review_id);
      setTasks([]);

      setIsGenerating(false);
      isGeneratingRef.current = false;
      setIsGeneratingStore(false);
      _onNavigate('review');
    } catch (error: unknown) {
      console.error('Failed to generate review:', error);
      const isCancelled =
        error instanceof Error
          ? error.message.includes('cancelled by user')
          : String(error).includes('cancelled by user');

      if (isCancelled) {
        addProgressMessage('error', 'Generation stopped by user');
        toast('Generation stopped', {
          description: 'The pending review has been deleted.',
        });
      } else {
        addProgressMessage('error', `Failed to generate review: ${error}`);
        toast.error('Generation failed', {
          description: String(error),
        });
      }

      setIsGenerating(false);
      isGeneratingRef.current = false;
      setIsGeneratingStore(false);
    }
  }, [
    diffText,
    agentId,
    validateDiff,
    parseDiff,
    generateReview,
    queryClient,
    setDiffTextStore,
    setAgentIdStore,
    setParsedDiff,
    setIsGeneratingStore,
    setReviewId,
    setRunId,
    setTasks,
    addProgressMessage,
    clearProgressMessages,
    handleServerUpdate,
    _onNavigate,
    pendingSource,
    updatePlanItemStatus,
  ]);

  const handleFetchPr = useCallback(async () => {
    if (!prRef.trim()) return;

    setIsLoadingPr(true);
    setValidationError(null);

    await Promise.resolve();
    try {
      const diff = await fetchGithubPR(prRef);
      setDiffText(diff.diff_text);
      if (diff.source) {
        setPendingSource(diff.source);
      }
      setParsedDiff(diff);
      setViewMode('diff');
    } catch (error) {
      console.error('Failed to fetch PR:', error);
      setValidationError(`Failed to fetch PR: ${error}`);
    } finally {
      setIsLoadingPr(false);
    }
  }, [prRef, fetchGithubPR, setParsedDiff, setPendingSource]);

  const handleClear = useCallback(() => {
    setDiffText('');
    setDiffTextStore('');
    setParsedDiff(null);
    setPendingSource(null);
    setPrRef('');
    setValidationError(null);
    setViewMode('raw');
  }, [setDiffTextStore, setParsedDiff, setPendingSource]);

  const planItemsToRender = useMemo(() => {
    const items =
      plan?.entries.map(e => ({
        content: e.content,
        status: e.status || 'pending',
      })) || [];

    // Add any ad-hoc tasks that aren't in the plan
    planItems.forEach(content => {
      if (!items.find(i => i.content === content)) {
        items.push({ content, status: 'completed' });
      }
    });

    return items;
  }, [plan, planItems]);

  useEffect(() => {
    if (planItemsToRender.length > 0) {
      setIsPlanExpanded(true);
    }
  }, [planItemsToRender.length]);

  const handleStop = useCallback(async () => {
    if (runId) {
      try {
        await stop_generation(runId);
        addProgressMessage('log', 'Stop signal sent...');
      } catch (error) {
        console.error('Failed to stop generation:', error);
        addProgressMessage('error', `Failed to stop: ${error}`);
      }
    }
  }, [runId, stop_generation, addProgressMessage]);

  return (
    <div className="bg-bg-primary flex h-full flex-col">
      <div className="flex flex-1 overflow-hidden">
        <div className="border-border bg-bg-primary relative flex min-w-0 flex-1 flex-col border-r">
          <div className="pointer-events-none absolute top-4 right-4 left-4 z-10 flex items-center gap-3">
            {pendingSource?.type === 'github_pr' ? (
              <div className="animate-in fade-in zoom-in-95 pointer-events-auto flex h-8 items-center gap-2 rounded-md border border-green-500/20 bg-green-500/10 px-3 text-xs font-medium text-green-400 shadow-sm backdrop-blur-sm duration-200">
                <GithubLogo size={14} weight="fill" />
                <span>
                  {pendingSource.owner}/{pendingSource.repo}#{pendingSource.number}
                </span>
                <div className="mx-1 h-3 w-px bg-green-500/20" />
                <button
                  onClick={handleClear}
                  className="rounded p-0.5 text-green-400/70 transition-colors hover:bg-green-500/20 hover:text-green-400"
                >
                  <X size={12} />
                </button>
              </div>
            ) : (
              <PrInput
                prRef={prRef}
                onPrRefChange={setPrRef}
                onFetch={handleFetchPr}
                isLoading={isLoadingPr}
                disabled={isGenerating}
              />
            )}

            <div className="flex-1" />

            <div className="pointer-events-auto flex gap-2">
              {(diffText.trim() || prRef.trim()) && (
                <button
                  onClick={handleClear}
                  className="bg-bg-secondary/90 hover:bg-bg-tertiary text-text-secondary hover:text-text-primary ring-border flex h-8 items-center gap-1.5 rounded-md px-3 text-[10px] font-medium shadow-sm ring-1 backdrop-blur-sm transition-all"
                >
                  <Trash size={13} />
                  <span>Clear</span>
                </button>
              )}

              <ViewModeToggle mode={viewMode} onChange={setViewMode} disabled={!diffText.trim()} />
            </div>
          </div>

          <DiffEditorPanel
            diffText={diffText}
            viewMode={viewMode}
            onDiffTextChange={setDiffText}
            validationError={diffValidationError || validationError}
          />

          <DiffStats
            charCount={diffText.length}
            additions={countAdditions(diffText)}
            deletions={countDeletions(diffText)}
          />
        </div>

        <div className="bg-bg-secondary border-border flex w-[380px] flex-col">
          <AgentConfigPanel
            agents={agents}
            repos={repos}
            selectedAgentId={agentId}
            selectedRepoId={selectedRepoId}
            onAgentSelect={setAgentId}
            onRepoSelect={setSelectedRepoId}
            isGenerating={isGenerating}
            onGenerate={handleGenerate}
            onStop={handleStop}
            isDiffValid={isDiffValid}
          />

          <PlanOverview
            items={planItemsToRender}
            isExpanded={isPlanExpanded}
            onToggle={() => setIsPlanExpanded(!isPlanExpanded)}
          />

          <LiveActivityFeed messages={progressMessages} isRunning={isGenerating} />
        </div>
      </div>
    </div>
  );
};
