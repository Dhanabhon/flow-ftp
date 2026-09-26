<script lang="ts">
  import { app } from '$lib/stores/app.svelte';
  import { cn } from '$lib/utils';
  import { profileDelete, profileSave } from '$lib/ipc';
  import type { Connection, View } from '$lib/types';
  import {
    IconPlug,
    IconArrowUpDown,
    IconRefresh,

    IconStar,
    IconClock,
    IconTrash,
    IconUnplug,
    IconServer,
    IconPlus,
    IconWifi,
    IconWifiOff,
    IconAlert,
    IconShield
  } from '$lib/components/icons';
  import Badge from '$lib/components/ui/badge.svelte';
  import Modal from '$lib/components/ui/modal.svelte';
  import Button from '$lib/components/ui/button.svelte';

  // Honest nav: only destinations that exist. Sync opens the planner modal;
  // History/Settings return when they have content.
  const nav: { id: View | 'sync'; label: string; icon: any; shortcut: string }[] = [
    { id: 'connections', label: 'Browser', icon: IconPlug, shortcut: '⌘1' },
    { id: 'transfers', label: 'Transfers', icon: IconArrowUpDown, shortcut: '⌘2' },
    { id: 'sync', label: 'Sync', icon: IconRefresh, shortcut: '⌘⇧S' }
  ];

  let activeView = $derived(app.view);

  const favorites = $derived(app.connections.filter((c) => c.favorite));
  const recent = $derived(app.recentConnections);

  /**
   * Click a saved connection: activate it when already connected, reconnect
   * via keychain when possible, or open Quick Connect prefilled.
   */
  async function openConnection(conn: Connection) {
    if (conn.status === 'connected') {
      app.activeConnectionId = conn.id;
      return;
    }
    if (conn.keychain) {
      try {
        await app.reconnectSavedConnection(conn);
      } catch (e) {
        app.notify('warning', 'Could not reconnect', e instanceof Error ? e.message : String(e));
        app.quickConnectMode = 'quick';
        app.quickConnectPrefill = {
          protocol: conn.protocol,
          host: conn.host,
          port: conn.port,
          username: conn.username
        };
        app.quickConnectOpen = true;
      }
      return;
    }
    app.quickConnectMode = 'quick';
    app.quickConnectPrefill = {
      protocol: conn.protocol,
      host: conn.host,
      port: conn.port,
      username: conn.username
    };
    app.quickConnectOpen = true;
  }

  async function toggleFavorite(conn: Connection) {
    try {
      app.connections = await profileSave({ ...conn, favorite: !conn.favorite });
    } catch (e) {
      app.notify('danger', 'Could not update favorite', e instanceof Error ? e.message : String(e));
    }
  }

  /** Pending profile deletion awaiting confirmation (zero-fear rule). */
  let deletePending = $state<Connection | null>(null);

  function requestDeleteProfile(conn: Connection) {
    deletePending = conn;
  }

  async function confirmDeleteProfile() {
    const conn = deletePending;
    deletePending = null;
    if (!conn) return;
    try {
      app.connections = await profileDelete(conn.id);
      if (app.activeConnectionId === conn.id) app.activeConnectionId = null;
    } catch (e) {
      app.notify('danger', 'Could not delete connection', e instanceof Error ? e.message : String(e));
    }
  }

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
        onclick={() => {
          if (item.id === 'sync') {
            app.syncOpen = true;
          } else {
            app.setView(item.id as View);
          }
        }}
      >
        <item.icon size={16} class={cn(active && 'text-accent-text')} />
        <span class="flex-1 text-left">{item.label}</span>
        {#if item.id === 'transfers' && app.activeTransfers.length > 0}
          <span class="rounded bg-accent/15 px-1.5 py-0.5 text-[10px] font-semibold text-accent-text">
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
      <div
        class="group flex cursor-pointer items-center gap-2.5 rounded-md px-2.5 py-1.5 text-left text-sm text-fg-muted transition-colors hover:bg-bg-hover hover:text-fg"
        role="button"
        tabindex="0"
        onclick={() => openConnection(conn)}
        onkeydown={(e) => e.key === 'Enter' && openConnection(conn)}
      >
        <span class={cn('h-1.5 w-1.5 shrink-0 rounded-full', statusDot(conn.status))}></span>
        <IconServer size={14} class="shrink-0 text-fg-subtle group-hover:text-fg-muted" />
        <span class="flex-1 truncate">{conn.name}</span>
        <span class="hidden items-center gap-0.5 group-hover:flex">
          {#if conn.status === 'connected'}
            <button
              class="rounded p-0.5 text-fg-subtle hover:text-accent-text"
              aria-label="Disconnect {conn.name}"
              title="Disconnect"
              onclick={(e) => { e.stopPropagation(); app.disconnectActive(); }}
            >
              <IconUnplug size={12} />
            </button>
          {/if}
          <button
            class="rounded p-0.5 text-fg-subtle hover:text-warning"
            aria-label="Remove {conn.name} from favorites"
            title="Remove from favorites"
            onclick={(e) => { e.stopPropagation(); toggleFavorite(conn); }}
          >
            <IconStar size={12} />
          </button>
          <button
            class="rounded p-0.5 text-fg-subtle hover:text-danger"
            aria-label="Delete connection {conn.name}"
            title="Delete connection"
            onclick={(e) => { e.stopPropagation(); requestDeleteProfile(conn); }}
          >
            <IconTrash size={12} />
          </button>
        </span>
        <span class="font-mono text-[10px] text-fg-faint uppercase group-hover:hidden">{conn.protocol}</span>
      </div>
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
        onclick={() => openConnection(conn)}
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

<Modal
  open={deletePending !== null}
  center
  title="Delete {deletePending?.name ?? 'connection'}?"
  description="Removes the saved connection. A stored keychain password for it is kept until you delete it there."
  width="sm"
>
  <p class="text-sm text-fg-muted">
    {deletePending?.protocol.toUpperCase()} · {deletePending?.username}@{deletePending?.host}:{deletePending?.port}
  </p>
  {#snippet footer()}
    <Button variant="ghost" onclick={() => (deletePending = null)}>Cancel</Button>
    <Button variant="danger" onclick={confirmDeleteProfile}>Delete</Button>
  {/snippet}
</Modal>
