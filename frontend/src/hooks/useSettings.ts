import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useTauri } from './useTauri';
import { queryKeys } from '../lib/query-keys';

export interface FeedbackFilterConfig {
  confidenceThreshold: number | null;
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
