import { useQuery } from '@tanstack/react-query';
import { useTauri } from '../hooks/useTauri';
import { queryKeys } from '../lib/query-keys';

export function useAgents() {
  const { getAgents } = useTauri();

  return useQuery({
    queryKey: queryKeys.agents,
    queryFn: async () => {
      const result = await getAgents();
      return result.map(a => ({
        id: a.id,
        name: a.name,
        description: a.description || 'Custom agent',
        path: a.path,
        available: a.available,
      }));
    },
  });
}
