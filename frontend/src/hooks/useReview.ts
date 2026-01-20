import { useQuery, keepPreviousData } from '@tanstack/react-query';
import { useTauri } from '../hooks/useTauri';
import { useMemo } from 'react';

interface ReviewRunData {
  id: string;
  review_id: string;
  agent_id: string;
  input_ref: string;
  diff_text: string;
  created_at: string;
  task_count: number;
  status?: string;
}

export interface UseReviewResult {
  runId: string | null;
  runs: ReviewRunData[];
  firstRun: ReviewRunData | null;
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
    queryFn: () => (reviewId ? getReviewRuns(reviewId) : Promise.resolve<ReviewRunData[]>([])),
    enabled: !!reviewId,
    staleTime: 30000,
    placeholderData: keepPreviousData,
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
