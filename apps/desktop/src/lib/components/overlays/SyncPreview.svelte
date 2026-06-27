<script lang="ts">
  import { app } from '$lib/stores/app.svelte';
  import { cn, formatBytes } from '$lib/utils';
  import type { SyncDiff, SyncDirection } from '$lib/types';
  import Modal from '$lib/components/ui/modal.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import Badge from '$lib/components/ui/badge.svelte';
  import {
    IconRefresh,
    IconUpload,
    IconDownload,
    IconCheck,
    IconAlert,
    IconHardDrive,
    IconCloud,
    IconArrowRight
  } from '$lib/components/icons';

  let direction = $state<SyncDirection>('both');
  let dryRun = $state(true);

  // Mock diff
  const diffs: SyncDiff[] = [
    { path: '/public/index.html', direction: 'upload', size: 14_200, reason: 'newer' },
    { path: '/assets/logo.svg', direction: 'upload', size: 8_400, reason: 'newer' },
    { path: '/logs/access.log', direction: 'download', size: 84_000_000, reason: 'larger' },
    { path: '/backups/db.sql.gz', direction: 'download', size: 1_200_000_000, reason: 'missing' },
    { path: '/config/secrets.yaml', direction: 'upload', size: 1_200, reason: 'conflict' }
  ];

  const filteredDiffs = $derived(
    direction === 'both'
      ? diffs
      : diffs.filter((d) =>
          direction === 'local-to-remote' ? d.direction === 'upload' : d.direction === 'download'
        )
  );

  const totals = $derived({
    uploads: filteredDiffs.filter((d) => d.direction === 'upload'),
    downloads: filteredDiffs.filter((d) => d.direction === 'download'),
    bytes: filteredDiffs.reduce((sum, d) => sum + d.size, 0)
  });

  function reasonMeta(r: SyncDiff['reason']) {
    switch (r) {
      case 'newer': return { label: 'Newer', tone: 'info' as const };
      case 'missing': return { label: 'Missing', tone: 'warning' as const };
      case 'larger': return { label: 'Larger', tone: 'neutral' as const };
      case 'conflict': return { label: 'Conflict', tone: 'danger' as const };
    }
  }
</script>

<Modal
  bind:open={app.syncOpen}
  title="Synchronize Folders"
  description="Compare local and remote folders and reconcile differences."
  width="xl"
>
  <!-- Direction tabs -->
  <div class="mb-3 flex items-center gap-1 rounded-lg border border-border bg-bg-panel p-1">
    {#each [{ id: 'both', label: 'Both ways', icon: IconRefresh }, { id: 'local-to-remote', label: 'Local → Remote', icon: IconUpload }, { id: 'remote-to-local', label: 'Remote → Local', icon: IconDownload }] as opt (opt.id)}
      <button
        class={cn(
          'flex flex-1 items-center justify-center gap-1.5 rounded-md py-1.5 text-xs font-medium transition-colors',
          direction === opt.id ? 'bg-bg-active text-fg shadow-sm' : 'text-fg-muted hover:text-fg'
        )}
        onclick={() => (direction = opt.id as SyncDirection)}
      >
        <opt.icon size={13} />
        {opt.label}
      </button>
    {/each}
  </div>

  <!-- Path summary -->
  <div class="mb-3 flex items-center gap-2 rounded-lg border border-border bg-bg-panel px-3 py-2 text-xs">
    <IconHardDrive size={14} class="text-fg-muted" />
    <span class="font-mono text-fg">~/Projects/site</span>
    <IconArrowRight size={13} class="text-fg-faint" />
    <IconCloud size={14} class="text-accent" />
    <span class="font-mono text-fg">{app.activeConnection?.host ?? '—'}:/var/www</span>
  </div>

  <!-- Summary stats -->
  <div class="mb-3 grid grid-cols-3 gap-2">
    <div class="rounded-lg border border-border bg-bg-panel p-2.5">
      <div class="text-[10px] uppercase tracking-wider text-fg-subtle">Uploads</div>
      <div class="mt-0.5 flex items-baseline gap-1">
        <span class="text-lg font-semibold text-accent">{totals.uploads.length}</span>
        <span class="text-[11px] text-fg-subtle">{formatBytes(totals.uploads.reduce((s, d) => s + d.size, 0))}</span>
      </div>
    </div>
    <div class="rounded-lg border border-border bg-bg-panel p-2.5">
      <div class="text-[10px] uppercase tracking-wider text-fg-subtle">Downloads</div>
      <div class="mt-0.5 flex items-baseline gap-1">
        <span class="text-lg font-semibold text-success">{totals.downloads.length}</span>
        <span class="text-[11px] text-fg-subtle">{formatBytes(totals.downloads.reduce((s, d) => s + d.size, 0))}</span>
      </div>
    </div>
    <div class="rounded-lg border border-border bg-bg-panel p-2.5">
      <div class="text-[10px] uppercase tracking-wider text-fg-subtle">Total</div>
      <div class="mt-0.5 text-lg font-semibold text-fg">{formatBytes(totals.bytes)}</div>
    </div>
  </div>

  <!-- Diff list -->
  <div class="max-h-72 overflow-y-auto rounded-lg border border-border">
    {#each filteredDiffs as d, i (d.path)}
      {@const rm = reasonMeta(d.reason)}
      {@const Icon = d.direction === 'upload' ? IconUpload : IconDownload}
      <div class="flex items-center gap-3 border-b border-border px-3 py-2 text-sm last:border-0">
        <Icon size={13} class={cn(d.direction === 'upload' ? 'text-accent' : 'text-success')} />
        <span class="flex-1 truncate font-mono text-xs text-fg">{d.path}</span>
        <Badge variant={rm.tone}>{rm.label}</Badge>
        <span class="w-20 text-right font-mono text-xs text-fg-subtle">{formatBytes(d.size)}</span>
      </div>
    {/each}
  </div>

  <!-- Dry run toggle -->
  <label class="mt-3 flex cursor-pointer items-center gap-2.5">
    <input type="checkbox" bind:checked={dryRun} class="h-4 w-4 accent-[var(--color-accent)]" />
    <span class="text-sm text-fg-muted">Dry run — preview only, don't transfer</span>
  </label>

  {#snippet footer()}
    <Button variant="ghost" onclick={() => (app.syncOpen = false)}>Cancel</Button>
    <Button variant="outline" onclick={() => (app.syncOpen = false)}>
      <IconCheck size={14} /> Apply
    </Button>
    <Button onclick={() => (app.syncOpen = false)}>
      <IconRefresh size={14} /> Sync Now
    </Button>
  {/snippet}
</Modal>
