<script lang="ts">
  import { QueryClient } from '@tanstack/svelte-query';
  import '../app.css';
  import { setSharedQueryClient } from '$lib/query-context';

  let { children } = $props();

  // Single QueryClient for the app. In real usage, queries call `invoke()`
  // to hit the Rust backend; here they're stubbed with mock data.
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        enabled: true,
        refetchOnWindowFocus: false,
        staleTime: 30_000,
        retry: 1
      }
    }
  });
  setSharedQueryClient(queryClient);
</script>

<svelte:head><title>FlowFTP</title></svelte:head>

{@render children?.()}
