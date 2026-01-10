import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { useTauri } from './useTauri';
import { queryKeys } from '../lib/query-keys';
import type { RuleScope } from '../types';

export interface ReviewRuleInput {
  scope: RuleScope;
  repo_id?: string | null;
  glob?: string | null;
  text: string;
  enabled: boolean;
}

export function useRules() {
  const { getReviewRules, createReviewRule, updateReviewRule, deleteReviewRule } = useTauri();
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: queryKeys.rules,
    queryFn: getReviewRules,
  });

  const createRule = useMutation({
    mutationFn: (input: ReviewRuleInput) => createReviewRule(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.rules });
      toast('Rule Saved', { description: 'The review rule is now active.' });
    },
    onError: (error: Error) => {
      toast('Failed to save rule', { description: error.message });
    },
  });

  const updateRule = useMutation({
    mutationFn: (payload: { id: string; input: ReviewRuleInput }) =>
      updateReviewRule(payload.id, payload.input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.rules });
    },
    onError: (error: Error) => {
      toast('Failed to update rule', { description: error.message });
    },
  });

  const removeRule = useMutation({
    mutationFn: (id: string) => deleteReviewRule(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.rules });
      toast('Rule Removed', { description: 'The review rule has been deleted.' });
    },
    onError: (error: Error) => {
      toast('Failed to remove rule', { description: error.message });
    },
  });

  return {
    ...query,
    createRule,
    updateRule,
    removeRule,
  };
}
