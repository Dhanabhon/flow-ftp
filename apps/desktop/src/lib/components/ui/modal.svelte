<script lang="ts">
  import { cn } from '$lib/utils';
  import { focusTrap } from '$lib/actions/focus-trap';
  import { fade, scale } from 'svelte/transition';
  import type { Snippet } from 'svelte';

  let {
    open = $bindable(false),
    title = '',
    description = '',
    width = 'md',
    center = false,
    class: className,
    children,
    footer
  }: {
    open?: boolean;
    title?: string;
    description?: string;
    width?: 'sm' | 'md' | 'lg' | 'xl';
    /** Center vertically in the window (for compact confirmations). */
    center?: boolean;
    class?: string;
    children: Snippet;
    footer?: Snippet;
  } = $props();

  const widths = {
    sm: 'max-w-sm',
    md: 'max-w-lg',
    lg: 'max-w-2xl',
    xl: 'max-w-4xl'
  };

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') open = false;
  }
</script>

<svelte:window on:keydown={onKeydown} />

{#if open}
  <div
    class={cn(
      'fixed inset-0 z-50 flex justify-center p-6',
      center ? 'items-center' : 'items-start pt-[12vh]'
    )}
  >
    <!-- backdrop -->
    <div
      class="absolute inset-0 bg-black/60 backdrop-blur-sm"
      transition:fade={{ duration: 150 }}
      onclick={() => (open = false)}
      role="presentation"
    ></div>

    <!-- panel -->
    <div
      class={cn(
        'relative w-full overflow-hidden rounded-xl border border-border bg-bg-elevated shadow-lg',
        'animate-[scale-in_0.16s_cubic-bezier(0.16,1,0.3,1)]',
        widths[width],
        className
      )}
      transition:scale={{ duration: 160, start: 0.97 }}
      role="dialog"
      aria-modal="true"
      aria-label={title}
      use:focusTrap
    >
      {#if title}
        <div class="border-b border-border px-5 py-4">
          <h2 class="text-base font-semibold text-fg">{title}</h2>
          {#if description}
            <p class="mt-0.5 text-sm text-fg-muted">{description}</p>
          {/if}
        </div>
      {/if}

      <div class="px-5 py-4">
        {@render children()}
      </div>

      {#if footer}
        <div class="flex justify-end gap-2 border-t border-border bg-bg/40 px-5 py-3">
          {@render footer()}
        </div>
      {/if}
    </div>
  </div>
{/if}
