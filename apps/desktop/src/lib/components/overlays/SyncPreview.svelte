<script lang="ts">
  import { app } from '$lib/stores/app.svelte';
  import { cn, formatBytes } from '$lib/utils';
  import { syncExecute, syncPreview } from '$lib/ipc';
  import type { SyncDiff, SyncDirection } from '$lib/types';
  import Modal from '$lib/components/ui/modal.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import Badge from '$lib/components/ui/badge.svelte';
  import {
    IconRefresh,
    IconUpload,
    IconDownload,
    IconCheck,
    IconHardDrive,
    IconCloud,
    IconArrowRight,
    IconAlert,
    IconLoader
  } from '$lib/components/icons';

  let direction = $state<SyncDirection>('both');
  let diffs = $state<SyncDiff[]>([]);
  let loading = $state(false);
  let errorMessage = $state<string | null>(null);
  let executing = $state(false);

  const hasConnection = $derived(app.activeConnectionId !== null);

  // Re-fetch the plan whenever the modal opens or the direction changes.
  $effect(() => {
    if (!app.syncOpen) return;
    const connectionId = app.activeConnectionId;
    const localDir = app.localPath;
    const remoteDir = app.remotePath;
    const dir = direction;
    if (!connectionId) return;

    loading = true;
    errorMessage = null;
    syncPreview({ connectionId, localDir, remoteDir, direction: dir })
      .then((plan) => (diffs = plan))
      .catch((e: Error) => (errorMessage = e.message))
      .finally(() => (loading = false));
  });

  const executable = $derived(diffs.filter((d) => d.reason !== 'conflict'));
  const totals = $derived({
    uploads: executable.filter((d) => d.direction === 'upload'),
    downloads: executable.filter((d) => d.direction === 'download'),
    conflicts: diffs.filter((d) => d.reason === 'conflict'),
    bytes: executable.reduce((sum, d) => sum + d.size, 0)
  });

  function reasonMeta(r: SyncDiff['reason']) {
    switch (r) {
      case 'newer': return { label: 'Newer', tone: 'info' as const };
      case 'missing': return { label: 'Missing', tone: 'warning' as const };
      case 'larger': return { label: 'Larger', tone: 'neutral' as const };
      case 'conflict': return { label: 'Conflict', tone: 'danger' as const };
    }
  }

  /** Execute the reviewed plan (conflicts are skipped server-side). */
  async function runSync() {
    if (!app.activeConnectionId || executing) return;
    executing = true;
    errorMessage = null;
    try {
      await syncExecute({
        connectionId: app.activeConnectionId,
        localRoot: app.localPath,
        remoteRoot: app.remotePath,
        diffs
      });
      app.syncOpen = false;
    } catch (e) {
      errorMessage = e instanceof Error ? e.message : String(e);
    } finally {
      executing = false;
    }
  }
</script>

<Modal
  bind:open={app.syncOpen}
  title="Synchronize Folders"
  description="Compare local and remote trees and reconcile the differences."
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
    <span class="truncate font-mono text-fg">{app.localPath}</span>
    <IconArrowRight size={13} class="shrink-0 text-fg-faint" />
    <IconCloud size={14} class="shrink-0 text-accent" />
    <span class="truncate font-mono text-fg">{app.remotePath}</span>
  </div>

  <!-- Summary stats -->
  <div class="mb-3 grid grid-cols-4 gap-2">
    <div class="rounded-lg border border-border bg-bg-panel p-2.5">
      <div class="text-[10px] uppercase tracking-wider text-fg-subtle">Uploads</div>
      <div class="mt-0.5 flex items-baseline gap-1">
        <span class="text-lg font-semibold text-accent">{totals.uploads.length}</span>
      </div>
    </div>
    <div class="rounded-lg border border-border bg-bg-panel p-2.5">
      <div class="text-[10px] uppercase tracking-wider text-fg-subtle">Downloads</div>
      <div class="mt-0.5 flex items-baseline gap-1">
        <span class="text-lg font-semibold text-success">{totals.downloads.length}</span>
      </div>
    </div>
    <div class="rounded-lg border border-border bg-bg-panel p-2.5">
      <div class="text-[10px] uppercase tracking-wider text-fg-subtle">Conflicts</div>
      <div class="mt-0.5 flex items-baseline gap-1">
        <span class={cn('text-lg font-semibold', totals.conflicts.length > 0 ? 'text-danger' : 'text-fg')}>
          {totals.conflicts.length}
        </span>
      </div>
    </div>
    <div class="rounded-lg border border-border bg-bg-panel p-2.5">
      <div class="text-[10px] uppercase tracking-wider text-fg-subtle">Total</div>
      <div class="mt-0.5 text-lg font-semibold text-fg">{formatBytes(totals.bytes)}</div>
    </div>
  </div>

  <!-- Diff list -->
  <div class="max-h-72 overflow-y-auto rounded-lg border border-border">
    {#if loading}
      <div class="flex items-center justify-center gap-2 p-8 text-sm text-fg-subtle">
        <IconLoader size={16} class="animate-spin" /> Comparing folders…
      </div>
    {:else if errorMessage}
      <div class="flex items-start gap-2 p-4 text-sm text-danger">
        <IconAlert size={15} class="mt-0.5 shrink-0" />
        {errorMessage}
      </div>
    {:else if diffs.length === 0}
      <div class="p-8 text-center text-sm text-fg-subtle">
        Folders are in sync — nothing to do.
      </div>
    {:else}
      {#each diffs as d (d.path)}
        {@const rm = reasonMeta(d.reason)}
        {@const Icon = d.direction === 'upload' ? IconUpload : IconDownload}
        <div class="flex items-center gap-3 border-b border-border px-3 py-2 text-sm last:border-0" class:opacity-60={d.reason === 'conflict'}>
          <Icon size={13} class={cn(d.direction === 'upload' ? 'text-accent' : 'text-success')} />
          <span class="flex-1 truncate font-mono text-xs text-fg">{d.path}</span>
          <Badge variant={rm.tone}>{rm.label}</Badge>
          <span class="w-20 text-right font-mono text-xs text-fg-subtle">{formatBytes(d.size)}</span>
        </div>
      {/each}
    {/if}
  </div>

  {#if totals.conflicts.length > 0}
    <p class="mt-2 flex items-center gap-1.5 text-[11px] text-fg-subtle">
      <IconAlert size={11} class="text-danger" />
      Conflicts are skipped — resolve them by transferring the file manually.
    </p>
  {/if}

  {#snippet footer()}
    <Button variant="ghost" onclick={() => (app.syncOpen = false)}>Cancel</Button>
    <Button
      onclick={runSync}
      disabled={loading || executing || executable.length === 0 || !hasConnection}
    >
      {#if executing}
        <IconLoader size={14} class="animate-spin" /> Starting…
      {:else}
        <IconCheck size={14} /> Sync {executable.length} change{executable.length === 1 ? '' : 's'}
      {/if}
    </Button>
  {/snippet}
</Modal>
