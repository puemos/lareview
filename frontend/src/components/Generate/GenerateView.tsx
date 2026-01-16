import React, { useState, useEffect, useMemo, useCallback } from 'react';
import { Trash, Spinner } from '@phosphor-icons/react';
import { useTauri } from '../../hooks/useTauri';
import { useAppStore } from '../../store';
import { useAgents } from '../../hooks/useAgents';
import { useRepos } from '../../hooks/useRepos';
import type { ReviewSource, ViewType } from '../../types';
import { useGeneration } from '../../contexts/useGeneration';
import { DiffEditorPanel } from './DiffEditorPanel';
import { AgentConfigPanel } from './AgentConfigPanel';
import { PlanOverview } from './PlanOverview';
import { LiveActivityFeed } from './LiveActivityFeed';
import { VcsInputCard } from './VcsInputCard';
import { ViewModeToggle } from './ViewModeToggle';
import { DiffStats } from './DiffStats';
import { countAdditions, countDeletions } from './DiffEditorPanel';

interface GenerateViewProps {
  onNavigate: (view: ViewType) => void;
}

interface RepoLinkCallout {
  provider: 'github' | 'gitlab';
  repo: string;
  host?: string;
  label: string;
}

const isVcsSource = (
  source: ReviewSource | null
): source is Extract<ReviewSource, { type: 'github_pr' | 'gitlab_mr' }> =>
  !!source && (source.type === 'github_pr' || source.type === 'gitlab_mr');

export const GenerateView: React.FC<GenerateViewProps> = ({ onNavigate: _onNavigate }) => {
  const [diffText, setDiffText] = useState('');
  const lastAutoSwitchedTextRef = React.useRef('');
  const hasAutoExpandedRef = React.useRef(false);

  const [isLoadingPr, setIsLoadingPr] = useState(false);

  const [validationError, setValidationError] = useState<string | null>(null);

  const { fetchRemotePr } = useTauri();
  const { startGeneration, stopGeneration } = useGeneration();
  const { data: agents = [] } = useAgents();
  const { data: repos = [], addRepo, cloneRepo, selectRepoFolder } = useRepos();

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
  const isPlanExpanded = useAppStore(state => state.isPlanExpanded);
  const setIsPlanExpanded = useAppStore(state => state.setIsPlanExpanded);

  const [repoLinkCallout, setRepoLinkCallout] = useState<RepoLinkCallout | null>(null);

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

  const findMatchingRepo = useCallback(
    (source: Extract<ReviewSource, { type: 'github_pr' | 'gitlab_mr' }>) => {
      if (source.type === 'github_pr') {
        const target = `${source.owner}/${source.repo}`.toLowerCase();
        let matchingRepo = repos.find(r =>
          r.remotes.some((url: string) => url.toLowerCase().includes(target))
        );

        if (!matchingRepo) {
          matchingRepo = repos.find(r => r.name.toLowerCase() === source.repo.toLowerCase());
        }

        return matchingRepo || null;
      }

      const targetHost = source.host.toLowerCase();
      const targetPath = source.project_path.toLowerCase();
      let matchingRepo = repos.find(r =>
        r.remotes.some((url: string) => {
          const lower = url.toLowerCase();
          return lower.includes(targetHost) && lower.includes(targetPath);
        })
      );

      if (!matchingRepo) {
        const repoName = source.project_path.split('/').pop() || source.project_path;
        matchingRepo = repos.find(r => r.name.toLowerCase() === repoName.toLowerCase());
      }

      return matchingRepo || null;
    },
    [repos]
  );

  const buildRepoLinkCallout = useCallback(
    (source: Extract<ReviewSource, { type: 'github_pr' | 'gitlab_mr' }>): RepoLinkCallout => {
      if (source.type === 'github_pr') {
        const repo = `${source.owner}/${source.repo}`;
        return {
          provider: 'github',
          repo,
          label: repo,
        };
      }

      const label = `${source.host}/${source.project_path}`;
      return {
        provider: 'gitlab',
        repo: source.project_path,
        host: source.host,
        label,
      };
    },
    []
  );

  const isRepoLinking = addRepo.isPending || cloneRepo.isPending;

  const handleCloneAndLink = useCallback(async () => {
    if (!repoLinkCallout) return;
    const destDir = await selectRepoFolder();
    if (!destDir) return;

    try {
      const linked = await cloneRepo.mutateAsync({
        provider: repoLinkCallout.provider,
        repo: repoLinkCallout.repo,
        host: repoLinkCallout.host,
        destDir,
      });
      setSelectedRepoId(linked.id);
      setRepoLinkCallout(null);
    } catch (error) {
      console.error('Failed to clone repo:', error);
    }
  }, [cloneRepo, repoLinkCallout, selectRepoFolder, setSelectedRepoId]);

  const handleLinkExisting = useCallback(async () => {
    const path = await selectRepoFolder();
    if (!path) return;

    try {
      const linked = await addRepo.mutateAsync(path);
      setSelectedRepoId(linked.id);
      setRepoLinkCallout(null);
    } catch (error) {
      console.error('Failed to link repo:', error);
    }
  }, [addRepo, selectRepoFolder, setSelectedRepoId]);

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
      repoId: selectedRepoId || undefined,
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
    selectedRepoId,
  ]);

  const handleFetchPr = useCallback(async () => {
    if (!prRef.trim()) return;

    setIsLoadingPr(true);
    setValidationError(null);

    await Promise.resolve();
    try {
      const diff = await fetchRemotePr(prRef, null);
      setDiffText(diff.diff_text);
      if (diff.source) {
        setPendingSource(diff.source);

        if (isVcsSource(diff.source)) {
          const matchingRepo = findMatchingRepo(diff.source);

          if (matchingRepo) {
            setSelectedRepoId(matchingRepo.id);
            setRepoLinkCallout(null);
          } else {
            setRepoLinkCallout(buildRepoLinkCallout(diff.source));
          }
        } else {
          setRepoLinkCallout(null);
        }
      } else {
        setRepoLinkCallout(null);
      }
      setParsedDiff(diff);
      setViewMode('diff');
    } catch (error) {
      console.error('Failed to fetch remote review:', error);
      setValidationError(`Failed to fetch remote review: ${error}`);
    } finally {
      setIsLoadingPr(false);
    }
  }, [
    prRef,
    fetchRemotePr,
    setParsedDiff,
    setPendingSource,
    setViewMode,
    findMatchingRepo,
    buildRepoLinkCallout,
    setSelectedRepoId,
  ]);

  const handleClear = useCallback(() => {
    setDiffText('');
    setDiffTextStore('');
    setParsedDiff(null);
    setPendingSource(null);
    setPrRef('');
    setValidationError(null);
    setViewMode('raw');
    setIsPlanExpanded(false);
    hasAutoExpandedRef.current = false;
    setRepoLinkCallout(null);
  }, [setDiffTextStore, setParsedDiff, setPendingSource, setPrRef, setViewMode, setIsPlanExpanded]);

  const planItemsToRender = useMemo(() => {
    return (
      plan?.entries.map(e => ({
        content: e.content,
        status: e.status || 'pending',
      })) || []
    );
  }, [plan]);

  useEffect(() => {
    if (planItemsToRender.length > 0 && !hasAutoExpandedRef.current) {
      setIsPlanExpanded(true);
      hasAutoExpandedRef.current = true;
    }
  }, [planItemsToRender.length, setIsPlanExpanded]);

  return (
    <div className="bg-bg-primary flex h-full flex-col">
      <div className="flex flex-1 overflow-hidden">
        <div className="border-border bg-bg-primary relative flex min-w-0 flex-1 flex-col border-r">
          <div className="flex flex-col gap-2 p-4 pb-0">
            <div className="flex items-center gap-3">
              <VcsInputCard
                pendingSource={pendingSource}
                prRef={prRef}
                onPrRefChange={setPrRef}
                onFetch={handleFetchPr}
                isLoading={isLoadingPr}
                disabled={isGenerating}
                onClear={handleClear}
              />

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

                <ViewModeToggle
                  mode={viewMode}
                  onChange={setViewMode}
                  disabled={!diffText.trim()}
                />
              </div>
            </div>

            {repoLinkCallout && (
              <div className="pointer-events-auto flex items-center justify-between gap-3 rounded-md border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-[11px] text-amber-200 shadow-sm">
                <div className="min-w-0">
                  <div className="font-medium text-amber-100">No linked repo found</div>
                  <div className="truncate text-amber-200/80">
                    Link or clone {repoLinkCallout.label} to enable snapshots.
                  </div>
                </div>
                <div className="flex flex-shrink-0 items-center gap-2">
                  <button
                    onClick={handleCloneAndLink}
                    disabled={isGenerating || isRepoLinking}
                    className="flex items-center gap-1 rounded bg-amber-500/20 px-2 py-1 text-[10px] font-semibold text-amber-100 transition-colors hover:bg-amber-500/30 disabled:opacity-60"
                  >
                    {isRepoLinking ? <Spinner size={12} className="animate-spin" /> : null}
                    <span>Clone &amp; Link</span>
                  </button>
                  <button
                    onClick={handleLinkExisting}
                    disabled={isGenerating || isRepoLinking}
                    className="bg-bg-secondary/80 hover:bg-bg-tertiary text-text-primary rounded px-2 py-1 text-[10px] font-semibold transition-colors disabled:opacity-60"
                  >
                    Link Existing
                  </button>
                  <button
                    onClick={() => setRepoLinkCallout(null)}
                    disabled={isRepoLinking}
                    className="text-text-tertiary hover:text-text-primary px-1 text-[10px] font-semibold transition-colors disabled:opacity-60"
                  >
                    Dismiss
                  </button>
                </div>
              </div>
            )}
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
            onStop={stopGeneration}
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
