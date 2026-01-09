import React, { useState, useEffect, useMemo, useCallback } from 'react';
import { GithubLogo, Trash, X } from '@phosphor-icons/react';
import { useTauri } from '../../hooks/useTauri';
import { useAppStore } from '../../store';
import { useAgents } from '../../hooks/useAgents';
import { useRepos } from '../../hooks/useRepos';
import type { ViewType } from '../../types';
import { useGeneration } from '../../contexts/useGeneration';
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
  const lastAutoSwitchedTextRef = React.useRef('');

  const [isLoadingPr, setIsLoadingPr] = useState(false);

  const [validationError, setValidationError] = useState<string | null>(null);

  const { fetchGithubPR } = useTauri();
  const { startGeneration, stopGeneration } = useGeneration();
  const { data: agents = [] } = useAgents();
  const { data: repos = [] } = useRepos();

  const setDiffTextStore = useAppStore(state => state.setDiffText);
  const agentId = useAppStore(state => state.agentId);
  const setAgentIdStore = useAppStore(state => state.setAgentId);
  const setParsedDiff = useAppStore(state => state.setParsedDiff);
  const isGenerating = useAppStore(state => state.isGenerating);
  const plan = useAppStore(state => state.plan);

  const progressMessages = useAppStore(state => state.progressMessages);
  const pendingSource = useAppStore(state => state.pendingSource);
  const setPendingSource = useAppStore(state => state.setPendingSource);
  const selectedRepoId = useAppStore(state => state.selectedRepoId);
  const setSelectedRepoId = useAppStore(state => state.setSelectedRepoId);
  const prRef = useAppStore(state => state.prRef);
  const setPrRef = useAppStore(state => state.setPrRef);
  const viewMode = useAppStore(state => state.viewMode);
  const setViewMode = useAppStore(state => state.setViewMode);
  const planItems = useAppStore(state => state.planItems);
  const setPlanItems = useAppStore(state => state.setPlanItems);
  const isPlanExpanded = useAppStore(state => state.isPlanExpanded);
  const setIsPlanExpanded = useAppStore(state => state.setIsPlanExpanded);

  const globalDiffText = useAppStore(state => state.diffText);

  useEffect(() => {
    if (globalDiffText) {
      setDiffText(globalDiffText);
    }
  }, [globalDiffText]);

  useEffect(() => {
    if (agents.length > 0 && !agentId) {
      setAgentIdStore(agents[0].id);
    }
  }, [agents, agentId, setAgentIdStore]);

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
    if (viewMode === 'raw' && isDiffValid && diffText !== lastAutoSwitchedTextRef.current) {
      setViewMode('diff');
      lastAutoSwitchedTextRef.current = diffText;
    }
  }, [diffText, viewMode, isDiffValid, setViewMode]);

  const handleGenerate = useCallback(async () => {
    setValidationError(null);

    const error = validateDiff(diffText);
    if (error) {
      setValidationError(error);
      return;
    }

    setDiffTextStore(diffText);
    setAgentIdStore(agentId);
    const ok = await startGeneration({
      diffText,
      agentId,
      source: pendingSource,
    });
    if (ok) {
      _onNavigate('review');
    }
  }, [
    diffText,
    agentId,
    validateDiff,
    startGeneration,
    setDiffTextStore,
    setAgentIdStore,
    _onNavigate,
    pendingSource,
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
  }, [prRef, fetchGithubPR, setParsedDiff, setPendingSource, setViewMode]);

  const handleClear = useCallback(() => {
    setDiffText('');
    setDiffTextStore('');
    setParsedDiff(null);
    setPendingSource(null);
    setPrRef('');
    setValidationError(null);
    setViewMode('raw');
    setPlanItems([]);
    setIsPlanExpanded(false);
  }, [
    setDiffTextStore,
    setParsedDiff,
    setPendingSource,
    setPrRef,
    setViewMode,
    setPlanItems,
    setIsPlanExpanded,
  ]);

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
  }, [planItemsToRender.length, setIsPlanExpanded]);

  const handleStop = useCallback(async () => {
    await stopGeneration();
  }, [stopGeneration]);

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
            onAgentSelect={setAgentIdStore}
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
