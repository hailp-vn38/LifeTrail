import { QueryClient } from "@tanstack/vue-query";

/**
 * Shared TanStack Query client.
 *
 * Remote/server state lives here. Shared UI state lives in Pinia stores
 * (see `src/stores/`); URL state lives in Vue Router. Do not turn Pinia
 * into an API response cache.
 */
export function createQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: 30_000,
        retry: 1,
        refetchOnWindowFocus: false,
      },
    },
  });
}
