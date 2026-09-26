<script lang="ts">
  import { cn, formatBytes, formatDate } from '$lib/utils';
  import type { RemoteFile } from '$lib/types';
  import {
    IconFolder,
    IconFile,
    IconFileText,
    IconImage,
    IconArchive,
    IconHardDrive,
    IconCloud,
    IconChevronRight,
    IconHome,
    IconRefresh,
    IconPlus,
    IconPencil,
    IconFolderPlus,
    IconTrash,
    IconPlug,
    IconZap,
    IconUnplug,
    IconUpload,
    IconDownload,
    IconLoader,
  } from '$lib/components/icons';
  import Button from '$lib/components/ui/button.svelte';
  import Modal from '$lib/components/ui/modal.svelte';
  import Tooltip from '$lib/components/ui/tooltip.svelte';

  let {
    side,
    title,
    files,
    path,
    selected,
    onSelect,
    showHidden = false,
    loading = false,
    connectionName,
    onNavigate,
    onNavigateTo,
    onUpload,
    onDownload,
    onEdit,
    onRefresh,
    disconnected = false,
    onConnect,
    onDisconnect,
    onCreateFolder,
    onRename,
    onDelete,
    error = null,
    externalDragOver = false,
    internalDropTarget = null
  }: {
    side: 'local' | 'remote';
    title: string;
    files: RemoteFile[];
    path: string;
    selected: Set<string>;
    onSelect: (name: string, additive?: boolean) => void;
    showHidden?: boolean;
    /** A listing request is in flight: hold the current rows, show a spinner. */
    loading?: boolean;
    connectionName?: string;
    /** Double-click a directory row -> navigate into it. */
    onNavigate?: (name: string) => void;
    /** Click a breadcrumb segment -> navigate to that absolute path. */
    onNavigateTo?: (path: string) => void;
    /** Footer Upload button (local pane). */
    onUpload?: () => void;
    /** Footer Download button (remote pane). */
    onDownload?: () => void;
    /** Double-click a remote file -> open it for editing. */
    onEdit?: (name: string) => void;
    /** Refresh this pane's listing. */
    onRefresh?: () => void;
    /** True when the pane's source is unavailable (remote, not connected). */
    disconnected?: boolean;
    /** Open the connect flow (disconnected empty state CTA). */
    onConnect?: () => void;
    /** Tear down the remote session (remote pane, connected). */
    onDisconnect?: () => void;
    /** Create a directory in this pane. */
    onCreateFolder?: (name: string) => void;
    /** Rename an entry in this pane. */
    onRename?: (from: string, to: string) => void;
    /** Delete the given entries in this pane. */
    onDelete?: (names: string[]) => void;
    /** Pane-level error surfaced by the data layer, null when healthy. */
    error?: string | null;
    /** External drag hovering (Finder drop onto the remote pane). */
    externalDragOver?: boolean;
    /** Internal file drag destination, coordinated by the page. */
    internalDropTarget?: { side: 'local' | 'remote'; folder: string | null } | null;
  } = $props();

  // ── Inline file management state ──────────────────────────────────────────
  /** Name being renamed in place (row shows an input). */
  let renaming = $state<string | null>(null);
  let renameValue = $state('');
  /** A new-folder input row is showing at the top of the list. */
  let creatingFolder = $state(false);
  let newFolderName = $state('');
  let disconnectConfirmOpen = $state(false);

  function confirmDisconnect() {
    disconnectConfirmOpen = false;
    onDisconnect?.();
  }

  function startRename(name: string) {
    renaming = name;
    renameValue = name;
  }

  function commitRename() {
    if (renaming && onRename && renameValue.trim() && renameValue !== renaming) {
      onRename(renaming, renameValue.trim());
    }
    renaming = null;
  }

  function commitNewFolder() {
    if (creatingFolder && onCreateFolder && newFolderName.trim()) {
      onCreateFolder(newFolderName.trim());
    }
    creatingFolder = false;
    newFolderName = '';
  }

  function deleteSelection() {
    const names = [...selected].filter((n) => n !== '..');
    if (names.length > 0) onDelete?.(names);
  }

  const selectedNames = $derived([...selected].filter((n) => n !== '..'));

  const isLocal = $derived(side === 'local');
  const visibleFiles = $derived(
    showHidden ? files : files.filter((f) => !f.name.startsWith('.') || f.name === '..')
  );

  // Sort state (applies to the rendered list; directories group first)
  let sortKey = $state<'name' | 'size' | 'modified'>('name');
  let sortDir = $state<'asc' | 'desc'>('asc');

  const sorted = $derived.by(() => {
    const dirs = visibleFiles.filter((f) => f.kind === 'directory' || f.name === '..');
    const others = visibleFiles.filter((f) => f.kind !== 'directory' && f.name !== '..');
    const cmp = (a: RemoteFile, b: RemoteFile) => {
      let r = 0;
      if (sortKey === 'name') r = a.name.localeCompare(b.name);
      else if (sortKey === 'size') r = a.size - b.size;
      else r = a.modified - b.modified;
      return sortDir === 'asc' ? r : -r;
    };
    return [...dirs.sort(cmp), ...others.sort(cmp)];
  });

  function toggleSort(k: 'name' | 'size' | 'modified') {
    if (sortKey === k) sortDir = sortDir === 'asc' ? 'desc' : 'asc';
    else {
      sortKey = k;
      sortDir = 'asc';
    }
  }

  function iconFor(f: RemoteFile) {
    if (f.kind === 'directory') return IconFolder;
    const ext = f.name.split('.').pop()?.toLowerCase();
    if (['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg'].includes(ext || '')) return IconImage;
    if (['zip', 'tar', 'gz', 'rar', '7z'].includes(ext || '')) return IconArchive;
    if (['md', 'txt', 'json', 'yaml', 'yml', 'toml', 'sh', 'conf'].includes(ext || ''))
      return IconFileText;
    return IconFile;
  }

  function iconColor(f: RemoteFile) {
    if (f.kind === 'directory') return 'text-accent-text';
    const ext = f.name.split('.').pop()?.toLowerCase();
    if (['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg'].includes(ext || '')) return 'text-info';
    if (['zip', 'tar', 'gz', 'rar', '7z'].includes(ext || '')) return 'text-warning';
    return 'text-fg-subtle';
  }

  // breadcrumb segments
  const segments = $derived(
    path.split('/').filter(Boolean).map((seg, i, arr) => ({
      label: seg,
      path: '/' + arr.slice(0, i + 1).join('/')
    }))
  );
</script>

<section
  class={cn(
    'flex min-h-0 min-w-0 flex-1 flex-col bg-bg-elevated transition-shadow',
    isLocal && 'border-r',
    (externalDragOver || (internalDropTarget?.side === side && !internalDropTarget.folder)) &&
      'ring-2 ring-inset ring-accent bg-accent/10 shadow-sm'
  )}
  aria-label='{title} files'
  data-file-pane={side}
  data-file-path={path}
>
  <!-- Pane header -->
  <div class="flex h-10 items-center gap-1 border-b border-border px-2.5">
    <div class={cn('grid h-5 w-5 place-items-center rounded', isLocal ? 'text-fg-muted' : 'text-accent-text')}>
      {#if isLocal}
        <IconHardDrive size={14} />
      {:else}
        <IconCloud size={14} />
      {/if}
    </div>
    <span class="text-xs font-semibold uppercase tracking-wider text-fg-muted">{title}</span>
    {#if connectionName}
      <span class="ml-1 truncate text-xs text-fg-subtle">· {connectionName}</span>
    {/if}

    <div class="ml-auto flex items-center gap-0.5">
      {#if !isLocal && onDisconnect && !disconnected}
        <Tooltip label="Disconnect" side="bottom">
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label="Disconnect"
            onclick={() => (disconnectConfirmOpen = true)}
          >
            <IconUnplug size={14} />
          </Button>
        </Tooltip>
      {/if}
      <Tooltip label="Up">
        <Button variant="ghost" size="icon-sm" aria-label="Go to parent folder" onclick={() => onNavigate?.('..')}>
          <IconHome size={14} />
        </Button>
      </Tooltip>
      <Tooltip label={loading ? 'Loading…' : 'Refresh'} side="bottom">
        <Button variant="ghost" size="icon-sm" aria-label="Refresh" disabled={loading} onclick={onRefresh}>
          {#if loading}
            <IconLoader size={14} class="animate-spin" />
          {:else}
            <IconRefresh size={14} />
          {/if}
        </Button>
      </Tooltip>
      <Tooltip label="New folder">
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label="New folder"
          onclick={() => { creatingFolder = true; newFolderName = ''; }}
        >
          <IconFolderPlus size={14} />
        </Button>
      </Tooltip>
      <Tooltip label="Rename">
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label="Rename selected"
          disabled={selectedNames.length !== 1}
          onclick={() => selectedNames[0] && startRename(selectedNames[0])}
        >
          <IconPencil size={14} />
        </Button>
      </Tooltip>
      <Tooltip label="Delete">
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label="Delete selected"
          disabled={selectedNames.length === 0}
          onclick={deleteSelection}
        >
          <IconTrash size={14} />
        </Button>
      </Tooltip>

    </div>
  </div>

  <!-- Breadcrumb -->
  <div class="flex h-8 items-center gap-0.5 overflow-hidden border-b border-border px-2.5 text-xs text-fg-subtle">
    <button
      class="rounded px-1.5 py-0.5 hover:bg-bg-hover hover:text-fg"
      onclick={() => onNavigateTo?.('/')}
      title="Go to root"
    >
      <IconHome size={12} />
    </button>
    {#each segments as seg, i}
      <IconChevronRight size={11} class="text-fg-faint" />
      <button
        class="rounded px-1.5 py-0.5 hover:bg-bg-hover hover:text-fg"
        onclick={() => onNavigateTo?.(seg.path)}
      >
        {seg.label}
      </button>
    {/each}
  </div>

  <!-- Column headers -->
  <div class="flex h-7 items-center gap-2 border-b border-border px-3 text-[11px] font-semibold uppercase tracking-wider text-fg-subtle">
    <button
      class="flex flex-1 items-center gap-1 hover:text-fg"
      onclick={() => toggleSort('name')}
    >
      Name
      {#if sortKey === 'name'}<span>{sortDir === 'asc' ? '↑' : '↓'}</span>{/if}
    </button>
    <button
      class="flex w-20 items-center justify-end gap-1 hover:text-fg"
      onclick={() => toggleSort('size')}
    >
      Size
      {#if sortKey === 'size'}<span>{sortDir === 'asc' ? '↑' : '↓'}</span>{/if}
    </button>
    <button
      class="flex w-28 items-center justify-end gap-1 hover:text-fg"
      onclick={() => toggleSort('modified')}
    >
      Modified
      {#if sortKey === 'modified'}<span>{sortDir === 'asc' ? '↑' : '↓'}</span>{/if}
    </button>
    <span class="w-16 text-right">Perm</span>
  </div>

  <!-- File rows -->
  <div
    class="min-h-0 flex-1 overflow-y-auto py-0.5 transition-opacity"
    class:opacity-60={loading}
    class:pointer-events-none={loading}
    aria-busy={loading}
  >
    {#if creatingFolder}
      <div class="flex w-full items-center gap-2 bg-bg-active px-3 py-1">
        <IconFolder size={15} class="shrink-0 text-accent-text" />
        <!-- svelte-ignore a11y_autofocus -->
        <input
          class="flex-1 bg-transparent text-sm text-fg outline-none placeholder:text-fg-faint"
          placeholder="Folder name"
          bind:value={newFolderName}
          onkeydown={(e) => {
            if (e.key === 'Enter') commitNewFolder();
            else if (e.key === 'Escape') { creatingFolder = false; newFolderName = ''; }
          }}
          onblur={commitNewFolder}
          autofocus
        />
      </div>
    {/if}
    {#each sorted as f (f.name)}
      {@const isSel = selected.has(f.name)}
      {@const Icon = iconFor(f)}
      <button
        class={cn(
          'group flex w-full items-center gap-2 px-3 py-1 text-left text-sm transition-colors',
          isSel ? 'bg-bg-active text-fg' : 'text-fg-muted hover:bg-bg-hover',
          internalDropTarget?.side === side && internalDropTarget.folder === f.name &&
            'bg-accent/20 text-fg ring-2 ring-inset ring-accent/70 shadow-sm',
          'select-none'
        )}
        data-drag-entry={f.name !== '..' && f.kind !== 'directory' ? f.name : undefined}
        data-drop-folder={f.name !== '..' && f.kind === 'directory' ? f.name : undefined}
        onclick={(e) => onSelect(f.name, e.metaKey || e.ctrlKey || e.shiftKey)}
        ondblclick={() => {
          if (f.kind === 'directory') onNavigate?.(f.name);
          else if (!isLocal) onEdit?.(f.name);
        }}
        title={
          f.kind === 'directory'
            ? 'Double-click to open'
            : isLocal
              ? undefined
              : 'Double-click to edit'
        }
      >
        <Icon size={15} class={cn('shrink-0', iconColor(f))} />
        {#if renaming === f.name}
          <!-- svelte-ignore a11y_autofocus -->
          <input
            class="flex-1 rounded border border-accent bg-bg px-1.5 py-0.5 text-sm text-fg outline-none"
            bind:value={renameValue}
            onkeydown={(e) => {
              if (e.key === 'Enter') commitRename();
              else if (e.key === 'Escape') renaming = null;
            }}
            onblur={commitRename}
            autofocus
          />
        {:else}
          <span class={cn('min-w-0 flex-1 truncate', f.kind === 'directory' && 'font-medium')}>
            {f.name}
          </span>
        {/if}
        <span class="w-20 shrink-0 text-right font-mono text-xs text-fg-subtle">
          {f.kind === 'directory' ? '—' : formatBytes(f.size)}
        </span>
        <span class="w-24 shrink-0 text-right text-xs text-fg-subtle">
          {f.modified ? formatDate(f.modified) : '—'}
        </span>
        <span class="hidden min-[1280px]:block w-16 shrink-0 text-right font-mono text-[11px] text-fg-subtle">
          {f.permissions ?? '—'}
        </span>
      </button>
    {/each}

    {#if disconnected}
      <div class="flex h-full flex-col items-center justify-center gap-3 py-12 text-fg-subtle">
        <IconPlug size={28} class="opacity-40" />
        <p class="text-sm font-medium text-fg-muted">Not connected</p>
        <p class="max-w-48 text-center text-xs">Connect to a server to browse its files.</p>
        <Button variant="default" size="sm" onclick={onConnect}>
          <IconZap size={13} /> Connect
        </Button>
      </div>
    {:else if loading && sorted.length === 0}
      <div class="flex h-full flex-col items-center justify-center gap-3 py-12 text-fg-subtle">
        <IconLoader size={22} class="animate-spin opacity-60" />
        <p class="text-sm">Loading…</p>
      </div>
    {:else if sorted.length === 0}
      <div class="flex h-full flex-col items-center justify-center gap-2 py-12 text-fg-faint">
        <IconFolder size={28} class="opacity-40" />
        <p class="text-sm">Empty directory</p>
      </div>
    {/if}
  </div>

  <!-- Action footer -->
  <div class="flex h-9 items-center gap-1.5 border-t border-border bg-bg/40 px-2.5">
    {#if error}
      <span class="min-w-0 flex-1 truncate text-xs text-danger" title={error}>{error}</span>
    {:else if loading}
      <span class="min-w-0 flex-1 truncate text-xs text-fg-subtle">Loading…</span>
    {:else if selectedNames.length > 0}
      <span class="min-w-0 flex-1 truncate text-xs text-fg-subtle">
        {selectedNames.length} item{selectedNames.length === 1 ? '' : 's'} selected
      </span>
    {/if}
    {#if isLocal}
      <Button variant="default" size="sm" class="ml-auto shrink-0" onclick={() => onUpload?.()}>
        <IconUpload size={13} /> Upload
      </Button>
    {:else}
      <Button variant="default" size="sm" class="ml-auto shrink-0" onclick={() => onDownload?.()}>
        <IconDownload size={13} /> Download
      </Button>
    {/if}
  </div>
</section>

<Modal bind:open={disconnectConfirmOpen} title="Disconnect from server?" center width="sm">
  <p class="text-sm leading-relaxed text-fg-muted">
    This will end the active session with {connectionName ?? 'this server'}. You can reconnect at any time.
  </p>
  {#snippet footer()}
    <Button variant="ghost" onclick={() => (disconnectConfirmOpen = false)}>Cancel</Button>
    <Button variant="danger" onclick={confirmDisconnect}>Disconnect</Button>
  {/snippet}
</Modal>
