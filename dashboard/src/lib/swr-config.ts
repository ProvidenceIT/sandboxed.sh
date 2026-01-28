import type { SWRConfiguration } from 'swr';

/**
 * SWR configuration for static/infrequently changing data.
 * Use for: agent lists, backend configs, provider types, etc.
 * 
 * - No revalidation on window focus
 * - 30s deduping interval to prevent redundant requests
 */
export const staticFetchConfig: SWRConfiguration = {
  revalidateOnFocus: false,
  dedupingInterval: 30000,
};

/**
 * SWR configuration for data that should refresh on focus.
 * Use for: health status, connection status, etc.
 * 
 * - Revalidates when user returns to the tab
 * - No deduping interval
 */
export const liveFetchConfig: SWRConfiguration = {
  revalidateOnFocus: true,
  dedupingInterval: 0,
};

/**
 * SWR configuration for frequently polling data.
 * Use for: mission status, queue updates, progress tracking
 * 
 * - Auto-refresh every 5 seconds
 * - Revalidates on focus
 */
export const pollingFetchConfig: SWRConfiguration = {
  revalidateOnFocus: true,
  refreshInterval: 5000,
};

/**
 * SWR configuration for data that should not auto-refresh.
 * Use for: library resources that only change on explicit user action
 * 
 * - No revalidation on focus
 * - No auto-refresh
 * - Revalidates on mount
 */
export const manualFetchConfig: SWRConfiguration = {
  revalidateOnFocus: false,
  revalidateOnReconnect: false,
  revalidateIfStale: true,
};
