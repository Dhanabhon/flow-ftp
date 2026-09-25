<script lang="ts">
  import { app } from '$lib/stores/app.svelte';
  import { cn, formatBytes, formatSpeed } from '$lib/utils';
  import {
    cancelTransfer,
    clearFinishedTransfers,
    IS_TAURI,
    onTransferUpdate,
    pauseTransfer,
    resumeTransfer,
    setRateLimit,
    transferList
  } from '$lib/ipc';
  import type { Transfer } from '$lib/types';
  import {
    IconChevronDown,
    IconChevronUp,
    IconUpload,
    IconDownload,
    IconPlay,
    IconPause,
    IconStop,
    IconTrash,
    IconCheckCircle,
    IconXCircle,
    IconLoader,
    IconAlert
  } from '$lib/components/icons';
  import Badge from '$lib/components/ui/badge.svelte';
  import Progress from '$lib/components/ui/progress.svelte';

  const transfers = $derived(app.transfers);

  // Live wiring: replace-by-id snapshots from the engine (Tauri only).
  $effect(() => {
    if (!IS_TAURI) return;
    let unlisten: (() => void) | null = null;
    let disposed = false;

    // Start from an empty queue (mock data is browser-only), then load.
    app.transfers = [];
    transferList()
      .then((records) => {
        if (!disposed) app.transfers = records;
      })
      .catch(() => {});
    const seenStatuses = new Map<string, Transfer['status']>();
    onTransferUpdate((record: Transfer) => {
      if (disposed) return;
      const previous = seenStatuses.get(record.id);
      seenStatuses.set(record.id, record.status);
      const index = app.transfers.findIndex((t) => t.id === record.id);
      if (index >= 0) {
        app.transfers[index] = record;
      } else {
        app.transfers = [...app.transfers, record];
      }
      // Toast on terminal transitions only (never on initial-load echoes).
      const dir = record.direction === 'upload' ? 'Upload' : 'Download';
      if (previous && previous !== record.status) {
        if (record.status === 'completed') {
          app.notify('success', `${dir} completed`, record.fileName);
        } else if (record.status === 'failed') {
          app.notify('danger', `${dir} failed`, record.error ?? record.fileName);
        }
      }
    }).then((un) => {
      if (disposed) un();
      else unlisten = un;
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  });

  // Bandwidth budget choices (bytes/sec); null = unlimited.
  const speedChoices = [
    { label: 'No limit', value: 0 },
    { label: '1 MB/s', value: 1_048_576 },
    { label: '5 MB/s', value: 5_242_880 },
    { label: '10 MB/s', value: 10_485_760 }
  ] as const;
  let speedLimit = $state<number>(0);

  function applySpeedLimit(value: number) {
    speedLimit = value;
    setRateLimit(value).catch(() => {});
  }

  function onRowPause(id: string) {
    pauseTransfer(id).catch(() => {});
  }
  function onRowResume(id: string) {
    resumeTransfer(id).catch(() => {});
  }
  function onRowCancel(id: string) {
    cancelTransfer(id).catch(() => {});
  }

  function pct(t: (typeof transfers)[number]) {
    if (t.size === 0) return 0;
    return (t.transferred / t.size) * 100;
  }

  function statusMeta(status: string) {
    switch (status) {
      case 'active':
        return { icon: IconLoader, tone: 'accent' as const, label: 'Active', spin: true };
      case 'queued':
        return { icon: IconPause, tone: 'neutral' as const, label: 'Queued', spin: false };
      case 'paused':
        return { icon: IconPause, tone: 'warning' as const, label: 'Paused', spin: false };
      case 'completed':
        return { icon: IconCheckCircle, tone: 'success' as const, label: 'Done', spin: false };
      case 'failed':
        return { icon: IconXCircle, tone: 'danger' as const, label: 'Failed', spin: false };
      default:
        return { icon: IconStop, tone: 'neutral' as const, label: status, spin: false };
    }
  }
</script>

<div class="flex flex-col border-t border-border bg-bg-elevated">
  <!-- Header bar -->
  <div class="flex h-9 items-center gap-2 px-3">
    <button
      class="flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-fg-muted hover:text-fg"
      onclick={() => app.toggleQueue()}
    >
      {#if app.queueCollapsed}
        <IconChevronUp size={14} />
      {:else}
        <IconChevronDown size={14} />
      {/if}
      Transfers
    </button>

    <div class="flex items-center gap-1.5">
      {#if app.activeTransfers.length > 0}
        <Badge variant="accent">{app.activeTransfers.length} active</Badge>
      {/if}
      {#if app.queuedCount > 0}
        <Badge variant="neutral">{app.queuedCount} queued</Badge>
      {/if}
      {#if app.failedCount > 0}
        <Badge variant="danger">{app.failedCount} failed</Badge>
      {/if}
    </div>

    <div class="ml-auto flex items-center gap-2">
      <label class="flex items-center gap-1 text-[10px] uppercase tracking-wider text-fg-subtle">
        Speed
        <select
          class="rounded border border-border bg-bg px-1.5 py-0.5 text-[11px] text-fg outline-none"
          value={speedLimit}
          onchange={(e) => applySpeedLimit(Number(e.currentTarget.value))}
        >
          {#each speedChoices as choice (choice.value)}
            <option value={choice.value}>{choice.label}</option>
          {/each}
        </select>
      </label>
      <button class="rounded p-1.5 text-fg-subtle transition-colors hover:bg-bg-hover hover:text-fg">
        <IconPause size={13} />
      </button>
      <button class="rounded p-1.5 text-fg-subtle transition-colors hover:bg-bg-hover hover:text-fg">
        <IconPlay size={13} />
      </button>
      <button
        class="rounded p-1.5 text-fg-subtle transition-colors hover:bg-bg-hover hover:text-fg"
        title="Clear finished transfers"
        onclick={() => clearFinishedTransfers().catch(() => {})}
      >
        <IconTrash size={13} />
      </button>
    </div>
  </div>

  {#if !app.queueCollapsed}
    <div class="max-h-56 overflow-y-auto border-t border-border">
      <!-- Column header -->
      <div class="flex h-6 items-center gap-3 px-3 text-[10px] font-semibold uppercase tracking-wider text-fg-faint">
        <span class="w-4"></span>
        <span class="w-64">Name</span>
        <span class="flex-1">Progress</span>
        <span class="w-20 text-right">Size</span>
        <span class="w-24 text-right">Speed</span>
        <span class="w-16 text-right">Status</span>
        <span class="w-16"></span>
      </div>

      {#each transfers as t (t.id)}
        {@const meta = statusMeta(t.status)}
        {@const Icon = t.direction === 'upload' ? IconUpload : IconDownload}
        {@const StatusIcon = meta.icon}
        <div
          class="group flex items-center gap-3 px-3 py-2 text-xs hover:bg-bg-hover/50"
          class:opacity-60={t.status === 'completed'}
        >
          <!-- direction icon -->
          <div class={cn('grid w-4 place-items-center', t.direction === 'upload' ? 'text-accent' : 'text-success')}>
            <Icon size={13} />
          </div>

          <!-- name -->
          <div class="w-64 min-w-0">
            <div class="truncate font-medium text-fg">{t.fileName}</div>
            <div class="truncate text-[10px] text-fg-subtle">
              {t.connectionName} · {t.remotePath}
            </div>
          </div>

          <!-- progress -->
          <div class="flex flex-1 flex-col gap-1">
            <Progress value={pct(t)} tone={t.status === 'failed' ? 'danger' : 'speed'} />
            {#if t.error}
              <div class="flex items-center gap-1 text-[10px] text-danger">
                <IconAlert size={10} /> {t.error}
              </div>
            {/if}
          </div>

          <!-- size -->
          <div class="w-20 text-right font-mono text-fg-subtle">
            {formatBytes(t.transferred)} / {formatBytes(t.size)}
          </div>

          <!-- speed -->
          <div class="w-24 text-right font-mono text-fg-subtle">
            {#if t.status === 'active'}
              <span class="text-success">{formatSpeed(t.speed)}</span>
            {:else}
              —
            {/if}
          </div>

          <!-- status -->
          <div class="flex w-16 items-center justify-end gap-1">
            <StatusIcon
              size={12}
              class={cn(
                meta.tone === 'accent' && 'text-accent',
                meta.tone === 'success' && 'text-success',
                meta.tone === 'danger' && 'text-danger',
                meta.tone === 'warning' && 'text-warning',
                meta.tone === 'neutral' && 'text-fg-subtle',
                meta.spin && 'animate-spin'
              )}
            />
            <span class="text-[10px] text-fg-subtle">{meta.label}</span>
          </div>

          <!-- row actions -->
          <div class="flex w-16 items-center justify-end gap-0.5 opacity-0 transition-opacity group-hover:opacity-100">
            {#if t.status === 'active'}
              <button class="rounded p-1 text-fg-subtle hover:bg-bg-active hover:text-fg" title="Pause" onclick={() => onRowPause(t.id)}><IconPause size={12} /></button>
            {:else if t.status === 'paused'}
              <button class="rounded p-1 text-fg-subtle hover:bg-bg-active hover:text-fg" title="Resume" onclick={() => onRowResume(t.id)}><IconPlay size={12} /></button>
            {/if}
            {#if t.status !== 'completed' && t.status !== 'canceled'}
              <button class="rounded p-1 text-fg-subtle hover:bg-bg-active hover:text-danger" title="Cancel" onclick={() => onRowCancel(t.id)}><IconStop size={12} /></button>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
