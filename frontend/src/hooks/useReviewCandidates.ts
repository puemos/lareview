import { useQuery } from '@tanstack/react-query';
import { useTauri } from './useTauri';
import { queryKeys } from '../lib/query-keys';

/** How long candidate listings stay fresh before a refetch is allowed. */
const STALE_TIME_MS = 2 * 60 * 1000;

/**
 * Pull requests awaiting the user's review.
 *
 * Candidates are remote state, so they are cached rather than persisted and can
 * be refetched on demand.
 */
export function useReviewCandidates(enabled = true) {
  const { listReviewCandidates } = useTauri();

  const query = useQuery({
    queryKey: queryKeys.reviewCandidates,
    queryFn: listReviewCandidates,
    staleTime: STALE_TIME_MS,
    enabled,
  });

  return {
    candidates: query.data?.candidates ?? [],
    errors: query.data?.errors ?? [],
    unsupportedProviders: query.data?.unsupportedProviders ?? [],
    isLoading: query.isLoading,
    isFetching: query.isFetching,
    error: query.error,
    refetch: query.refetch,
  };
}
