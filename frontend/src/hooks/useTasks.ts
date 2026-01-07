import { useQuery, useMutation, useQueryClient, keepPreviousData } from '@tanstack/react-query';
import { useTauri } from '../hooks/useTauri';
import { queryKeys } from '../lib/query-keys';
import type { ReviewTask } from '../types';

export interface UseTasksResult {
  data: ReviewTask[];
  isLoading: boolean;
  isFetching: boolean;
  isPending: boolean;
  error: Error | null;
  updateTaskStatus: (vars: { taskId: string; status: ReviewTask['status'] }) => void;
  isUpdatingStatus: boolean;
}

export function useTasks(runId: string | null): UseTasksResult & {
  updateTaskStatus: (vars: { taskId: string; status: ReviewTask['status'] }) => void;
} {
  const { loadTasks, updateTaskStatus: updateStatus } = useTauri();
  const queryClient = useQueryClient();

  const queryKey = runId ? queryKeys.tasks(runId) : [];

  const { data, isLoading, isFetching, isPending, error } = useQuery({
    queryKey,
    queryFn: () => (runId ? loadTasks(runId) : Promise.resolve<ReviewTask[]>([])),
    enabled: !!runId,
    staleTime: 30000,
    placeholderData: keepPreviousData,
  });

  const mutation = useMutation({
    mutationFn: ({ taskId, status }: { taskId: string; status: ReviewTask['status'] }) =>
      updateStatus(taskId, status),
    onMutate: async ({ taskId, status }) => {
      await queryClient.cancelQueries({ queryKey });
      const previousTasks = queryClient.getQueryData<ReviewTask[]>(queryKey);

      queryClient.setQueryData<ReviewTask[]>(
        queryKey,
        old => old?.map(t => (t.id === taskId ? { ...t, status } : t)) || []
      );

      return { previousTasks };
    },
    onError: (_error, _vars, context) => {
      if (context?.previousTasks) {
        queryClient.setQueryData(queryKey, context.previousTasks);
      }
    },
    onSettled: () => {
      if (runId) {
        queryClient.invalidateQueries({ queryKey: queryKeys.tasks(runId) });
      }
    },
  });

  return {
    data: data || [],
    isLoading,
    isFetching,
    isPending,
    error: error instanceof Error ? error : error ? new Error(String(error)) : null,
    updateTaskStatus: mutation.mutate,
    isUpdatingStatus: mutation.isPending,
  };
}
