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
    IconArrowLeft,
    IconRefresh,
    IconMoreHorizontal,
    IconUpload,
    IconDownload,
    IconList,
    IconGrid
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
    error = null
  }: {
    side: 'local' | 'remote';
    title: string;
    files: RemoteFile[];
    path: string;
    selected: Set<string>;
    onSelect: (name: string, additive?: boolean) => void;
    showHidden?: boolean;
    connectionName?: string;
    /** Double-click a directory row → navigate into it. */
    onNavigate?: (name: string) => void;
    /** Click a breadcrumb segment → navigate to that absolute path. */
    onNavigateTo?: (path: string) => void;
    /** Footer Upload button (local pane). */
    onUpload?: () => void;
    /** Footer Download button (remote pane). */
    onDownload?: () => void;
    /** Pane-level error surfaced by the data layer, null when healthy. */
    error?: string | null;
  } = $props();

  const isLocal = $derived(side === 'local');
  const visibleFiles = $derived(
    showHidden ? files : files.filter((f) => !f.name.startsWith('.') || f.name === '..')
  );

  // Header sort state (mock — sorts by name asc by default)
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
    if (f.kind === 'directory') return 'text-accent';
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
  class="flex min-h-0 flex-1 flex-col border-border bg-bg-elevated"
  class:border-r={isLocal}
>
  <!-- Pane header -->
  <div class="flex h-10 items-center gap-1 border-b border-border px-2.5">
    <div class={cn('grid h-5 w-5 place-items-center rounded', isLocal ? 'text-fg-muted' : 'text-accent')}>
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
      <Tooltip label="Back">
        <Button variant="ghost" size="icon-sm"><IconArrowLeft size={14} /></Button>
      </Tooltip>
      <Tooltip label="Up">
        <Button variant="ghost" size="icon-sm"><IconHome size={14} /></Button>
      </Tooltip>
      <Tooltip label="Refresh">
        <Button variant="ghost" size="icon-sm"><IconRefresh size={14} /></Button>
      </Tooltip>
      <Tooltip label="More">
        <Button variant="ghost" size="icon-sm"><IconMoreHorizontal size={14} /></Button>
      </Tooltip>
    </div>
  </div>

  <!-- Breadcrumb -->
  <div class="flex h-8 items-center gap-0.5 border-b border-border px-2.5 text-xs text-fg-subtle">
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
  <div class="flex h-7 items-center gap-2 border-b border-border px-3 text-[10px] font-semibold uppercase tracking-wider text-fg-subtle">
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
    {#each sorted as f (f.name)}
      {@const isSel = selected.has(f.name)}
      {@const Icon = iconFor(f)}
      <button
        class={cn(
          'group flex w-full items-center gap-2 px-3 py-1 text-left text-sm transition-colors',
          isSel ? 'bg-bg-active text-fg' : 'text-fg-muted hover:bg-bg-hover'
        )}
        onclick={(e) => onSelect(f.name, e.metaKey || e.ctrlKey || e.shiftKey)}
        ondblclick={() => {
          if (f.kind === 'directory') onNavigate?.(f.name);
        }}
        title={f.kind === 'directory' ? 'Double-click to open' : undefined}
      >
        <Icon size={15} class={cn('shrink-0', iconColor(f))} />
        <span class={cn('flex-1 truncate', f.kind === 'directory' && 'font-medium')}>
          {f.name}
        </span>
        <span class="w-20 shrink-0 text-right font-mono text-xs text-fg-subtle">
          {f.kind === 'directory' ? '—' : formatBytes(f.size)}
        </span>
        <span class="w-28 shrink-0 text-right text-xs text-fg-subtle">
          {f.modified ? formatDate(f.modified) : '—'}
        </span>
        <span class="w-16 shrink-0 text-right font-mono text-[10px] text-fg-faint">
          {f.permissions ?? '—'}
        </span>
      </button>
    {/each}

    {#if sorted.length === 0}
      <div class="flex h-full flex-col items-center justify-center gap-2 py-12 text-fg-faint">
        <IconFolder size={28} class="opacity-40" />
        <p class="text-sm">Empty directory</p>
      </div>
    {/if}
  </div>

  <!-- Action footer -->
  <div class="flex h-9 items-center gap-1.5 border-t border-border bg-bg/40 px-2.5">
    {#if error}
      <span class="truncate text-xs text-danger" title={error}>{error}</span>
    {/if}
    {#if isLocal}
      <Button variant="default" size="sm" class="ml-auto" onclick={() => onUpload?.()}>
        <IconUpload size={13} /> Upload
      </Button>
    {:else}
      <Button variant="default" size="sm" class="ml-auto" onclick={() => onDownload?.()}>
        <IconDownload size={13} /> Download
      </Button>
    {/if}
  </div>
</section>
