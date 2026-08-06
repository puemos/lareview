import React, { useState, useEffect, useMemo, useCallback } from 'react';
import { Trash, Spinner } from '@phosphor-icons/react';
import { LayoutGroup, motion, useReducedMotion } from 'framer-motion';
import { useTauri } from '../../hooks/useTauri';
import { useAppStore } from '../../store';
import { useAgents } from '../../hooks/useAgents';
import { useRepos } from '../../hooks/useRepos';
import type { ReviewSource, ViewType } from '../../types';
import { useGeneration } from '../../contexts/useGeneration';
import { DiffEditorPanel } from './DiffEditorPanel';
import { AgentConfigPanel } from './AgentConfigPanel';
import { VcsInputCard } from './VcsInputCard';
import { ViewModeToggle } from './ViewModeToggle';
import { DiffStats } from './DiffStats';
import { countAdditions, countDeletions } from './DiffEditorPanel';
import { toAgentConfigSelections } from '../../lib/agent-config';

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
  const [isLoadingPr, setIsLoadingPr] = useState(false);
  const [isStarting, setIsStarting] = useState(false);

  const [validationError, setValidationError] = useState<string | null>(null);
  const shouldReduceMotion = useReducedMotion();

  const { fetchRemotePr } = useTauri();
  const { startGeneration } = useGeneration();
  const { data: agents = [] } = useAgents();
  const { data: repos = [], addRepo, cloneRepo, selectRepoFolder } = useRepos();

  const setDiffTextStore = useAppStore(state => state.setDiffText);
  const agentId = useAppStore(state => state.agentId);
  const setAgentIdStore = useAppStore(state => state.setAgentId);
  const agentConfigPreferences = useAppStore(state => state.agentConfigPreferences);
  const setParsedDiff = useAppStore(state => state.setParsedDiff);
  const setReviewId = useAppStore(state => state.setReviewId);
  const setTasks = useAppStore(state => state.setTasks);
  const selectTask = useAppStore(state => state.selectTask);
  const selectFeedback = useAppStore(state => state.selectFeedback);
  const selectFile = useAppStore(state => state.selectFile);
  const setReviewViewMode = useAppStore(state => state.setReviewViewMode);
  const pendingSource = useAppStore(state => state.pendingSource);
  const setPendingSource = useAppStore(state => state.setPendingSource);
  const selectedRepoId = useAppStore(state => state.selectedRepoId);
  const setSelectedRepoId = useAppStore(state => state.setSelectedRepoId);
  const prRef = useAppStore(state => state.prRef);
  const setPrRef = useAppStore(state => state.setPrRef);
  const viewMode = useAppStore(state => state.viewMode);
  const setViewMode = useAppStore(state => state.setViewMode);

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
  const isComposerEmpty = !diffText.trim() && !pendingSource;

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
    setIsStarting(true);
    const result = await startGeneration({
      diffText,
      agentId,
      repoId: selectedRepoId || undefined,
      source: pendingSource,
      agentConfig: toAgentConfigSelections(agentConfigPreferences[agentId] || []),
    });
    setIsStarting(false);
    if (result) {
      setReviewId(result.review_id);
      setTasks([]);
      selectTask(null);
      selectFeedback(null);
      selectFile(null);
      setReviewViewMode('summary');
      setDiffText('');
      setDiffTextStore('');
      setParsedDiff(null);
      setPendingSource(null);
      setPrRef('');
      setViewMode('raw');
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
    agentConfigPreferences,
    setReviewId,
    setTasks,
    selectTask,
    selectFeedback,
    selectFile,
    setParsedDiff,
    setPendingSource,
    setPrRef,
    setViewMode,
    setReviewViewMode,
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
    setRepoLinkCallout(null);
  }, [setDiffTextStore, setParsedDiff, setPendingSource, setPrRef, setViewMode]);

  return (
    <div className="bg-bg-primary flex h-full flex-col">
      <AgentConfigPanel
        agents={agents}
        repos={repos}
        selectedAgentId={agentId}
        selectedRepoId={selectedRepoId}
        onAgentSelect={setAgentIdStore}
        onRepoSelect={setSelectedRepoId}
        isStarting={isStarting}
        onGenerate={handleGenerate}
        isDiffValid={isDiffValid}
      />

      <div className="flex flex-1 overflow-hidden">
        <LayoutGroup id="generate-source-input">
          <div className="bg-bg-primary relative flex min-w-0 flex-1 flex-col">
            <div className="flex flex-col gap-2 p-4 pb-0">
              <div className="flex items-center gap-3">
                {!isComposerEmpty && (
                  <motion.div
                    layoutId="review-source-input"
                    transition={{
                      layout: {
                        duration: shouldReduceMotion ? 0 : 0.22,
                        ease: [0.23, 1, 0.32, 1],
                      },
                    }}
                    className="min-w-0"
                  >
                    <VcsInputCard
                      pendingSource={pendingSource}
                      prRef={prRef}
                      onPrRefChange={setPrRef}
                      onFetch={handleFetchPr}
                      isLoading={isLoadingPr}
                      disabled={isStarting}
                      onClear={handleClear}
                    />
                  </motion.div>
                )}

                <div className="flex-1" />

                <div className="pointer-events-auto flex gap-2">
                  {(diffText.trim() || prRef.trim()) && (
                    <button
                      onClick={handleClear}
                      className="bg-bg-secondary/90 hover:bg-bg-tertiary text-text-secondary hover:text-text-primary ring-border flex h-8 items-center gap-1.5 rounded-md px-3 text-[10px] font-medium shadow-sm ring-1 backdrop-blur-sm transition-[background-color,color,transform] active:scale-[0.97]"
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
                      disabled={isStarting || isRepoLinking}
                      className="flex items-center gap-1 rounded bg-amber-500/20 px-2 py-1 text-[10px] font-semibold text-amber-100 transition-colors hover:bg-amber-500/30 disabled:opacity-60"
                    >
                      {isRepoLinking ? <Spinner size={12} className="animate-spin" /> : null}
                      <span>Clone &amp; Link</span>
                    </button>
                    <button
                      onClick={handleLinkExisting}
                      disabled={isStarting || isRepoLinking}
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

            {isComposerEmpty && (
              <div className="pointer-events-none absolute inset-0 z-10 flex items-center justify-center px-8 pb-20">
                <div className="pointer-events-auto w-full max-w-xl">
                  <div className="mb-4 text-center">
                    <h1 className="text-text-primary text-base font-semibold">Start a review</h1>
                    <p className="text-text-tertiary mt-1 text-xs">
                      Paste a pull request or merge request link
                    </p>
                  </div>
                  <motion.div
                    layoutId="review-source-input"
                    transition={{
                      layout: {
                        duration: shouldReduceMotion ? 0 : 0.22,
                        ease: [0.23, 1, 0.32, 1],
                      },
                    }}
                  >
                    <VcsInputCard
                      pendingSource={pendingSource}
                      prRef={prRef}
                      onPrRefChange={setPrRef}
                      onFetch={handleFetchPr}
                      isLoading={isLoadingPr}
                      disabled={isStarting}
                      onClear={handleClear}
                      prominent
                    />
                  </motion.div>
                  <p className="text-text-disabled mt-3 text-center text-[11px]">
                    Or paste a unified diff directly into the editor
                  </p>
                </div>
              </div>
            )}

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
        </LayoutGroup>
      </div>
    </div>
  );
};
