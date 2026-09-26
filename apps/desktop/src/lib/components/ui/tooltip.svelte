<script lang="ts">
  /**
   * Lightweight tooltip — shows on hover or keyboard focus, with a short
   * delay. Pure CSS positioning (above the trigger by default).
   *
   * Smart horizontal alignment: near the window's right edge the tooltip
   * right-aligns to the trigger instead of centering (which would clip);
   * near the left edge it left-aligns. For complex positioning, swap in
   * bits-ui's Popper.
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

  let wrapperEl = $state<HTMLElement | null>(null);
  let tipEl = $state<HTMLElement | null>(null);
  type Align = 'center' | 'start' | 'end';
  let align = $state<Align>('center');

  /** Choose horizontal alignment before the tooltip becomes visible. */
  function updateAlign() {
    if (!wrapperEl || !tipEl || (side !== 'top' && side !== 'bottom')) return;
    const rect = wrapperEl.getBoundingClientRect();
    const tipWidth = tipEl.offsetWidth || 0;
    const margin = 8;
    const centerX = rect.left + rect.width / 2;
    if (centerX + tipWidth / 2 > window.innerWidth - margin) {
      align = 'end';
    } else if (centerX - tipWidth / 2 < margin) {
      align = 'start';
    } else {
      align = 'center';
    }
  }

  const vertical = {
    top: 'bottom-full mb-1.5',
    bottom: 'top-full mt-1.5'
  } as const;

  const horizontal: Record<Align, string> = {
    center: 'left-1/2 -translate-x-1/2',
    start: 'left-0',
    end: 'right-0'
  };
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<span
  bind:this={wrapperEl}
  class="relative inline-flex group/tt"
  onpointerenter={updateAlign}
  onfocusin={updateAlign}
>
  {@render children()}
  <span
    bind:this={tipEl}
    class={cn(
      'pointer-events-none absolute z-50 whitespace-nowrap rounded-md border border-border bg-bg-elevated px-2 py-1 text-xs text-fg shadow-md',
      'opacity-0 scale-95 transition-all duration-150',
      'group-hover/tt:opacity-100 group-hover/tt:scale-100 group-hover/tt:delay-200',
      'group-focus-within/tt:opacity-100 group-focus-within/tt:scale-100 group-focus-within/tt:delay-200',
      side === 'top' && vertical.top,
      side === 'bottom' && vertical.bottom,
      (side === 'top' || side === 'bottom') && horizontal[align],
      side === 'left' && 'right-full top-1/2 -translate-y-1/2 mr-1.5',
      side === 'right' && 'left-full top-1/2 -translate-y-1/2 ml-1.5',
      className
    )}
  >
    {label}
  </span>
</span>
