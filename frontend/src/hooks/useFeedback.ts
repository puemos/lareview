import { useQuery, useMutation, useQueryClient, keepPreviousData } from '@tanstack/react-query';
import { useTauri } from '../hooks/useTauri';
import { queryKeys } from '../lib/query-keys';
import type { Feedback, Comment } from '../types';
import { QUERY_CONFIG } from '../constants/query-config';

export interface UseFeedbackResult {
  data: Feedback[];
  isLoading: boolean;
  isFetching: boolean;
  isPending: boolean;
  error: Error | null;
  createFeedback: (vars: CreateFeedbackInput) => void;
  isCreating: boolean;
  updateStatus: (vars: { feedbackId: string; status: Feedback['status'] }) => void;
  isUpdatingStatus: boolean;
  updateImpact: (vars: { feedbackId: string; impact: Feedback['impact'] }) => void;
  isUpdatingImpact: boolean;
  deleteFeedback: (vars: { feedbackId: string }) => void;
  isDeleting: boolean;
}

export interface CreateFeedbackInput {
  review_id: string;
  task_id: string;
  title: string;
  file_path: string;
  line_number: number;
  side: string;
  content: string;
  impact: Feedback['impact'];
}

export function useFeedback(reviewId: string | null): UseFeedbackResult {
  const {
    saveFeedback,
    updateFeedbackStatus,
    updateFeedbackImpact,
    deleteFeedback,
    getFeedbackByReview,
  } = useTauri();
  const queryClient = useQueryClient();

  const queryKey = reviewId ? queryKeys.feedbackByReview(reviewId) : [];

  const { data, isLoading, isFetching, isPending, error } = useQuery({
    queryKey,
    queryFn: async () => {
      if (!reviewId) return [];
      const result = await getFeedbackByReview(reviewId);
      return result.map(f => ({
        ...f,
        status: f.status as Feedback['status'],
        impact: f.impact as Feedback['impact'],
        anchor: f.anchor
          ? {
              ...f.anchor,
              side: f.anchor.side as 'old' | 'new' | null,
            }
          : null,
      }));
    },
    enabled: !!reviewId,
    staleTime: QUERY_CONFIG.feedbackStaleTime,
    placeholderData: keepPreviousData,
  });

  const createMutation = useMutation({
    mutationFn: (input: CreateFeedbackInput) => saveFeedback(input),
    onSuccess: () => {
      if (reviewId) {
        queryClient.invalidateQueries({
          queryKey: queryKeys.feedbackByReview(reviewId),
        });
      }
    },
  });

  const statusMutation = useMutation({
    mutationFn: ({ feedbackId, status }: { feedbackId: string; status: Feedback['status'] }) =>
      updateFeedbackStatus(feedbackId, status),
    onSuccess: () => {
      if (reviewId) {
        queryClient.invalidateQueries({
          queryKey: queryKeys.feedbackByReview(reviewId),
        });
      }
    },
  });

  const impactMutation = useMutation({
    mutationFn: ({ feedbackId, impact }: { feedbackId: string; impact: Feedback['impact'] }) =>
      updateFeedbackImpact(feedbackId, impact),
    onSuccess: () => {
      if (reviewId) {
        queryClient.invalidateQueries({
          queryKey: queryKeys.feedbackByReview(reviewId),
        });
      }
    },
  });

  const deleteMutation = useMutation({
    mutationFn: ({ feedbackId }: { feedbackId: string }) => deleteFeedback(feedbackId),
    onSuccess: () => {
      if (reviewId) {
        queryClient.invalidateQueries({
          queryKey: queryKeys.feedbackByReview(reviewId),
        });
      }
    },
  });

  return {
    data: data || [],
    isLoading,
    isFetching,
    isPending,
    error: error instanceof Error ? error : error ? new Error(String(error)) : null,
    createFeedback: createMutation.mutate,
    isCreating: createMutation.isPending,
    updateStatus: statusMutation.mutate,
    isUpdatingStatus: statusMutation.isPending,
    updateImpact: impactMutation.mutate,
    isUpdatingImpact: impactMutation.isPending,
    deleteFeedback: deleteMutation.mutate,
    isDeleting: deleteMutation.isPending,
  };
}

export function useFeedbackComments(feedbackId: string | null) {
  const { getFeedbackComments } = useTauri();

  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ['feedback-comments', feedbackId],
    queryFn: () => (feedbackId ? getFeedbackComments(feedbackId) : Promise.resolve<Comment[]>([])),
    enabled: !!feedbackId,
  });

  return { comments: data || [], isLoading, error, refetch };
}

export function useAddComment() {
  const { addComment } = useTauri();
  const queryClient = useQueryClient();

  const mutation = useMutation({
    mutationFn: ({ feedbackId, body }: { feedbackId: string; body: string }) =>
      addComment(feedbackId, body),
    onSuccess: (_result, { feedbackId }) => {
      queryClient.invalidateQueries({
        queryKey: ['feedback-comments', feedbackId],
      });
    },
  });

  return mutation;
}
