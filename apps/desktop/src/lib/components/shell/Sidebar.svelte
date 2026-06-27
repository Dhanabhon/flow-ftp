<script lang="ts">
  import { app } from '$lib/stores/app.svelte';
  import { cn } from '$lib/utils';
  import type { View } from '$lib/types';
  import {
    IconPlug,
    IconArrowUpDown,
    IconRefresh,
    IconHistory,
    IconSettings,
    IconStar,
    IconClock,
    IconServer,
    IconPlus,
    IconWifi,
    IconWifiOff,
    IconAlert,
    IconShield
  } from '$lib/components/icons';
  import Badge from '$lib/components/ui/badge.svelte';

  const nav: { id: View; label: string; icon: any; shortcut: string }[] = [
    { id: 'connections', label: 'Connections', icon: IconPlug, shortcut: '⌘1' },
    { id: 'transfers', label: 'Transfers', icon: IconArrowUpDown, shortcut: '⌘2' },
    { id: 'sync', label: 'Sync', icon: IconRefresh, shortcut: '⌘3' },
    { id: 'history', label: 'History', icon: IconHistory, shortcut: '⌘4' },
    { id: 'settings', label: 'Settings', icon: IconSettings, shortcut: '⌘,' }
  ];

  let activeView = $derived(app.view);

  const favorites = $derived(app.connections.filter((c) => c.favorite));
  const recent = $derived(
    [...app.connections]
      .sort((a, b) => (b.lastConnected ?? 0) - (a.lastConnected ?? 0))
      .slice(0, 3)
  );

  function statusDot(status: string) {
    return status === 'connected'
      ? 'bg-success'
      : status === 'connecting'
        ? 'bg-warning'
        : status === 'error'
          ? 'bg-danger'
          : 'bg-fg-faint';
  }
</script>

<aside class="flex w-60 flex-col border-r border-border bg-bg-elevated">
  <!-- Primary nav -->
  <nav class="flex flex-col gap-0.5 p-2">
    {#each nav as item (item.id)}
      {@const active = activeView === item.id}
      <button
        class={cn(
          'group flex items-center gap-2.5 rounded-md px-2.5 py-1.5 text-sm transition-colors',
          active
            ? 'bg-bg-active text-fg'
            : 'text-fg-muted hover:bg-bg-hover hover:text-fg'
        )}
        onclick={() => app.setView(item.id)}
      >
        <item.icon size={16} class={cn(active && 'text-accent')} />
        <span class="flex-1 text-left">{item.label}</span>
        {#if item.id === 'transfers' && app.activeTransfers.length > 0}
          <span class="rounded bg-accent/15 px-1.5 py-0.5 text-[10px] font-semibold text-accent">
            {app.activeTransfers.length}
          </span>
        {:else if item.id === 'transfers' && app.failedCount > 0}
          <span class="rounded bg-danger/15 px-1.5 py-0.5 text-[10px] font-semibold text-danger">
            {app.failedCount}
          </span>
        {/if}
      </button>
    {/each}
  </nav>

  <div class="mx-3 my-1 h-px bg-border"></div>

  <!-- Favorites -->
  <div class="flex items-center justify-between px-3 pb-1 pt-2">
    <span class="text-[10px] font-semibold uppercase tracking-wider text-fg-subtle">
      Favorites
    </span>
    <button class="text-fg-subtle transition-colors hover:text-fg">
      <IconPlus size={12} />
    </button>
  </div>
  <div class="flex flex-col gap-0.5 px-2">
    {#each favorites as conn (conn.id)}
      <button
        class="group flex items-center gap-2.5 rounded-md px-2.5 py-1.5 text-left text-sm text-fg-muted transition-colors hover:bg-bg-hover hover:text-fg"
      >
        <span class={cn('h-1.5 w-1.5 shrink-0 rounded-full', statusDot(conn.status))}></span>
        <IconServer size={14} class="shrink-0 text-fg-subtle group-hover:text-fg-muted" />
        <span class="flex-1 truncate">{conn.name}</span>
        <span class="font-mono text-[10px] text-fg-faint uppercase">{conn.protocol}</span>
      </button>
    {/each}
  </div>

  <!-- Recent -->
  <div class="mt-3 flex items-center px-3 pb-1 pt-2">
    <span class="text-[10px] font-semibold uppercase tracking-wider text-fg-subtle">
      Recent
    </span>
  </div>
  <div class="flex flex-col gap-0.5 px-2">
    {#each recent as conn (conn.id)}
      <button
        class="group flex items-center gap-2.5 rounded-md px-2.5 py-1.5 text-left text-sm text-fg-muted transition-colors hover:bg-bg-hover hover:text-fg"
      >
        <IconClock size={14} class="shrink-0 text-fg-faint" />
        <span class="flex-1 truncate">{conn.name}</span>
      </button>
    {/each}
  </div>

  <!-- Spacer pushes footer down -->
  <div class="flex-1"></div>

  <!-- Footer: security + keychain status -->
  <div class="m-2 rounded-lg border border-border bg-bg-panel p-3">
    <div class="flex items-center gap-2">
      <div class="grid h-7 w-7 place-items-center rounded-md bg-success/15">
        <IconShield size={14} class="text-success" />
      </div>
      <div class="min-w-0 flex-1">
        <div class="flex items-center gap-1.5">
          <span class="text-xs font-semibold text-fg">Keychain</span>
          <Badge variant="success" dot>Active</Badge>
        </div>
        <p class="mt-0.5 truncate text-[11px] text-fg-subtle">
          {app.connections.filter((c) => c.keychain).length} credentials secured
        </p>
      </div>
    </div>
  </div>
</aside>
