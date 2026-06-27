<script lang="ts">
  import { app } from '$lib/stores/app.svelte';
  import { IconSearch, IconPlus, IconRefresh, IconCommand, IconZap } from '$lib/components/icons';
  import Tooltip from '$lib/components/ui/tooltip.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import Badge from '$lib/components/ui/badge.svelte';

  // Keyboard shortcuts for the whole app
  function onKeydown(e: KeyboardEvent) {
    const mod = e.metaKey || e.ctrlKey;
    if (mod && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      app.openCommandPalette();
    }
  }
</script>

<svelte:window on:keydown={onKeydown} />

<header
  class="drag-region flex h-12 items-center gap-3 border-b border-border bg-bg-elevated px-3"
>
  <!-- macOS traffic-light spacer -->
  <div class="traffic-spacer"></div>

  <!-- Brand -->
  <div class="flex items-center gap-2 select-none">
    <div class="grid h-6 w-6 place-items-center rounded-md bg-accent text-white shadow-sm">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
        <path d="M3 12h4l3-9 4 18 3-9h4"/>
      </svg>
    </div>
    <span class="text-sm font-semibold tracking-tight">FlowFTP</span>
  </div>

  <!-- Center: command search -->
  <div class="mx-auto w-full max-w-xl">
    <button
      class="no-drag group flex h-8 w-full items-center gap-2.5 rounded-lg border border-border bg-bg/60 px-3 text-sm text-fg-subtle transition-colors hover:border-border-strong hover:bg-bg-hover"
      onclick={() => app.openCommandPalette()}
    >
      <IconSearch size={14} />
      <span class="flex-1 text-left">Search or run a command…</span>
      <kbd class="flex items-center gap-0.5 rounded border border-border bg-bg px-1.5 py-0.5 font-mono text-[10px] text-fg-subtle">
        <IconCommand size={10} /> K
      </kbd>
    </button>
  </div>

  <!-- Right cluster -->
  <div class="no-drag flex items-center gap-1">
    <Tooltip label="Quick Connect">
      <Button variant="ghost" size="icon-sm" onclick={() => (app.quickConnectOpen = true)}>
        <IconZap size={15} />
      </Button>
    </Tooltip>
    <Tooltip label="Refresh">
      <Button variant="ghost" size="icon-sm">
        <IconRefresh size={15} />
      </Button>
    </Tooltip>
    <Tooltip label="New connection">
      <Button variant="ghost" size="icon-sm">
        <IconPlus size={15} />
      </Button>
    </Tooltip>

    <div class="mx-1 h-5 w-px bg-border"></div>

    <!-- Plan indicator -->
    <Badge variant="accent" class="hidden sm:inline-flex">
      <IconZap size={10} /> Pro
    </Badge>
  </div>
</header>
