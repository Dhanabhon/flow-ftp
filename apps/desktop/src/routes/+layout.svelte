<script lang="ts">
  import { QueryClient } from '@tanstack/svelte-query';
  import { browser } from '$app/environment';
  import '../app.css';
  import { setSharedQueryClient } from '$lib/query-context';
  import { theme } from '$lib/stores/theme.svelte';

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

  // Hydrate theme store from storage + DOM. app.html already set the right
  // class pre-paint; this keeps the store in sync for the toggle UI.
  if (browser) theme.init();
</script>

<!--
  Kill the webview's default right-click menu (Reload / Inspect Element)
  outside editable fields. Text inputs keep the native paste menu.
-->
<svelte:window
  oncontextmenu={(e) => {
    const target = e.target as HTMLElement | null;
    if (!target?.closest('input, textarea, [contenteditable="true"]')) {
      e.preventDefault();
    }
  }}
/>

<svelte:head><title>FlowFTP</title></svelte:head>

{@render children?.()}
