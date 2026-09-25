<script lang="ts">
  import { app } from '$lib/stores/app.svelte';
import { theme } from '$lib/stores/theme.svelte';
  import { mockCommands, type Command } from '$lib/mock';
  import { cn } from '$lib/utils';
  import {
    IconSearch,
    IconChevronRight,
    IconCornerDownLeft,
    IconPlus,
    IconZap,
    IconRefresh,
    IconUpload,
    IconDownload,
    IconPlug,
    IconArrowUpDown,
    IconHistory,
    IconSettings,
    IconServer,
    IconEye,
    IconCommand
  } from '$lib/components/icons';

  let query = $state('');
  let activeIndex = $state(0);
  let inputEl = $state<HTMLInputElement | null>(null);

  // Focus the search field each time the palette opens.
  $effect(() => {
    if (app.commandPaletteOpen && inputEl) {
      // tick to ensure the element is mounted
      requestAnimationFrame(() => inputEl?.focus());
    }
  });

  const iconMap: Record<string, any> = {
    plus: IconPlus, zap: IconZap, 'refresh-cw': IconRefresh, upload: IconUpload,
    download: IconDownload, plug: IconPlug, 'arrow-up-down': IconArrowUpDown,
    history: IconHistory, settings: IconSettings, server: IconServer, eye: IconEye
  };

  const filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return mockCommands;
    return mockCommands.filter(
      (c) =>
        c.label.toLowerCase().includes(q) ||
        c.group.toLowerCase().includes(q)
    );
  });

  // Group filtered results, preserving order
  const grouped = $derived.by(() => {
    const out: Record<string, Command[]> = {};
    for (const c of filtered) {
      (out[c.group] ??= []).push(c);
    }
    return out;
  });

  // Flat list for keyboard navigation
  const flat = $derived(filtered);

  function onKeydown(e: KeyboardEvent) {
    if (!app.commandPaletteOpen) return;
    if (e.key === 'Escape') {
      app.commandPaletteOpen = false;
      query = '';
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      activeIndex = Math.min(activeIndex + 1, flat.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      activeIndex = Math.max(activeIndex - 1, 0);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const cmd = flat[activeIndex];
      if (cmd) runCommand(cmd);
    }
  }

  function runCommand(cmd: Command) {
    if (cmd.id === 'quick-connect' || cmd.id === 'new-conn') app.quickConnectOpen = true;
    else if (cmd.id === 'sync') app.syncOpen = true;
    else if (cmd.id === 'go-browser') app.setView('connections');
    else if (cmd.id === 'go-transfers') app.setView('transfers');
    else if (cmd.id === 'toggle-hidden') app.toggleHidden();
    else if (cmd.id === 'toggle-theme') theme.toggle();
    app.commandPaletteOpen = false;
    query = '';
    activeIndex = 0;
  }

  // Reset index when query changes
  $effect(() => {
    query;
    activeIndex = 0;
  });
</script>

<svelte:window on:keydown={onKeydown} />

{#if app.commandPaletteOpen}
  <div class="fixed inset-0 z-50 flex items-start justify-center pt-[14vh]">
    <div
      class="absolute inset-0 bg-black/50 backdrop-blur-sm"
      onclick={() => (app.commandPaletteOpen = false)}
      role="presentation"
    ></div>

    <div
      class="relative w-full max-w-xl overflow-hidden rounded-xl border border-border bg-bg-elevated shadow-lg animate-[scale-in_0.14s_cubic-bezier(0.16,1,0.3,1)]"
      role="dialog"
      aria-modal="true"
    >
      <!-- Search input -->
      <div class="flex items-center gap-3 border-b border-border px-4">
        <IconSearch size={16} class="text-fg-subtle" />
        <input
          class="flex-1 bg-transparent py-3.5 text-sm text-fg outline-none placeholder:text-fg-faint"
          placeholder="Type a command or search…"
          bind:value={query}
          bind:this={inputEl}
        />
        <kbd class="rounded border border-border bg-bg px-1.5 py-0.5 font-mono text-[10px] text-fg-subtle">
          esc
        </kbd>
      </div>

      <!-- Results -->
      <div class="max-h-[50vh] overflow-y-auto p-2">
        {#each Object.entries(grouped) as [group, items]}
          <div class="mb-1 mt-2 px-2 text-[10px] font-semibold uppercase tracking-wider text-fg-subtle">
            {group}
          </div>
          {#each items as cmd (cmd.id)}
            {@const flatIdx = flat.indexOf(cmd)}
            {@const Icon = cmd.icon ? iconMap[cmd.icon] : null}
            <button
              class={cn(
                'flex w-full items-center gap-3 rounded-md px-2.5 py-2 text-left text-sm transition-colors',
                flatIdx === activeIndex ? 'bg-bg-active text-fg' : 'text-fg-muted'
              )}
              onclick={() => runCommand(cmd)}
              onmouseenter={() => (activeIndex = flatIdx)}
            >
              {#if Icon}
                <Icon size={15} class={cn(flatIdx === activeIndex ? 'text-accent' : 'text-fg-subtle')} />
              {:else}
                <span class="w-[15px]"></span>
              {/if}
              <span class="flex-1 truncate">{cmd.label}</span>
              {#if cmd.shortcut}
                <span class="flex items-center gap-0.5">
                  {#each cmd.shortcut as key}
                    <kbd class="rounded border border-border bg-bg px-1.5 py-0.5 font-mono text-[10px] text-fg-subtle">
                      {key}
                    </kbd>
                  {/each}
                </span>
              {/if}
              {#if flatIdx === activeIndex}
                <IconCornerDownLeft size={13} class="text-fg-subtle" />
              {/if}
            </button>
          {/each}
        {:else}
          <div class="py-8 text-center text-sm text-fg-subtle">
            No commands found for "{query}"
          </div>
        {/each}
      </div>

      <!-- Footer hint -->
      <div class="flex items-center justify-between border-t border-border px-4 py-2 text-[10px] text-fg-faint">
        <div class="flex items-center gap-3">
          <span class="flex items-center gap-1">
            <kbd class="rounded border border-border bg-bg px-1 font-mono">↑</kbd>
            <kbd class="rounded border border-border bg-bg px-1 font-mono">↓</kbd>
            to navigate
          </span>
          <span class="flex items-center gap-1">
            <kbd class="rounded border border-border bg-bg px-1 font-mono">↵</kbd>
            to select
          </span>
        </div>
        <span class="flex items-center gap-1">
          <IconCommand size={10} /> FlowFTP
        </span>
      </div>
    </div>
  </div>
{/if}
