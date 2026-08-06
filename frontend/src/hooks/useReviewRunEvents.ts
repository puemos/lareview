import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useTauri } from './useTauri';
import { queryKeys } from '../lib/query-keys';
import type { ReviewRunEvent } from '../types';

export function useReviewRunEvents(runId: string | null) {
  const { getReviewRunEvents } = useTauri();
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: runId ? queryKeys.reviewRunEvents(runId) : ['reviewRunEvents', 'none'],
    queryFn: () => (runId ? getReviewRunEvents(runId) : Promise.resolve([])),
    enabled: !!runId,
    staleTime: Infinity,
  });

  useEffect(() => {
    if (!runId) return;
    const queryKey = queryKeys.reviewRunEvents(runId);
    const unlisten = listen<ReviewRunEvent>('review-run-event', ({ payload }) => {
      if (payload.run_id !== runId) return;
      queryClient.setQueryData<ReviewRunEvent[]>(queryKey, current => {
        const events = current || [];
        if (events.some(event => event.id === payload.id)) return events;
        return [...events, payload].sort((left, right) => left.id - right.id);
      });
    });

    getReviewRunEvents(runId)
      .then(history => {
        queryClient.setQueryData<ReviewRunEvent[]>(queryKey, current => {
          const byId = new Map<number, ReviewRunEvent>();
          for (const event of [...history, ...(current || [])]) byId.set(event.id, event);
          return [...byId.values()].sort((left, right) => left.id - right.id);
        });
      })
      .catch(console.error);

    return () => {
      unlisten.then(dispose => dispose()).catch(console.error);
    };
  }, [getReviewRunEvents, queryClient, runId]);

  return query;
}
