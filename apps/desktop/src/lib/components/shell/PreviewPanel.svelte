<script lang="ts">
  import { mockPreview } from '$lib/mock';
  import { formatBytes, formatDate } from '$lib/utils';
  import {
    IconImage,
    IconFile,
    IconArchive,
    IconMaximize,
    IconLock,
    IconClock,
    IconInfo,
    IconStar
  } from '$lib/components/icons';
  import Badge from '$lib/components/ui/badge.svelte';

  const p = mockPreview;

  function kindMeta(kind: string) {
    switch (kind) {
      case 'image': return { Icon: IconImage, label: 'Image', tone: 'info' as const };
      case 'text': return { Icon: IconFile, label: 'Text', tone: 'neutral' as const };
      case 'archive': return { Icon: IconArchive, label: 'Archive', tone: 'warning' as const };
      default: return { Icon: IconFile, label: 'File', tone: 'neutral' as const };
    }
  }

  const km = kindMeta(p.kind);
</script>

<aside class="flex w-72 flex-col border-l border-border bg-bg-elevated">
  <!-- Header -->
  <div class="flex h-10 items-center gap-2 border-b border-border px-3">
    <span class="text-xs font-semibold uppercase tracking-wider text-fg-muted">Preview</span>
    <button class="ml-auto rounded p-1 text-fg-subtle hover:bg-bg-hover hover:text-fg">
      <IconMaximize size={13} />
    </button>
  </div>

  <div class="flex-1 overflow-y-auto p-4">
    <!-- Preview surface -->
    <div class="relative mb-4 aspect-[4/3] overflow-hidden rounded-lg border border-border bg-bg-panel">
      <!-- Faux preview: gradient placeholder -->
      <div class="absolute inset-0 bg-gradient-to-br from-accent/20 via-bg-panel to-info/10"></div>
      <div class="absolute inset-0 grid place-items-center">
        <km.Icon size={48} class="text-fg-subtle" />
      </div>
      <div class="absolute left-2 top-2">
        <Badge variant={km.tone}>{km.label}</Badge>
      </div>
      <button
        class="absolute right-2 top-2 rounded-md bg-bg/60 p-1 text-fg-subtle backdrop-blur hover:text-fg"
        title="Favorite"
      >
        <IconStar size={12} />
      </button>
    </div>

    <!-- Filename -->
    <div class="mb-3">
      <h3 class="truncate text-sm font-semibold text-fg">{p.name}</h3>
      <p class="mt-0.5 text-xs text-fg-subtle">{formatBytes(p.size)} · {formatDate(p.modified)}</p>
    </div>

    <!-- Metadata -->
    <div class="space-y-1.5">
      <div class="mb-2 flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-wider text-fg-subtle">
        <IconInfo size={11} /> Details
      </div>
      {#each Object.entries(p.meta) as [key, val]}
        <div class="flex items-baseline gap-2 text-xs">
          <span class="w-24 shrink-0 text-fg-subtle">{key}</span>
          <span class="flex-1 truncate text-fg">{val}</span>
        </div>
      {/each}
    </div>

    <!-- Sync status mock -->
    <div class="mt-4 rounded-lg border border-border bg-bg-panel p-3">
      <div class="flex items-center gap-2">
        <IconLock size={13} class="text-success" />
        <span class="text-xs font-semibold text-fg">Synced</span>
      </div>
      <p class="mt-1 flex items-center gap-1 text-[11px] text-fg-subtle">
        <IconClock size={10} /> Last synced 2 hours ago
      </p>
    </div>
  </div>
</aside>
