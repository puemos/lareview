import { create } from 'zustand';
import { persist, devtools } from 'zustand/middleware';
import {
  ReviewSource,
  Feedback,
  DiffFile,
  CommentThread,
  ReviewTask,
  DiffComment,
  ParsedDiff,
  AgentConfigPreference,
} from '../types';
import { PERSIST_CONFIG, STORAGE_KEYS } from '../constants/query-config';

interface AppStore {
  diffText: string;
  parsedDiff: ParsedDiff | null;
  selectedFile: DiffFile | null;
  commentThreads: Record<number, CommentThread[]>;
  tasks: ReviewTask[];
  selectedTaskId: string | null;
  feedbacks: Feedback[];
  selectedFeedbackId: string | null;
  agentId: string;
  agentConfigPreferences: Record<string, AgentConfigPreference[]>;
  reviewId: string | null;
  pendingSource: ReviewSource | null;
  selectedRepoId: string;
  prRef: string;
  viewMode: 'raw' | 'diff';
  reviewViewMode: 'summary' | 'review';

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
  setAgentId: (agentId: string) => void;
  setAgentConfigPreference: (
    agentId: string,
    preference: AgentConfigPreference,
    clearCategories?: string[]
  ) => void;
  setAgentConfigPreferences: (agentId: string, preferences: AgentConfigPreference[]) => void;
  setReviewId: (id: string | null) => void;
  setPendingSource: (source: ReviewSource | null) => void;
  setSelectedRepoId: (repoId: string) => void;
  setPrRef: (prRef: string) => void;
  setViewMode: (mode: 'raw' | 'diff') => void;
  setReviewViewMode: (mode: 'summary' | 'review') => void;
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
        agentId: 'default',
        agentConfigPreferences: {},
        reviewId: null,
        pendingSource: null,
        selectedRepoId: '',
        prRef: '',
        viewMode: 'raw',
        reviewViewMode: 'summary',

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
        setAgentId: agentId => set({ agentId }),
        setAgentConfigPreference: (agentId, preference, clearCategories = []) =>
          set(state => {
            const current = state.agentConfigPreferences[agentId] || [];
            const next = current.filter(
              item =>
                item.configId !== preference.configId &&
                (!item.category || !clearCategories.includes(item.category))
            );
            next.push(preference);
            return {
              agentConfigPreferences: {
                ...state.agentConfigPreferences,
                [agentId]: next,
              },
            };
          }),
        setAgentConfigPreferences: (agentId, preferences) =>
          set(state => ({
            agentConfigPreferences: {
              ...state.agentConfigPreferences,
              [agentId]: preferences,
            },
          })),

        setReviewId: id => set({ reviewId: id }),

        setPendingSource: source => set({ pendingSource: source }),

        setSelectedRepoId: repoId => set({ selectedRepoId: repoId }),
        setPrRef: prRef => set({ prRef }),
        setViewMode: mode => set({ viewMode: mode }),
        setReviewViewMode: mode => set({ reviewViewMode: mode }),
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
            reviewId: null,
            pendingSource: null,
            selectedRepoId: '',
            prRef: '',
            viewMode: 'raw',
            reviewViewMode: 'summary',
          }),
      }),
      {
        name: PERSIST_CONFIG.name,
        partialize: state => ({
          [STORAGE_KEYS.agentId]: state.agentId,
          [STORAGE_KEYS.agentConfigPreferences]: state.agentConfigPreferences,
        }),
      }
    )
  )
);
