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
    IconUpload,
    IconDownload,
  } from '$lib/components/icons';
  import Button from '$lib/components/ui/button.svelte';
  import Tooltip from '$lib/components/ui/tooltip.svelte';

  let {
    side,
    title,
    files,
    path,
    selected,
    onSelect,
    showHidden = false,
    connectionName,
    onNavigate,
    onNavigateTo,
    onUpload,
    onDownload,
    onEdit,
    onRefresh,
    disconnected = false,
    onConnect,
    onCreateFolder,
    onRename,
    onDelete,
    error = null,
    onDropLocalFiles,
    onDropRemoteEntries,
    externalDragOver = false
  }: {
    side: 'local' | 'remote';
    title: string;
    files: RemoteFile[];
    path: string;
    selected: Set<string>;
    onSelect: (name: string, additive?: boolean) => void;
    showHidden?: boolean;
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
    /** Create a directory in this pane. */
    onCreateFolder?: (name: string) => void;
    /** Rename an entry in this pane. */
    onRename?: (from: string, to: string) => void;
    /** Delete the given entries in this pane. */
    onDelete?: (names: string[]) => void;
    /** Pane-level error surfaced by the data layer, null when healthy. */
    error?: string | null;
    /** Local files dropped onto this pane (remote side uploads them). */
    onDropLocalFiles?: (files: { localPath: string; fileName: string }[], targetDir: string) => void;
    /** Remote entries dropped onto this pane (local side downloads them). */
    onDropRemoteEntries?: (entries: { name: string; isDir: boolean }[], targetDir: string) => void;
    /** External drag hovering (Finder drop onto the remote pane). */
    externalDragOver?: boolean;
  } = $props();


  // ── Drag & drop ───────────────────────────────────────────────────────────
  const LOCAL_MIME = 'application/x-flowftp-local';
  const REMOTE_MIME = 'application/x-flowftp-remote';

  /** Pane-level highlight while a compatible drag hovers. */
  let paneDragOver = $state(false);
  /** Directory row currently highlighted as the drop target. */
  let rowDragOver = $state<string | null>(null);
  /** Depth counter: child dragleave events fire before the real pane exit. */
  let dragDepth = 0;

  function dragFromOtherPane(e: DragEvent): boolean {
    const types = e.dataTransfer?.types ?? [];
    if (isLocal) return types.includes(REMOTE_MIME);
    return types.includes(LOCAL_MIME) || types.includes('Files');
  }

  function onPaneDragEnter(e: DragEvent) {
    if (!dragFromOtherPane(e) || disconnected) return;
    dragDepth += 1;
    paneDragOver = true;
  }

  function onPaneDragLeave() {
    dragDepth = Math.max(0, dragDepth - 1);
    if (dragDepth === 0) {
      paneDragOver = false;
      rowDragOver = null;
    }
  }

  /** Accept a drop: route to the directory row if one is targeted, else the pane root. */
  function onPaneDrop(e: DragEvent) {
    dragDepth = 0;
    const wasOver = paneDragOver;
    paneDragOver = false;
    const targetRow = rowDragOver;
    rowDragOver = null;
    if (disconnected || !wasOver) return;
    e.preventDefault();

    const targetDir =
      targetRow && targetRow !== '..'
        ? path === '/'
          ? `/${targetRow}`
          : `${path.replace(/\/+$/, '')}/${targetRow}`
        : path;

    const localData = e.dataTransfer?.getData(LOCAL_MIME);
    const remoteData = e.dataTransfer?.getData(REMOTE_MIME);

    // Remote pane receives local files -> upload.
    if (!isLocal && localData) {
      try {
        const entries = JSON.parse(localData) as { name: string; localPath: string; isDir?: boolean }[];
        onDropLocalFiles?.(
          entries.filter((entry) => !entry.isDir).map((entry) => ({ localPath: entry.localPath, fileName: entry.name })),
          targetDir
        );
      } catch {
        // malformed payload: ignore
      }
      return;
    }

    // Local pane receives remote entries -> download.
    if (isLocal && remoteData) {
      try {
        const entries = JSON.parse(remoteData) as { name: string; isDir: boolean }[];
        onDropRemoteEntries?.(
          entries.filter((entry) => !entry.isDir),
          targetDir
        );
      } catch {
        // malformed payload: ignore
      }
    }
  }

  function rowDragStart(e: DragEvent, file: RemoteFile) {
    const entry = { name: file.name, isDir: file.kind === 'directory' };
    const payload = JSON.stringify([entry]);
    if (isLocal) {
      const base = path === '/' ? '' : path.replace(/\/+$/, '');
      const withPaths = JSON.stringify([
        { ...entry, localPath: `${base}/${file.name}` }
      ]);
      e.dataTransfer?.setData(LOCAL_MIME, withPaths);
    } else {
      e.dataTransfer?.setData(REMOTE_MIME, payload);
    }
    e.dataTransfer!.effectAllowed = 'copy';
  }

  function rowDragOverFolder(e: DragEvent, name: string) {
    if (name === '..') return;
    if (!dragFromOtherPane(e)) return;
    e.preventDefault();
    e.stopPropagation();
    rowDragOver = name;
  }

  function rowDropFolder(e: DragEvent, name: string) {
    if (name === '..' || !dragFromOtherPane(e)) return;
    e.preventDefault();
    e.stopPropagation();
    // Reuse the pane handler with the row targeted.
    const previous = rowDragOver;
    rowDragOver = name;
    onPaneDrop(e);
    if (previous !== null) rowDragOver = previous;
  }

  // ── Inline file management state ──────────────────────────────────────────
  /** Name being renamed in place (row shows an input). */
  let renaming = $state<string | null>(null);
  let renameValue = $state('');
  /** A new-folder input row is showing at the top of the list. */
  let creatingFolder = $state(false);
  let newFolderName = $state('');

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
    (paneDragOver || externalDragOver) && 'ring-2 ring-inset ring-accent bg-accent/5'
  )}
  aria-label='{title} files'
  ondragenter={onPaneDragEnter}
  ondragover={(e) => { if (dragFromOtherPane(e) && !disconnected) e.preventDefault(); }}
  ondragleave={onPaneDragLeave}
  ondrop={onPaneDrop}
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
      <Tooltip label="Up">
        <Button variant="ghost" size="icon-sm" aria-label="Go to parent folder" onclick={() => onNavigate?.('..')}>
          <IconHome size={14} />
        </Button>
      </Tooltip>
      <Tooltip label="Refresh">
        <Button variant="ghost" size="icon-sm" aria-label="Refresh" onclick={onRefresh}>
          <IconRefresh size={14} />
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
  <div class="min-h-0 flex-1 overflow-y-auto py-0.5">
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
          rowDragOver === f.name && 'bg-accent/10 ring-1 ring-inset ring-accent'
        )}
        draggable={isLocal && f.name !== '..'}
        ondragstart={(e) => rowDragStart(e, f)}
        onclick={(e) => onSelect(f.name, e.metaKey || e.ctrlKey || e.shiftKey)}
        ondblclick={() => {
          if (f.kind === 'directory') onNavigate?.(f.name);
          else if (!isLocal) onEdit?.(f.name);
        }}
        ondragover={(e) => f.kind === 'directory' && rowDragOverFolder(e, f.name)}
        ondragleave={() => { if (rowDragOver === f.name) rowDragOver = null; }}
        ondrop={(e) => f.kind === 'directory' && rowDropFolder(e, f.name)}
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
