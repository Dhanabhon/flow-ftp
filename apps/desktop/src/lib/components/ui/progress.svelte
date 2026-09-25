<script lang="ts">
  import { cn } from '$lib/utils';

  let {
    value = 0,
    tone = 'accent',
    class: className
  }: {
    value?: number; // 0..100
    tone?: 'accent' | 'success' | 'warning' | 'danger' | 'speed';
    class?: string;
  } = $props();

  const pct = $derived(Math.max(0, Math.min(100, value)));

  const barTone: Record<string, string> = {
    accent: 'bg-accent',
    success: 'bg-success',
    warning: 'bg-warning',
    danger: 'bg-danger',
    speed: 'bg-gradient-to-r from-speed-slow via-speed-medium to-speed-fast'
  };
</script>

<div class={cn('h-1.5 w-full overflow-hidden rounded-full bg-bg-hover', className)}>
  <div
    class={cn('h-full w-full origin-left rounded-full transition-transform duration-300 ease-out', barTone[tone])}
    style="transform: scaleX({pct / 100})"
    role="progressbar"
    aria-valuenow={Math.round(pct)}
    aria-valuemin="0"
    aria-valuemax="100"
  ></div>
</div>
