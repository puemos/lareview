import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useTauri } from './useTauri';
import { queryKeys } from '../lib/query-keys';

export interface FeedbackFilterConfig {
  confidenceThreshold: number | null;
}

export interface TimeoutConfig {
  timeoutSecs: number | null;
}

export function useTimeoutConfig() {
  const { getTimeoutConfig, updateTimeoutConfig } = useTauri();
  const queryClient = useQueryClient();

  const { data, isLoading } = useQuery({
    queryKey: queryKeys.timeoutConfig,
    queryFn: async () => {
      const config = await getTimeoutConfig();
      return {
        timeoutSecs: config.timeout_secs,
      };
    },
    staleTime: Infinity,
  });

  const updateMutation = useMutation({
    mutationFn: (timeoutSecs: number | null) => updateTimeoutConfig(timeoutSecs),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.timeoutConfig });
    },
  });

  return {
    config: data ?? { timeoutSecs: null },
    isLoading,
    updateTimeout: updateMutation.mutate,
    isUpdating: updateMutation.isPending,
  };
}

export function useFeedbackFilterConfig() {
  const { getFeedbackFilterConfig, updateFeedbackFilterConfig } = useTauri();
  const queryClient = useQueryClient();

  const { data, isLoading } = useQuery({
    queryKey: queryKeys.feedbackFilterConfig,
    queryFn: async () => {
      const config = await getFeedbackFilterConfig();
      return {
        confidenceThreshold: config.confidence_threshold,
      };
    },
    staleTime: Infinity, // Config rarely changes
  });

  const updateMutation = useMutation({
    mutationFn: (threshold: number | null) => updateFeedbackFilterConfig(threshold),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.feedbackFilterConfig });
    },
  });

  return {
    config: data ?? { confidenceThreshold: null },
    isLoading,
    updateThreshold: updateMutation.mutate,
    isUpdating: updateMutation.isPending,
  };
}
