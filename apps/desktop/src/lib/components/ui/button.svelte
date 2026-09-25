<script lang="ts" module>
  import { tv } from 'tailwind-variants';
  export const buttonVariants = tv({
    base: 'inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-all duration-150 disabled:pointer-events-none disabled:opacity-40 select-none',
    variants: {
      variant: {
        default: 'bg-accent-solid text-accent-fg hover:bg-accent-solid-hover shadow-sm',
        secondary:
          'bg-bg-panel text-fg hover:bg-bg-hover border border-border',
        ghost: 'text-fg-muted hover:text-fg hover:bg-bg-hover',
        outline:
          'border border-border text-fg hover:bg-bg-hover hover:border-border-strong',
        danger:
          'bg-danger-solid text-white hover:brightness-110 shadow-sm',
        subtle: 'bg-bg-hover text-fg hover:bg-bg-active'
      },
      size: {
        sm: 'h-7 px-2.5 text-xs',
        md: 'h-9 px-3.5',
        lg: 'h-11 px-5 text-base',
        icon: 'h-8 w-8 p-0',
        'icon-sm': 'h-7 w-7 p-0'
      }
    },
    defaultVariants: { variant: 'default', size: 'md' }
  });
  export type ButtonVariant =
    | 'default' | 'secondary' | 'ghost' | 'outline' | 'danger' | 'subtle';
  export type ButtonSize =
    | 'sm' | 'md' | 'lg' | 'icon' | 'icon-sm';
</script>

<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes, HTMLAnchorAttributes } from 'svelte/elements';

  let {
    variant = 'default',
    size = 'md',
    class: className,
    children,
    ...restProps
  }: {
    variant?: ButtonVariant;
    size?: ButtonSize;
    class?: string;
    children: Snippet;
  } & HTMLButtonAttributes = $props();
</script>

<button class={cn(buttonVariants({ variant, size }), className)} {...restProps}>
  {@render children()}
</button>
