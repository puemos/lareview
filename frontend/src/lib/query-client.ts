import { QueryClient } from '@tanstack/react-query';
import { persistQueryClient } from '@tanstack/react-query-persist-client';
import { createSyncStoragePersister } from '@tanstack/query-sync-storage-persister';
import { QUERY_CONFIG, PERSIST_CONFIG } from '../constants/query-config';

function createPersister() {
  if (typeof window === 'undefined') {
    return null;
  }
  try {
    return createSyncStoragePersister({
      storage: window.localStorage,
    });
  } catch {
    return null;
  }
}

export const createQueryClient = () => {
  const client = new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: QUERY_CONFIG.staleTime,
        gcTime: QUERY_CONFIG.gcTime,
        retry: QUERY_CONFIG.retry,
        refetchOnWindowFocus: QUERY_CONFIG.refetchOnWindowFocus,
      },
    },
  });

  const persister = createPersister();
  if (persister) {
    persistQueryClient({
      queryClient: client,
      persister,
      maxAge: PERSIST_CONFIG.maxAge,
      hydrateOptions: {
        defaultOptions: {
          queries: {
            gcTime: PERSIST_CONFIG.maxAge,
          },
        },
      },
    });
  }

  return client;
};
