<script lang="ts">
  /**
   * Lightweight tooltip — shows on hover with a short delay.
   * Pure CSS positioning (above the trigger by default).
   * For complex positioning, swap in bits-ui's Popper.
   */
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';

  let {
    label,
    side = 'top',
    class: className,
    children
  }: {
    label: string;
    side?: 'top' | 'bottom' | 'left' | 'right';
    class?: string;
    children: Snippet;
  } = $props();

  const pos = $derived(
    ({
      top: 'bottom-full left-1/2 -translate-x-1/2 mb-1.5',
      bottom: 'top-full left-1/2 -translate-x-1/2 mt-1.5',
      left: 'right-full top-1/2 -translate-y-1/2 mr-1.5',
      right: 'left-full top-1/2 -translate-y-1/2 ml-1.5'
    } as const)[side]
  );
</script>

<span class="relative inline-flex group/tt">
  {@render children()}
  <span
    class={cn(
      'pointer-events-none absolute z-50 whitespace-nowrap rounded-md border border-border bg-bg-elevated px-2 py-1 text-xs text-fg shadow-md',
      'opacity-0 scale-95 transition-all duration-150',
      'group-hover/tt:opacity-100 group-hover/tt:scale-100 group-hover/tt:delay-200',
      pos,
      className
    )}
  >
    {label}
  </span>
</span>
