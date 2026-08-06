import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { toast } from 'sonner';
import { useQueryClient } from '@tanstack/react-query';
import { useTauri } from '../hooks/useTauri';
import { useAppStore } from '../store';
import type { ReviewRunEvent } from '../types';
import {
  GenerationContext,
  type GenerationContextValue,
  type StartGenerationArgs,
} from './generation-context';
import { ConfirmationModal } from '../components/Common/ConfirmationModal';
import { queryKeys } from '../lib/query-keys';

interface WorktreeRequest {
  repoId: string;
  repoName: string;
  commitSha: string;
  resolve: (confirmed: boolean) => void;
}

const isTerminalPayload = (event: ReviewRunEvent) =>
  event.payload.event === 'Completed' || event.payload.event === 'Error';

export const GenerationProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const { generateReview, parseDiff, stop_generation, getLinkedRepos, setRepoSnapshotAccess } =
    useTauri();
  const queryClient = useQueryClient();
  const [worktreeRequest, setWorktreeRequest] = useState<WorktreeRequest | null>(null);
  const [worktreeRemember, setWorktreeRemember] = useState(false);

  const setDiffTextStore = useAppStore(state => state.setDiffText);
  const setAgentIdStore = useAppStore(state => state.setAgentId);
  const setParsedDiff = useAppStore(state => state.setParsedDiff);

  useEffect(() => {
    const unlisten = listen<ReviewRunEvent>('review-run-event', ({ payload: event }) => {
      if (
        event.payload.event === 'Status' ||
        event.payload.event === 'Completed' ||
        event.payload.event === 'Error'
      ) {
        queryClient.invalidateQueries({ queryKey: queryKeys.reviews });
        queryClient.invalidateQueries({ queryKey: queryKeys.reviewRuns(event.review_id) });
      }

      if (event.payload.event === 'TaskCompleted' || event.payload.event === 'Completed') {
        queryClient.invalidateQueries({ queryKey: queryKeys.tasks(event.run_id) });
      }

      if (event.payload.event === 'Completed') {
        queryClient.invalidateQueries({ queryKey: queryKeys.feedbackByReview(event.review_id) });
        queryClient.invalidateQueries({ queryKey: queryKeys.issueChecks(event.run_id) });
        queryClient.invalidateQueries({ queryKey: queryKeys.mergeConfidence(event.run_id) });
      }

      if (!isTerminalPayload(event)) return;
      if (event.payload.event === 'Completed') {
        toast.success('Review ready', {
          description: 'Generation finished in the background.',
        });
      } else {
        const data = event.payload.data as { message?: string };
        toast.error('Review generation stopped', {
          description: data.message || 'The review did not finish.',
        });
      }
    });

    return () => {
      unlisten.then(dispose => dispose()).catch(console.error);
    };
  }, [queryClient]);

  const startGeneration = useCallback(
    async ({
      diffText,
      agentId,
      repoId,
      source,
      agentConfig,
    }: StartGenerationArgs): ReturnType<GenerationContextValue['startGeneration']> => {
      setDiffTextStore(diffText);
      setAgentIdStore(agentId);

      try {
        const diff = await parseDiff(diffText);
        setParsedDiff(diff);

        let useSnapshot = false;
        if (
          repoId &&
          source &&
          'head_sha' in source &&
          source.head_sha &&
          (source.type === 'github_pr' || source.type === 'gitlab_mr')
        ) {
          try {
            const linkedRepos = await getLinkedRepos();
            const matchingRepo = linkedRepos.find(repo => repo.id === repoId);
            if (matchingRepo?.allow_snapshot_access) {
              useSnapshot = true;
            } else if (matchingRepo) {
              setWorktreeRemember(false);
              useSnapshot = await new Promise<boolean>(resolve => {
                setWorktreeRequest({
                  repoId,
                  repoName: matchingRepo.name,
                  commitSha: source.head_sha!,
                  resolve,
                });
              });
            }
          } catch (error) {
            console.warn('Failed to check snapshot eligibility:', error);
          }
        }

        const result = await generateReview(
          diffText,
          agentId,
          repoId,
          source || undefined,
          useSnapshot,
          agentConfig
        );
        queryClient.invalidateQueries({ queryKey: queryKeys.reviews });
        return result;
      } catch (error) {
        console.error('Failed to start review:', error);
        toast.error('Failed to start review', {
          description: error instanceof Error ? error.message : String(error),
        });
        return null;
      }
    }, [
      generateReview,
      getLinkedRepos,
      parseDiff,
      queryClient,
      setAgentIdStore,
      setDiffTextStore,
      setParsedDiff,
    ]
  );

  const stopGeneration = useCallback(
    async (runId: string): Promise<void> => {
      try {
        await stop_generation(runId);
        toast('Stopping review…', {
          description: 'The agent is cleaning up its current work.',
        });
      } catch (error) {
        toast.error('Failed to stop review', {
          description: error instanceof Error ? error.message : String(error),
        });
      }
    },
    [stop_generation]
  );

  const value = useMemo<GenerationContextValue>(
    () => ({ startGeneration, stopGeneration }),
    [startGeneration, stopGeneration]
  );

  const handleWorktreeConfirm = useCallback(async () => {
    if (!worktreeRequest) return;
    if (worktreeRemember) {
      try {
        await setRepoSnapshotAccess(worktreeRequest.repoId, true);
        queryClient.invalidateQueries({ queryKey: queryKeys.repos });
        toast.success('Snapshot preference saved');
      } catch (error) {
        console.error('Failed to save snapshot preference:', error);
        toast.error('Failed to save preference');
      }
    }
    worktreeRequest.resolve(true);
    setWorktreeRequest(null);
  }, [queryClient, setRepoSnapshotAccess, worktreeRemember, worktreeRequest]);

  const handleWorktreeCancel = useCallback(() => {
    worktreeRequest?.resolve(false);
    setWorktreeRequest(null);
  }, [worktreeRequest]);

  return (
    <GenerationContext.Provider value={value}>
      {children}
      <ConfirmationModal
        isOpen={!!worktreeRequest}
        onClose={handleWorktreeCancel}
        onConfirm={handleWorktreeConfirm}
        title="Enable Code Access"
        message={
          worktreeRequest
            ? `Create a temporary snapshot of "${worktreeRequest.repoName}" at commit ${worktreeRequest.commitSha.slice(0, 7)}? This lets the agent read the PR/MR source code for better analysis.`
            : ''
        }
        confirmLabel="Create Snapshot"
        confirmVariant="brand"
      >
        <div className="flex items-center gap-2">
          <input
            type="checkbox"
            id="worktreeRemember"
            checked={worktreeRemember}
            onChange={event => setWorktreeRemember(event.target.checked)}
            className="border-border text-brand focus:ring-brand h-4 w-4 rounded"
          />
          <label htmlFor="worktreeRemember" className="text-text-secondary text-sm select-none">
            Always allow snapshots for this repository
          </label>
        </div>
      </ConfirmationModal>
    </GenerationContext.Provider>
  );
};
