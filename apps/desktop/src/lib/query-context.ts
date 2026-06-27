import type { QueryClient } from '@tanstack/svelte-query';
import { writable, type Readable } from 'svelte/store';

/**
 * Symbol key for the shared QueryClient in Svelte context.
 * Components retrieve it with `useQueryClient()` from this lib.
 */
export const QUERY_CLIENT_KEY = Symbol('flow-ftp/query-client');

export function getQueryClient(): QueryClient {
  // Lazy singleton — the layout sets it via context, but we also keep a
  // module-level fallback so stores/utilities outside the component tree
  // (e.g. Tauri event handlers) can reach it.
  if (!_qc) throw new Error('QueryClient not initialized');
  return _qc;
}

let _qc: QueryClient | null = null;
export function setSharedQueryClient(qc: QueryClient) {
  _qc = qc;
}

// Convenience: a reactive flag for whether the desktop bridge is online.
export const bridgeOnline = writable<boolean>(true) as unknown as Readable<boolean>;
