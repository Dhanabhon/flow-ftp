<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { app } from '$lib/stores/app.svelte';
  import { IconSearch, IconPlus, IconCommand, IconZap } from '$lib/components/icons';
  import Tooltip from '$lib/components/ui/tooltip.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import ThemeToggle from '$lib/components/ui/theme-toggle.svelte';

  // Keyboard shortcuts for the whole app
  function onKeydown(e: KeyboardEvent) {
    const mod = e.metaKey || e.ctrlKey;
    if (mod && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      app.openCommandPalette();
    }
  }

  function startWindowDrag(e: MouseEvent) {
    if (e.button !== 0) return;
    if (e.target instanceof Element && e.target.closest('.no-drag, button, input, textarea, select, a')) {
      return;
    }
    e.preventDefault();
    void getCurrentWindow().startDragging().catch(() => {});
  }

  function windowDrag(node: HTMLElement) {
    node.addEventListener('mousedown', startWindowDrag);
    return {
      destroy() {
        node.removeEventListener('mousedown', startWindowDrag);
      }
    };
  }
</script>

<svelte:window on:keydown={onKeydown} />

<header
  data-tauri-drag-region
  use:windowDrag
  class="relative flex h-12 items-center gap-3 border-b border-border bg-bg-elevated px-3 drag-strip"
>
  <!-- macOS traffic-light spacer (overlay style -> lights paint over this) -->
  <div class="traffic-spacer" data-tauri-drag-region></div>

  <!-- Brand -->
  <div class="relative z-10 flex items-center gap-2 select-none">
    <div class="grid h-6 w-6 place-items-center text-fg">
      <svg width="24" height="24" viewBox="0 0 128 128" fill="none" aria-hidden="true">
        <path d="M14 40C30 22 47 22 63 40S96 58 114 40" stroke="currentColor" stroke-width="14" stroke-linecap="round"/>
        <path d="M14 64C30 46 47 46 63 64S96 82 114 64" stroke="#2563EB" stroke-width="14" stroke-linecap="round"/>
        <path d="M14 88C30 70 47 70 63 88S96 106 114 88" stroke="#72C7FA" stroke-width="14" stroke-linecap="round"/>
      </svg>
    </div>
    <span class="text-sm font-semibold tracking-tight">FlowFTP</span>
  </div>

  <!-- Center: command search -->
  <div class="no-drag relative z-10 mx-auto w-full max-w-xl">
    <button
      class="group flex h-8 w-full items-center gap-2.5 rounded-lg border border-border bg-bg/60 px-3 text-sm text-fg-subtle transition-colors hover:border-border-strong hover:bg-bg-hover"
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
  <div class="no-drag relative z-10 flex items-center gap-1">
    <Tooltip label="Quick Connect" side="bottom">
      <Button
        variant="ghost"
        size="icon-sm"
        aria-label="Quick Connect"
        onclick={() => {
          app.quickConnectMode = 'quick';
          app.quickConnectOpen = true;
        }}>
        <IconZap size={15} />
      </Button>
    </Tooltip>
    <Tooltip label="New connection" side="bottom">
      <Button
        variant="ghost"
        size="icon-sm"
        aria-label="New connection"
        onclick={() => {
          app.quickConnectMode = 'new';
          app.quickConnectOpen = true;
        }}>
        <IconPlus size={15} />
      </Button>
    </Tooltip>

    <ThemeToggle />

  </div>
</header>
