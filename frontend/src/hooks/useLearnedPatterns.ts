import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useTauri } from './useTauri';
import { queryKeys } from '../lib/query-keys';
import type { LearnedPatternInput } from '../types';

export function useLearnedPatterns() {
  const queryClient = useQueryClient();
  const {
    getLearnedPatterns,
    createLearnedPattern,
    updateLearnedPattern,
    deleteLearnedPattern,
    toggleLearnedPattern,
    getLearningStatus,
    triggerLearningCompaction,
  } = useTauri();

  const patterns = useQuery({
    queryKey: queryKeys.learnedPatterns,
    queryFn: getLearnedPatterns,
  });

  const status = useQuery({
    queryKey: queryKeys.learningStatus,
    queryFn: getLearningStatus,
  });

  const create = useMutation({
    mutationFn: createLearnedPattern,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.learnedPatterns });
      queryClient.invalidateQueries({ queryKey: queryKeys.learningStatus });
    },
  });

  const update = useMutation({
    mutationFn: ({ id, input }: { id: string; input: LearnedPatternInput }) =>
      updateLearnedPattern(id, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.learnedPatterns });
    },
  });

  const remove = useMutation({
    mutationFn: deleteLearnedPattern,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.learnedPatterns });
      queryClient.invalidateQueries({ queryKey: queryKeys.learningStatus });
    },
  });

  const toggle = useMutation({
    mutationFn: ({ id, enabled }: { id: string; enabled: boolean }) =>
      toggleLearnedPattern(id, enabled),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.learnedPatterns });
      queryClient.invalidateQueries({ queryKey: queryKeys.learningStatus });
    },
  });

  const compact = useMutation({
    mutationFn: (agentId: string) => triggerLearningCompaction(agentId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.learnedPatterns });
      queryClient.invalidateQueries({ queryKey: queryKeys.learningStatus });
    },
  });

  return {
    patterns,
    status,
    create,
    update,
    remove,
    toggle,
    compact,
  };
}
