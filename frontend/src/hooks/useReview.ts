import { useQuery } from '@tanstack/react-query';
import { useTauri } from '../hooks/useTauri';
import { useMemo } from 'react';
import type { ReviewRun } from '../types';

export interface UseReviewResult {
  runId: string | null;
  runs: ReviewRun[];
  firstRun: ReviewRun | null;
  isLoading: boolean;
  error: Error | null;
}

export function useReview(reviewId: string | null): UseReviewResult {
  const { getReviewRuns } = useTauri();

  const {
    data: runs = [],
    isLoading,
    error,
  } = useQuery({
    queryKey: ['reviewRuns', reviewId],
    queryFn: () => (reviewId ? getReviewRuns(reviewId) : Promise.resolve<ReviewRun[]>([])),
    enabled: !!reviewId,
    staleTime: 30000,
  });

  const result = useMemo((): UseReviewResult => {
    const firstRun = runs.length > 0 ? runs[0] : null;
    return {
      runId: firstRun?.id ?? null,
      runs,
      firstRun,
      isLoading,
      error: error instanceof Error ? error : error ? new Error(String(error)) : null,
    };
  }, [runs, isLoading, error]);

  return result;
}
