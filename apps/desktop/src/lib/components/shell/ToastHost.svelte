<script lang="ts">
  import { onMount } from 'svelte';
  import { cn } from '$lib/utils';
  import { app } from '$lib/stores/app.svelte';
  import { onEditUpdate } from '$lib/ipc';

  // Remote-edit outcomes land here as toasts.
  onMount(() => {
    let unlisten: (() => void) | null = null;
    onEditUpdate((event) => {
      if (event.kind === 'synced') {
        app.notify('success', 'Edit synced', event.fileName);
      } else {
        app.notify('danger', 'Edit sync failed', event.message ?? event.fileName);
      }
    }).then((un) => (unlisten = un));
    return () => unlisten?.();
  });
  import { IconCheckCircle, IconXCircle, IconAlert, IconX } from '$lib/components/icons';

  const icons = {
    success: IconCheckCircle,
    danger: IconXCircle,
    warning: IconAlert
  } as const;
</script>

<!--
  Bottom-right toast stack (DESIGN.md: "Never intrusive. Small toast.
  Bottom-right corner."). Topped above the transfer queue, pointer-events
  only on the toasts themselves.
-->
<div class="pointer-events-none fixed bottom-14 right-4 z-50 flex flex-col items-end gap-2">
  {#each app.toasts as toast (toast.id)}
    {@const Icon = icons[toast.kind]}
    <div
      class="pointer-events-auto flex w-80 items-start gap-2.5 rounded-lg border border-border bg-bg-elevated/95 p-3 shadow-lg backdrop-blur animate-[slide-up_0.22s_cubic-bezier(0.16,1,0.3,1)]"
      role="status"
    >
      <Icon
        size={16}
        class={cn(
          'mt-0.5 shrink-0',
          toast.kind === 'success' && 'text-success',
          toast.kind === 'warning' && 'text-warning',
          toast.kind === 'danger' && 'text-danger'
        )}
      />
      <div class="min-w-0 flex-1">
        <p class="text-sm font-medium text-fg">{toast.title}</p>
        {#if toast.detail}
          <p class="mt-0.5 truncate text-xs text-fg-subtle" title={toast.detail}>{toast.detail}</p>
        {/if}
      </div>
      <button
        class="rounded p-0.5 text-fg-subtle transition-colors hover:text-fg"
        onclick={() => app.dismissToast(toast.id)}
        aria-label="Dismiss"
      >
        <IconX size={12} />
      </button>
    </div>
  {/each}
</div>
