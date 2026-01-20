export const QUERY_CONFIG = {
  staleTime: 30 * 1000,
  gcTime: 60 * 60 * 1000,
  retry: 0,
  refetchOnWindowFocus: false,
  feedbackStaleTime: 30 * 1000,
} as const;

export const PERSIST_CONFIG = {
  name: 'lareview-storage',
  maxAge: 24 * 60 * 60 * 1000,
} as const;

export const STORAGE_KEYS = {
  agentId: 'agentId',
} as const;
