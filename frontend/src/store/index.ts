import { create } from 'zustand';
import { persist, devtools } from 'zustand/middleware';
import {
  Plan,
  ReviewSource,
  Feedback,
  DiffFile,
  CommentThread,
  ReviewTask,
  DiffComment,
  ParsedDiff,
} from '../types';
import { PERSIST_CONFIG, STORAGE_KEYS } from '../constants/query-config';
import type { AvailableCommand, SessionUpdate } from '../hooks/useTauri';
import {
  isAgentMessageChunk,
  isAgentThoughtChunk,
  isToolCall,
  isToolCallUpdate,
  isPlan,
  isAvailableCommandsUpdate,
} from '../hooks/useTauri';

interface ProgressMessage {
  type: string;
  message: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  data?: any;
  timestamp: number;
  id?: string;
}

interface AppStore {
  diffText: string;
  parsedDiff: ParsedDiff | null;
  selectedFile: DiffFile | null;
  commentThreads: Record<number, CommentThread[]>;
  tasks: ReviewTask[];
  selectedTaskId: string | null;
  feedbacks: Feedback[];
  selectedFeedbackId: string | null;
  isGenerating: boolean;
  agentId: string;
  reviewId: string | null;
  runId: string | null;
  progressMessages: ProgressMessage[];
  plan: Plan | null;
  pendingSource: ReviewSource | null;
  selectedRepoId: string;
  prRef: string;
  viewMode: 'raw' | 'diff';
  planItems: string[];
  isPlanExpanded: boolean;

  setDiffText: (text: string) => void;
  setParsedDiff: (diff: ParsedDiff | null) => void;
  selectFile: (file: DiffFile | null) => void;
  addCommentThread: (lineNumber: number, thread: CommentThread) => void;
  addComment: (lineNumber: number, threadId: string, comment: DiffComment) => void;
  removeCommentThread: (lineNumber: number, threadId: string) => void;
  setTasks: (tasks: ReviewTask[]) => void;
  selectTask: (taskId: string | null) => void;
  setFeedbacks: (feedbacks: Feedback[]) => void;
  selectFeedback: (feedbackId: string | null) => void;
  setIsGenerating: (isGenerating: boolean) => void;
  setAgentId: (agentId: string) => void;
  setReviewId: (id: string | null) => void;
  setRunId: (id: string | null) => void;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  addProgressMessage: (type: string, message: string, data?: any) => void;
  updateLastProgressMessage: (updates: Partial<ProgressMessage>) => void;
  handleServerUpdate: (update: SessionUpdate | Plan) => void;
  clearProgressMessages: () => void;
  setPendingSource: (source: ReviewSource | null) => void;
  setSelectedRepoId: (repoId: string) => void;
  setPrRef: (prRef: string) => void;
  setViewMode: (mode: 'raw' | 'diff') => void;
  setIsPlanExpanded: (isExpanded: boolean) => void;
  reset: () => void;
}

export const useAppStore = create<AppStore>()(
  devtools(
    persist(
      (set, get) => ({
        diffText: '',
        parsedDiff: null,
        selectedFile: null,
        commentThreads: {},
        tasks: [],
        selectedTaskId: null,
        feedbacks: [],
        selectedFeedbackId: null,
        isGenerating: false,
        agentId: 'default',
        reviewId: null,
        runId: null,
        progressMessages: [],
        plan: null,
        pendingSource: null,
        selectedRepoId: '',
        prRef: '',
        viewMode: 'raw',
        planItems: [],
        isPlanExpanded: false,

        setDiffText: text => set({ diffText: text }),
        setParsedDiff: diff => set({ parsedDiff: diff }),
        selectFile: file => set({ selectedFile: file }),

        addCommentThread: (lineNumber, thread) => {
          const threads = get().commentThreads;
          const existing = threads[lineNumber] || [];
          set({
            commentThreads: { ...threads, [lineNumber]: [...existing, thread] },
          });
        },

        addComment: (lineNumber, threadId, comment) => {
          const threads = get().commentThreads;
          const existing = threads[lineNumber] || [];
          set({
            commentThreads: {
              ...threads,
              [lineNumber]: existing.map(t =>
                t.id === threadId ? { ...t, comments: [...t.comments, comment] } : t
              ),
            },
          });
        },

        removeCommentThread: (lineNumber, threadId) => {
          const threads = get().commentThreads;
          const existing = threads[lineNumber] || [];
          const filtered = existing.filter(t => t.id !== threadId);
          if (filtered.length === 0) {
            const { [lineNumber]: _, ...rest } = threads;
            set({ commentThreads: rest });
          } else {
            set({ commentThreads: { ...threads, [lineNumber]: filtered } });
          }
        },

        setTasks: tasks => set({ tasks }),
        selectTask: taskId => set({ selectedTaskId: taskId, selectedFeedbackId: null }),
        setFeedbacks: feedbacks => set({ feedbacks }),
        selectFeedback: feedbackId => set({ selectedFeedbackId: feedbackId, selectedTaskId: null }),
        setIsGenerating: isGenerating => set({ isGenerating }),
        setAgentId: agentId => set({ agentId }),

        setReviewId: id => set({ reviewId: id }),
        setRunId: id => set({ runId: id }),

        addProgressMessage: (type, message, data) => {
          set(state => ({
            progressMessages: [
              ...state.progressMessages,
              {
                type,
                message,
                data,
                timestamp: Date.now(),
              },
            ],
          }));
        },

        updateLastProgressMessage: updates => {
          set(state => {
            const msgs = [...state.progressMessages];
            if (msgs.length > 0) {
              const last = msgs[msgs.length - 1];
              msgs[msgs.length - 1] = { ...last, ...updates };
            }
            return { progressMessages: msgs };
          });
        },

        handleServerUpdate: (update: SessionUpdate | Plan) => {
          set(state => {
            const msgs = [...state.progressMessages];
            const lastMsg = msgs.length > 0 ? msgs[msgs.length - 1] : null;

            if ('entries' in update) {
              return {
                plan: update, // ACP: Client MUST replace the current plan completely
                progressMessages: [
                  ...msgs,
                  {
                    type: 'agent_plan',
                    message: 'Agent Plan Update',
                    data: update,
                    timestamp: Date.now(),
                  },
                ],
              };
            }

            const sessionUpdate = update as SessionUpdate;

            if (isAgentMessageChunk(sessionUpdate)) {
              const text = sessionUpdate.content?.text || '';
              if (lastMsg && lastMsg.type === 'agent_message') {
                msgs[msgs.length - 1] = {
                  ...lastMsg,
                  message: lastMsg.message + text,
                };
                return { progressMessages: msgs };
              } else {
                return {
                  progressMessages: [
                    ...msgs,
                    {
                      type: 'agent_message',
                      message: text,
                      data: sessionUpdate,
                      timestamp: Date.now(),
                    },
                  ],
                };
              }
            } else if (isAgentThoughtChunk(sessionUpdate)) {
              const text = sessionUpdate.content?.text || '';
              if (lastMsg && lastMsg.type === 'agent_thought') {
                msgs[msgs.length - 1] = {
                  ...lastMsg,
                  message: lastMsg.message + text,
                };
                return { progressMessages: msgs };
              } else {
                return {
                  progressMessages: [
                    ...msgs,
                    {
                      type: 'agent_thought',
                      message: text,
                      data: sessionUpdate,
                      timestamp: Date.now(),
                    },
                  ],
                };
              }
            } else if (isToolCall(sessionUpdate)) {
              return {
                progressMessages: [
                  ...msgs,
                  {
                    type: 'tool_call',
                    message: sessionUpdate.title || 'Tool Call',
                    data: sessionUpdate,
                    timestamp: Date.now(),
                  },
                ],
              };
            } else if (isToolCallUpdate(sessionUpdate)) {
              // Find the tool call and update it
              const toolCallId = sessionUpdate.toolCallId?.id;

              if (
                toolCallId &&
                lastMsg &&
                lastMsg.type === 'tool_call' &&
                lastMsg.data?.toolCallId?.id === toolCallId
              ) {
                msgs[msgs.length - 1] = {
                  ...lastMsg,
                  data: { ...lastMsg.data, ...sessionUpdate.fields },
                };
                return { progressMessages: msgs };
              }

              if (toolCallId) {
                for (let i = msgs.length - 1; i >= 0; i--) {
                  if (msgs[i].type === 'tool_call' && msgs[i].data?.toolCallId?.id === toolCallId) {
                    msgs[i] = {
                      ...msgs[i],
                      data: { ...msgs[i].data, ...sessionUpdate.fields },
                    };
                    return { progressMessages: msgs };
                  }
                }
              }

              return { progressMessages: msgs };
            } else if (isPlan(sessionUpdate)) {
              return {
                plan: sessionUpdate as unknown as Plan,
                progressMessages: [
                  ...msgs,
                  {
                    type: 'agent_plan',
                    message: 'Agent Plan Update',
                    data: sessionUpdate,
                    timestamp: Date.now(),
                  },
                ],
              };
            } else if (isAvailableCommandsUpdate(sessionUpdate)) {
              const commands =
                sessionUpdate.availableCommands?.map((c: AvailableCommand) => c.name).join(', ') ||
                '';
              return {
                progressMessages: [
                  ...msgs,
                  {
                    type: 'system',
                    message: `Available commands: ${commands}`,
                    data: sessionUpdate,
                    timestamp: Date.now(),
                  },
                ],
              };
            } else {
              return {
                progressMessages: [
                  ...msgs,
                  {
                    type: 'debug',
                    message: `Unknown: ${sessionUpdate.sessionUpdate}`,
                    data: sessionUpdate,
                    timestamp: Date.now(),
                  },
                ],
              };
            }
          });
        },

        clearProgressMessages: () => set({ progressMessages: [] }),

        setPendingSource: source => set({ pendingSource: source }),

        setSelectedRepoId: repoId => set({ selectedRepoId: repoId }),
        setPrRef: prRef => set({ prRef }),
        setViewMode: mode => set({ viewMode: mode }),
        setIsPlanExpanded: isExpanded => set({ isPlanExpanded: isExpanded }),

        reset: () =>
          set({
            diffText: '',
            parsedDiff: null,
            selectedFile: null,
            commentThreads: {},
            tasks: [],
            selectedTaskId: null,
            feedbacks: [],
            selectedFeedbackId: null,
            isGenerating: false,
            reviewId: null,
            runId: null,
            progressMessages: [],
            plan: null,
            pendingSource: null,
            selectedRepoId: '',
            prRef: '',
            viewMode: 'raw',
            planItems: [],
            isPlanExpanded: false,
          }),
      }),
      {
        name: PERSIST_CONFIG.name,
        partialize: state => ({
          [STORAGE_KEYS.agentId]: state.agentId,
        }),
      }
    )
  )
);
