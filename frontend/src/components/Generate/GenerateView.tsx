import React, { useState, useEffect, useMemo, useCallback } from 'react';
import { Trash, Spinner, GitPullRequest, GitBranch, Code } from '@phosphor-icons/react';
import './generateWorkspace.css';
import { useTauri } from '../../hooks/useTauri';
import { useAppStore } from '../../store';
import { useAgents } from '../../hooks/useAgents';
import { useRepos } from '../../hooks/useRepos';
import type { ReviewSource, ViewType } from '../../types';
import { useGeneration } from '../../contexts/useGeneration';
import { DiffEditorPanel } from './DiffEditorPanel';
import { AgentConfigPanel } from './AgentConfigPanel';
import { VcsInputCard } from './VcsInputCard';
import { LocalGitInput } from './LocalGitInput';
import { ViewModeToggle } from './ViewModeToggle';
import { DiffStats } from './DiffStats';
import { countAdditions, countDeletions } from './DiffEditorPanel';
import { toAgentConfigSelections } from '../../lib/agent-config';
import { InlineError } from '../Common/InlineError';
import { findSourceRepository } from '../../lib/source-repository';

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
  const [isLoadingPr, setIsLoadingPr] = useState(false);
  const [isStarting, setIsStarting] = useState(false);

  const [validationError, setValidationError] = useState<string | null>(null);
  const [startError, setStartError] = useState<string | null>(null);

  const { fetchRemotePr, fetchLocalGitDiff } = useTauri();
  const { startGeneration } = useGeneration();
  const { data: agents = [], isLoading: isAgentsLoading, error: agentsError } = useAgents();
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
  const [sourceMode, setSourceMode] = useState<'remote' | 'local' | 'paste'>(
    pendingSource?.type === 'local_git' || (!pendingSource && selectedRepoId) ? 'local' : 'remote'
  );

  const [repoLinkCallout, setRepoLinkCallout] = useState<RepoLinkCallout | null>(null);

  const globalDiffText = useAppStore(state => state.diffText);

  useEffect(() => {
    if (globalDiffText) {
      setDiffText(globalDiffText);
    }
  }, [globalDiffText]);

  useEffect(() => {
    if (
      agents.length > 0 &&
      !agents.some(agent => agent.id === agentId && agent.available !== false)
    ) {
      const available = agents.find(agent => agent.available !== false);
      if (available) setAgentIdStore(available.id);
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

  const findMatchingRepo = useCallback(
    (source: Extract<ReviewSource, { type: 'github_pr' | 'gitlab_mr' }>) => {
      return findSourceRepository(source, repos) ?? null;
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
    if (isStarting || isLoadingPr) return;
    setValidationError(null);
    setStartError(null);

    const error = validateDiff(diffText);
    if (error) {
      setValidationError(error);
      return;
    }

    setDiffTextStore(diffText);
    setAgentIdStore(agentId);
    setIsStarting(true);
    try {
      const result = await startGeneration({
        diffText,
        agentId,
        repoId: selectedRepoId || undefined,
        source: pendingSource,
        agentConfig: toAgentConfigSelections(agentConfigPreferences[agentId] || []),
      });
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
    } catch (error) {
      setStartError(String(error));
    } finally {
      setIsStarting(false);
    }
  }, [
    isStarting,
    isLoadingPr,
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
    setStartError(null);
    setParsedDiff(null);
    setPendingSource(null);
    setPrRef('');
    setValidationError(null);
    setViewMode('raw');
    setRepoLinkCallout(null);
  }, [setDiffTextStore, setParsedDiff, setPendingSource, setPrRef, setViewMode]);

  const handleCompareRefs = async (base: string, head: string) => {
    if (!selectedRepoId) return false;
    setIsLoadingPr(true);
    setValidationError(null);
    try {
      const diff = await fetchLocalGitDiff(selectedRepoId, base, head);
      setDiffText(diff.diff_text);
      setParsedDiff(diff);
      setPendingSource(diff.source ?? null);
      setPrRef('');
      setRepoLinkCallout(null);
      setViewMode('diff');
      return true;
    } catch (error) {
      setValidationError(String(error));
      return false;
    } finally {
      setIsLoadingPr(false);
    }
  };

  const localGitInput = (
    <LocalGitInput
      repoName={repos.find(repo => repo.id === selectedRepoId)?.name}
      source={pendingSource}
      compact={!isComposerEmpty}
      isLoading={isLoadingPr}
      disabled={isStarting}
      error={validationError}
      onCompare={handleCompareRefs}
      onCancelEdit={() => setValidationError(null)}
      onLinkRepo={() => _onNavigate('repos')}
    />
  );

  const sourceTabs = (
    <div className="generate-source-tabs" role="group" aria-label="Review input source">
      {(['remote', 'local', 'paste'] as const).map(mode => (
        <button
          key={mode}
          type="button"
          aria-pressed={sourceMode === mode}
          disabled={isStarting || isLoadingPr}
          onClick={() => {
            if (mode !== sourceMode) {
              if (mode === 'paste') {
                setPendingSource(null);
                setPrRef('');
                setRepoLinkCallout(null);
                setViewMode('raw');
              } else handleClear();
            }
            setSourceMode(mode);
            setValidationError(null);
          }}
        >
          {mode === 'remote' ? (
            <GitPullRequest size={14} />
          ) : mode === 'local' ? (
            <GitBranch size={14} />
          ) : (
            <Code size={14} />
          )}
          {mode === 'remote' ? 'Pull request' : mode === 'local' ? 'Local Git' : 'Paste diff'}
        </button>
      ))}
    </div>
  );
  const repoCallout = (
    <>
      {' '}
      {repoLinkCallout && (
        <div className="border-border bg-bg-secondary text-text-secondary pointer-events-auto flex flex-wrap items-center justify-between gap-3 rounded-md border px-4 py-3 text-xs">
          <div className="min-w-0">
            <div className="text-text-primary font-medium">Add repository context</div>
            <div className="text-text-tertiary mt-1 leading-5 break-words">
              Link {repoLinkCallout.label} to review with repository context.
            </div>
          </div>
          <div className="flex flex-shrink-0 items-center gap-2">
            <button
              onClick={handleCloneAndLink}
              disabled={isStarting || isRepoLinking}
              className="settings-secondary-action"
            >
              {isRepoLinking ? <Spinner size={12} className="animate-spin" /> : null}
              <span>Clone &amp; Link</span>
            </button>
            <button
              onClick={handleLinkExisting}
              disabled={isStarting || isRepoLinking}
              className="settings-secondary-action"
            >
              Link Existing
            </button>
            <button
              onClick={() => setRepoLinkCallout(null)}
              disabled={isRepoLinking}
              className="text-text-tertiary hover:text-text-primary px-1 text-[0.625rem] font-semibold transition-colors disabled:opacity-60"
            >
              Dismiss
            </button>
          </div>
        </div>
      )}
    </>
  );
  const isSourceEmpty = isComposerEmpty && sourceMode !== 'paste';
  const hasLocalSource = sourceMode === 'local' && pendingSource?.type === 'local_git';

  return (
    <div className="generate-workspace bg-bg-primary flex h-full min-w-0 flex-col">
      <AgentConfigPanel
        agents={agents}
        repos={repos}
        selectedAgentId={agentId}
        selectedRepoId={selectedRepoId}
        onAgentSelect={setAgentIdStore}
        onRepoSelect={repoId => {
          if (pendingSource?.type === 'local_git' && repoId !== selectedRepoId) handleClear();
          setSelectedRepoId(repoId);
        }}
        isStarting={isStarting}
        isLoadingSource={isLoadingPr}
        onGenerate={handleGenerate}
        isDiffValid={isDiffValid && !isLoadingPr && !validationError}
      />
      {!isAgentsLoading && (agentsError || !agents.some(agent => agent.available !== false)) && (
        <div className="border-border border-b px-6 py-3">
          <InlineError
            title="No available review agent"
            message={
              agentsError
                ? String(agentsError)
                : 'Install an agent or configure its command in Settings.'
            }
          />
          <button
            className="text-text-secondary mt-2 rounded py-1 text-xs underline underline-offset-4"
            onClick={() => _onNavigate('settings')}
          >
            Open settings
          </button>
        </div>
      )}
      <div className="relative flex min-h-0 flex-1 flex-col">
        {(startError || (validationError && !isSourceEmpty && sourceMode !== 'local')) && (
          <div className="px-5 pt-3">
            <InlineError
              title="Could not prepare review"
              message={startError ?? validationError ?? ''}
            />
          </div>
        )}
        {isSourceEmpty ? (
          <div className="generate-empty pointer-events-none absolute inset-0 z-10 px-8">
            <div className="pointer-events-auto mx-auto w-full max-w-[600px]">
              <div className="mb-7">
                <h1 className="text-text-primary text-2xl font-semibold tracking-[-0.025em]">
                  Start a review
                </h1>
                <p className="text-text-tertiary mt-2 text-[0.8125rem] leading-6">
                  Review a pull request or compare changes in a local repository.
                </p>
              </div>
              <section className="generate-source-panel">
                {sourceTabs}
                <div className="p-5">
                  {sourceMode === 'local' ? (
                    localGitInput
                  ) : (
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
                  )}
                  {sourceMode === 'remote' && validationError && (
                    <p role="alert" className="mt-3 text-xs leading-5 text-red-400">
                      {validationError}
                    </p>
                  )}
                </div>
              </section>
            </div>
          </div>
        ) : (
          <div className="generate-loaded-source border-border/50 space-y-3 border-b px-5 py-3">
            <div className="flex flex-wrap items-center gap-3">
              {hasLocalSource ? <div className="min-w-0 flex-1">{localGitInput}</div> : sourceTabs}
              {sourceMode === 'remote' && (
                <VcsInputCard
                  pendingSource={pendingSource}
                  prRef={prRef}
                  onPrRefChange={setPrRef}
                  onFetch={handleFetchPr}
                  isLoading={isLoadingPr}
                  disabled={isStarting}
                  onClear={handleClear}
                />
              )}
              {!hasLocalSource && <div className="flex-1" />}
              {(diffText.trim() || prRef.trim()) && (
                <button
                  type="button"
                  onClick={handleClear}
                  aria-label="Clear diff"
                  disabled={isStarting || isLoadingPr}
                  className="generate-clear flex h-8 items-center gap-1.5 rounded-md px-2.5 text-[0.6875rem]"
                >
                  <Trash size={13} />
                  <span>Clear</span>
                </button>
              )}
              <ViewModeToggle mode={viewMode} onChange={setViewMode} disabled={!diffText.trim()} />
            </div>
            {sourceMode === 'local' && !hasLocalSource && (
              <div className="max-w-[600px]">{localGitInput}</div>
            )}
            {repoCallout}
          </div>
        )}
        <div
          className={`generate-editor-shell flex min-h-0 flex-1 flex-col ${isSourceEmpty ? 'invisible' : ''}`}
        >
          <DiffEditorPanel
            diffText={diffText}
            viewMode={viewMode}
            onDiffTextChange={value => {
              setDiffText(value);
              setDiffTextStore(value);
              setPendingSource(null);
              setParsedDiff(null);
              setPrRef('');
              setRepoLinkCallout(null);
              setSourceMode('paste');
              setValidationError(null);
            }}
            validationError={
              diffValidationError || (sourceMode === 'remote' ? validationError : null)
            }
          />
        </div>
        {!isSourceEmpty && (
          <DiffStats
            charCount={diffText.length}
            additions={countAdditions(diffText)}
            deletions={countDeletions(diffText)}
          />
        )}
      </div>
    </div>
  );
};
