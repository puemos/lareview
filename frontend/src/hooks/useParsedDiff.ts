import { useQuery, keepPreviousData } from '@tanstack/react-query';
import { useTauri } from '../hooks/useTauri';
import type { ParsedDiff } from '../types';

export function useParsedDiff(runId: string | null, diffText: string | null) {
  const { parseDiff } = useTauri();

  return useQuery({
    queryKey: ['parsedDiff', runId, diffText],
    queryFn: async (): Promise<ParsedDiff | null> => {
      if (!runId || !diffText) return null;
      return parseDiff(diffText);
    },
    enabled: !!runId && !!diffText,
    staleTime: 1000 * 60 * 30,
    gcTime: 1000 * 60 * 60,
    placeholderData: keepPreviousData,
  });
}
