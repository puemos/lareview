import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { useTauri } from '../hooks/useTauri';
import { queryKeys } from '../lib/query-keys';

interface Repo {
  id: string;
  name: string;
  path: string;
  review_count: number;
  linked_at: string;
  remotes: string[];
}

export function useRepos() {
  const { getLinkedRepos, linkRepo, cloneAndLinkRepo, unlinkRepo, selectRepoFolder } = useTauri();
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: queryKeys.repos,
    queryFn: getLinkedRepos,
  });

  const addRepo = useMutation({
    mutationFn: (path: string) => linkRepo(path),
    onSuccess: (_result, path) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.repos });
      toast('Repository Linked', {
        description: `${path.split('/').pop()} is now linked.`,
      });
    },
    onError: (error: Error) => {
      console.error('Failed to add repo:', error);
      toast('Failed to link repository', {
        description: error.message,
      });
    },
  });

  const cloneRepo = useMutation({
    mutationFn: (input: {
      provider: 'github' | 'gitlab';
      repo: string;
      host?: string;
      destDir: string;
    }) => cloneAndLinkRepo(input),
    onSuccess: result => {
      queryClient.invalidateQueries({ queryKey: queryKeys.repos });
      toast('Repository Cloned', {
        description: `${result.name} is now linked.`,
      });
    },
    onError: (error: Error) => {
      console.error('Failed to clone repo:', error);
      toast('Failed to clone repository', {
        description: error.message,
      });
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
    onError: (error, _repoId, context) => {
      if (context?.previousRepos) {
        queryClient.setQueryData(queryKeys.repos, context.previousRepos);
      }
      toast('Failed to unlink repository', {
        description: error instanceof Error ? error.message : String(error),
      });
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.repos });
    },
    onSuccess: () => {
      toast('Repository Unlinked', {
        description: 'The repository has been removed.',
      });
    },
  });

  return {
    ...query,
    addRepo,
    cloneRepo,
    removeRepo,
    selectRepoFolder,
  };
}
