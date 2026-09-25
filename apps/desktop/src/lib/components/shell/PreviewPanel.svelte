<script lang="ts">
  import { app } from '$lib/stores/app.svelte';
  import { formatBytes, formatDate } from '$lib/utils';
  import type { RemoteFile } from '$lib/types';
  import {
    IconImage,
    IconFile,
    IconFileText,
    IconArchive,
    IconInfo,
    IconFolder,
    IconHardDrive,
    IconCloud
  } from '$lib/components/icons';
  import Badge from '$lib/components/ui/badge.svelte';

  /** The inspected entry — resolved live from the pane listing. */
  const entry = $derived.by<RemoteFile | null>(() => {
    const selection = app.inspector;
    if (!selection) return null;
    const files = selection.side === 'local' ? app.localFiles : app.remoteFiles;
    return files.find((f) => f.name === selection.name) ?? null;
  });

  const isRemote = $derived(app.inspector?.side === 'remote');

  function kindMeta(entry: RemoteFile) {
    if (entry.kind === 'directory') return { Icon: IconFolder, label: 'Folder', tone: 'neutral' as const };
    const ext = entry.name.split('.').pop()?.toLowerCase() ?? '';
    if (['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg'].includes(ext))
      return { Icon: IconImage, label: 'Image', tone: 'info' as const };
    if (['zip', 'tar', 'gz', 'rar', '7z'].includes(ext))
      return { Icon: IconArchive, label: 'Archive', tone: 'warning' as const };
    if (['md', 'txt', 'json', 'yaml', 'yml', 'toml', 'sh', 'conf', 'log'].includes(ext))
      return { Icon: IconFileText, label: 'Text', tone: 'neutral' as const };
    return { Icon: IconFile, label: 'File', tone: 'neutral' as const };
  }
</script>

<aside class="flex w-72 flex-col border-l border-border bg-bg-elevated">
  <!-- Header -->
  <div class="flex h-10 items-center gap-2 border-b border-border px-3">
    <span class="text-xs font-semibold uppercase tracking-wider text-fg-muted">
      {#if isRemote}Remote{:else}Local{/if}
    </span>
  </div>

  {#if entry}
    {@const km = kindMeta(entry)}
    <div class="flex-1 overflow-y-auto p-4">
      <!-- Preview surface -->
      <div class="relative mb-4 aspect-[4/3] overflow-hidden rounded-lg border border-border bg-bg-panel">
        <div class="absolute inset-0 bg-gradient-to-br from-accent/20 via-bg-panel to-info/10"></div>
        <div class="absolute inset-0 grid place-items-center">
          <km.Icon size={48} class="text-fg-subtle" />
        </div>
        <div class="absolute left-2 top-2">
          <Badge variant={km.tone}>{km.label}</Badge>
        </div>
      </div>

      <!-- Filename -->
      <div class="mb-3">
        <h3 class="truncate text-sm font-semibold text-fg" title={entry.name}>{entry.name}</h3>
        <p class="mt-0.5 text-xs text-fg-subtle">
          {entry.kind === 'directory' ? '—' : formatBytes(entry.size)}
          · {entry.modified ? formatDate(entry.modified) : 'unknown date'}
        </p>
      </div>

      <!-- Metadata -->
      <div class="space-y-1.5">
        <div class="mb-2 flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-wider text-fg-subtle">
          <IconInfo size={11} /> Details
        </div>
        <div class="flex items-baseline gap-2 text-xs">
          <span class="w-24 shrink-0 text-fg-subtle">Kind</span>
          <span class="flex-1 truncate text-fg">{km.label}</span>
        </div>
        {#if entry.permissions}
          <div class="flex items-baseline gap-2 text-xs">
            <span class="w-24 shrink-0 text-fg-subtle">Permissions</span>
            <span class="flex-1 truncate font-mono text-fg">{entry.permissions}</span>
          </div>
        {/if}
        {#if entry.owner}
          <div class="flex items-baseline gap-2 text-xs">
            <span class="w-24 shrink-0 text-fg-subtle">Owner</span>
            <span class="flex-1 truncate text-fg">{entry.owner}</span>
          </div>
        {/if}
        {#if entry.group}
          <div class="flex items-baseline gap-2 text-xs">
            <span class="w-24 shrink-0 text-fg-subtle">Group</span>
            <span class="flex-1 truncate text-fg">{entry.group}</span>
          </div>
        {/if}
        <div class="flex items-baseline gap-2 text-xs">
          <span class="w-24 shrink-0 text-fg-subtle">Size</span>
          <span class="flex-1 truncate text-fg">
            {entry.kind === 'directory' ? '—' : `${formatBytes(entry.size)} (${entry.size.toLocaleString()} bytes)`}
          </span>
        </div>
        <div class="flex items-baseline gap-2 text-xs">
          <span class="w-24 shrink-0 text-fg-subtle">Modified</span>
          <span class="flex-1 truncate text-fg">{entry.modified ? formatDate(entry.modified) : '—'}</span>
        </div>
      </div>

      <!-- Location -->
      <div class="mt-4 flex items-center gap-2 rounded-lg border border-border bg-bg-panel p-3 text-xs">
        {#if isRemote}
          <IconCloud size={13} class="shrink-0 text-accent" />
          <span class="truncate font-mono text-fg-subtle">{app.remotePath}</span>
        {:else}
          <IconHardDrive size={13} class="shrink-0 text-fg-muted" />
          <span class="truncate font-mono text-fg-subtle">{app.localPath}</span>
        {/if}
      </div>
    </div>
  {:else}
    <!-- Empty state per DESIGN.md: never blank, always guidance -->
    <div class="flex flex-1 flex-col items-center justify-center gap-2 p-6 text-center text-fg-faint">
      <IconInfo size={28} class="opacity-40" />
      <p class="text-sm text-fg-subtle">Nothing selected</p>
      <p class="text-xs">Select a file in either pane to inspect it.</p>
    </div>
  {/if}
</aside>
