import { useQuery } from '@tanstack/react-query';
import { useTauri } from './useTauri';
import { queryKeys } from '../lib/query-keys';

export function useMergeConfidence(runId: string | undefined) {
  const { getMergeConfidence } = useTauri();

  const query = useQuery({
    queryKey: queryKeys.mergeConfidence(runId ?? ''),
    queryFn: () => getMergeConfidence(runId!),
    enabled: !!runId,
  });

  return {
    data: query.data,
    isLoading: query.isLoading,
    error: query.error,
  };
}
