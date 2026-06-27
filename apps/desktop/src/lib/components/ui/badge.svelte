<script lang="ts" module>
  import { tv } from 'tailwind-variants';
  export const badgeVariants = tv({
    base: 'inline-flex items-center gap-1 rounded-md px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wider transition-colors',
    variants: {
      variant: {
        neutral: 'bg-bg-hover text-fg-muted',
        accent: 'bg-accent/15 text-accent',
        success: 'bg-success/15 text-success',
        warning: 'bg-warning/15 text-warning',
        danger: 'bg-danger/15 text-danger',
        info: 'bg-info/15 text-info'
      },
      dot: { true: '', false: '' }
    },
    compoundVariants: [
      { variant: 'success', dot: true, class: 'before:content-[""] before:h-1.5 before:w-1.5 before:rounded-full before:bg-success' }
    ]
  });
</script>

<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';

  let {
    variant = 'neutral',
    dot = false,
    class: className,
    children
  }: {
    variant?: 'neutral' | 'accent' | 'success' | 'warning' | 'danger' | 'info';
    dot?: boolean;
    class?: string;
    children: Snippet;
  } = $props();
</script>

<span class={cn(badgeVariants({ variant, dot }), className)}>
  {@render children()}
</span>
