import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useTauri } from '../hooks/useTauri';
import { queryKeys } from '../lib/query-keys';
import type { Review } from '../types';

export function useReviews() {
  const { getAllReviews } = useTauri();
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: queryKeys.reviews,
    queryFn: async () => {
      const result = await getAllReviews();
      return result.map(r => ({
        ...r,
        status: 'pending',
      })) as Review[];
    },
  });

  const invalidate = () => {
    queryClient.invalidateQueries({ queryKey: queryKeys.reviews });
  };

  return {
    ...query,
    invalidate,
  };
}
