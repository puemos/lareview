import React, { useCallback, useMemo, useRef, useState } from 'react';
import { Channel } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { useQueryClient } from '@tanstack/react-query';
import type { ProgressEventPayload } from '../hooks/useTauri';
import { useTauri } from '../hooks/useTauri';
import { useAppStore } from '../store';

import {
  GenerationContext,
  type GenerationContextValue,
  type StartGenerationArgs,
} from './generation-context';
import { ConfirmationModal } from '../components/Common/ConfirmationModal';

interface WorktreeRequest {
  repoId: string;
  repoName: string;
  commitSha: string;
  resolve: (confirmed: boolean) => void;
}

export const GenerationProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const { generateReview, parseDiff, stop_generation, getLinkedRepos, setRepoSnapshotAccess } =
    useTauri();
  const queryClient = useQueryClient();

  const [worktreeRequest, setWorktreeRequest] = useState<WorktreeRequest | null>(null);
  const [worktreeRemember, setWorktreeRemember] = useState(false);

  const setDiffTextStore = useAppStore(state => state.setDiffText);
  const setAgentIdStore = useAppStore(state => state.setAgentId);
  const setParsedDiff = useAppStore(state => state.setParsedDiff);
  const setIsGeneratingStore = useAppStore(state => state.setIsGenerating);
  const handleServerUpdate = useAppStore(state => state.handleServerUpdate);
  const addProgressMessage = useAppStore(state => state.addProgressMessage);
  const clearProgressMessages = useAppStore(state => state.clearProgressMessages);
  const setReviewId = useAppStore(state => state.setReviewId);
  const setRunId = useAppStore(state => state.setRunId);
  const runId = useAppStore(state => state.runId);
  const setTasks = useAppStore(state => state.setTasks);

  const isGeneratingRef = useRef(false);
  const currentTaskTitleRef = useRef<string | null>(null);

  const startGeneration = useCallback(
    async ({ diffText, agentId, repoId, source }: StartGenerationArgs): Promise<boolean> => {
      if (isGeneratingRef.current) return false;

      isGeneratingRef.current = true;
      setIsGeneratingStore(true);
      clearProgressMessages();

      setDiffTextStore(diffText);
      setAgentIdStore(agentId);

      try {
        const diff = await parseDiff(diffText);
        setParsedDiff(diff);

        const onProgress = new Channel<ProgressEventPayload>();
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
              break;
            }
            case 'TaskCompleted':
              addProgressMessage('task_added', 'Task completed');
              break;
            case 'Completed':
              addProgressMessage('completed', 'Review generation complete!');
              queryClient.invalidateQueries({ queryKey: ['reviews'] });
              isGeneratingRef.current = false;
              setIsGeneratingStore(false);
              break;
            case 'Error':
              addProgressMessage('error', (payload.data as { message: string }).message);
              isGeneratingRef.current = false;
              setIsGeneratingStore(false);
              break;
            default:
              console.warn('[Progress] Unknown event type:', payload.event);
          }
        };

        // Check if we should create a worktree for GitHub/GitLab PRs
        let useSnapshot = false;

        if (
          repoId &&
          source &&
          'head_sha' in source &&
          (source.type === 'github_pr' || source.type === 'gitlab_mr')
        ) {
          // Check if we have a linked repo that matches
          try {
            const linkedRepos = await getLinkedRepos();
            const matchingRepo = linkedRepos.find(r => r.id === repoId);

            if (matchingRepo) {
              if (matchingRepo.allow_snapshot_access) {
                useSnapshot = true;
                addProgressMessage('log', 'Using existing snapshot preference: Allowed');
              } else {
                // Show modal and wait for user response
                setWorktreeRemember(false); // Reset default
                const shouldCreate = await new Promise<boolean>(resolve => {
                  setWorktreeRequest({
                    repoId,
                    repoName: matchingRepo.name,
                    commitSha: source.head_sha!,
                    resolve,
                  });
                });

                if (shouldCreate) {
                  useSnapshot = true;
                }
              }
            }
          } catch (err) {
            console.warn('Failed to check worktree eligibility:', err);
          }
        }

        const result = await generateReview(
          diffText,
          agentId,
          currentRunId,
          repoId,
          source || undefined,
          useSnapshot,
          onProgress
        );

        setReviewId(result.review_id);
        setTasks([]);
        queryClient.invalidateQueries({ queryKey: ['reviews'] });

        isGeneratingRef.current = false;
        setIsGeneratingStore(false);
        toast('Generation Complete', {
          description: 'Your review plan is ready.',
        });
        return true;
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
          toast('Generation failed', {
            description: String(error),
          });
        }

        isGeneratingRef.current = false;
        setIsGeneratingStore(false);
        return false;
      }
    },
    [
      addProgressMessage,
      clearProgressMessages,
      generateReview,
      getLinkedRepos,
      handleServerUpdate,
      parseDiff,
      queryClient,
      setAgentIdStore,
      setDiffTextStore,
      setIsGeneratingStore,
      setParsedDiff,
      setReviewId,
      setRunId,
      setTasks,
    ]
  );

  const stopGeneration = useCallback(async (): Promise<void> => {
    if (!runId) return;

    try {
      await stop_generation(runId);
      addProgressMessage('log', 'Stop signal sent...');
    } catch (error) {
      console.error('Failed to stop generation:', error);
      addProgressMessage('error', `Failed to stop: ${error}`);
    }
  }, [addProgressMessage, runId, stop_generation]);

  const value = useMemo<GenerationContextValue>(
    () => ({
      startGeneration,
      stopGeneration,
    }),
    [startGeneration, stopGeneration]
  );

  const handleWorktreeConfirm = useCallback(async () => {
    if (worktreeRequest) {
      if (worktreeRemember) {
        try {
          await setRepoSnapshotAccess(worktreeRequest.repoId, true);
          queryClient.invalidateQueries({ queryKey: ['repos'] });
          toast.success('Snapshot preference saved');
        } catch (err) {
          console.error('Failed to save snapshot preference:', err);
          toast.error('Failed to save preference');
        }
      }

      worktreeRequest.resolve(true);
      setWorktreeRequest(null);
    }
  }, [worktreeRequest, worktreeRemember, setRepoSnapshotAccess, queryClient]);

  const handleWorktreeCancel = useCallback(() => {
    if (worktreeRequest) {
      worktreeRequest.resolve(false);
      setWorktreeRequest(null);
    }
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
            onChange={e => setWorktreeRemember(e.target.checked)}
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
