import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { useTauri } from './useTauri';
import { queryKeys } from '../lib/query-keys';
import type { LibraryCategory, RuleScope } from '../types';

export function useRuleLibrary() {
  const {
    getRuleLibrary,
    getRuleLibraryByCategory,
    getRuleLibraryChecklists,
    getRuleLibraryGuidelines,
    addRuleFromLibrary,
    getDefaultIssueCategories,
  } = useTauri();
  const queryClient = useQueryClient();

  const allRules = useQuery({
    queryKey: queryKeys.ruleLibrary,
    queryFn: getRuleLibrary,
  });

  const defaultCategories = useQuery({
    queryKey: queryKeys.defaultIssueCategories,
    queryFn: getDefaultIssueCategories,
  });

  const addFromLibrary = useMutation({
    mutationFn: ({
      libraryRuleId,
      scope,
      repoId,
    }: {
      libraryRuleId: string;
      scope: RuleScope;
      repoId?: string;
    }) => addRuleFromLibrary(libraryRuleId, scope, repoId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.rules });
      toast('Rule Added', { description: 'The rule from the library has been added.' });
    },
    onError: (error: Error) => {
      toast('Failed to add rule', { description: error.message });
    },
  });

  const getRulesByCategory = async (category: LibraryCategory) => {
    return getRuleLibraryByCategory(category);
  };

  return {
    allRules,
    defaultCategories,
    addFromLibrary,
    getRulesByCategory,
    getChecklists: getRuleLibraryChecklists,
    getGuidelines: getRuleLibraryGuidelines,
  };
}
