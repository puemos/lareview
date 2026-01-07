import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useTauri } from '../hooks/useTauri';
import { queryKeys } from '../lib/query-keys';

interface Repo {
  id: string;
  name: string;
  path: string;
  review_count: number;
  linked_at: string;
}

export function useRepos() {
  const { getLinkedRepos, linkRepo, unlinkRepo, selectRepoFolder } = useTauri();
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: queryKeys.repos,
    queryFn: getLinkedRepos,
  });

  const addRepo = useMutation({
    mutationFn: (path: string) => linkRepo(path),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.repos });
    },
    onError: (error: Error) => {
      console.error('Failed to add repo:', error);
    },
  });

  const removeRepo = useMutation({
    mutationFn: (repoId: string) => unlinkRepo(repoId),
    onMutate: async repoId => {
      await queryClient.cancelQueries({ queryKey: queryKeys.repos });
      const previousRepos = queryClient.getQueryData<Repo[]>(queryKeys.repos);

      queryClient.setQueryData<Repo[]>(
        queryKeys.repos,
        old => old?.filter(r => r.id !== repoId) || []
      );

      return { previousRepos };
    },
    onError: (_error, _repoId, context) => {
      if (context?.previousRepos) {
        queryClient.setQueryData(queryKeys.repos, context.previousRepos);
      }
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.repos });
    },
  });

  return {
    ...query,
    addRepo,
    removeRepo,
    selectRepoFolder,
  };
}
