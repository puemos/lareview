import { useQuery } from '@tanstack/react-query';
import { useTauri } from './useTauri';
import { queryKeys } from '../lib/query-keys';

export function useIssueChecks(runId: string | undefined) {
  const { getIssueChecksForRun } = useTauri();

  return useQuery({
    queryKey: queryKeys.issueChecks(runId ?? ''),
    queryFn: () => getIssueChecksForRun(runId!),
    enabled: !!runId,
  });
}
