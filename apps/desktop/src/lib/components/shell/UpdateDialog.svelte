<script lang="ts">
  import { onMount } from 'svelte';
  import { app } from '$lib/stores/app.svelte';
  import {
    IS_TAURI,
    appVersion,
    checkForUpdate,
    installUpdate,
    onCheckUpdates,
    relaunchApp,
    type UpdateInfo,
    type UpdateProgress,
  } from '$lib/ipc';
  import Modal from '$lib/components/ui/modal.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import { formatBytes } from '$lib/utils';
  import {
    IconAlert,
    IconCheckCircle,
    IconDownload,
    IconLoader,
    IconRefresh,
  } from '$lib/components/icons';

  /**
   * Software Update, in the app's own theme. Opened from the FlowFTP menu,
   * the command palette, or the launch auto-check toast. Download progress
   * survives closing and reopening the dialog; dismissal is disabled only
   * where it would lose work.
   */
  type Phase =
    | 'idle'
    | 'checking'
    | 'current'
    | 'available'
    | 'downloading'
    | 'installed'
    | 'error';

  let phase = $state<Phase>('idle');
  let info = $state<UpdateInfo | null>(null);
  let runningVersion = $state('');
  let error = $state<string | null>(null);
  let progress = $state<UpdateProgress>({ contentLength: null, received: 0 });

  async function runCheck() {
    phase = 'checking';
    error = null;
    try {
      const [found, current] = await Promise.all([checkForUpdate(), appVersion()]);
      runningVersion = current;
      info = found;
      phase = found ? 'available' : 'current';
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      phase = 'error';
    }
  }

  // Each open runs a fresh check unless an install is already in flight;
  // closing after a plain check resets so the next open re-checks.
  $effect(() => {
    if (app.updateDialogOpen) {
      if (phase === 'idle') void runCheck();
    } else if (phase === 'checking' || phase === 'current' || phase === 'error') {
      phase = 'idle';
    }
  });

  async function startInstall() {
    phase = 'downloading';
    progress = { contentLength: null, received: 0 };
    try {
      await installUpdate((p) => (progress = p));
      phase = 'installed';
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      phase = 'error';
    }
  }

  const percent = $derived(
    progress.contentLength && progress.contentLength > 0
      ? Math.min(100, (progress.received / progress.contentLength) * 100)
      : null
  );

  // Dev builds cannot be update-checked (no signing); the dialog explains it.
  const devBuildHint = 'Unsigned development builds cannot check for updates — use an installed release.';

  onMount(() => {
    if (!IS_TAURI) return;
    let unlisten: (() => void) | null = null;
    let disposed = false;
    onCheckUpdates(() => (app.updateDialogOpen = true)).then((un) => {
      if (disposed) un();
      else unlisten = un;
    });

    // Silent once-per-launch check: only surfaces a toast when newer exists.
    const timer = setTimeout(() => {
      checkForUpdate()
        .then((found) => {
          if (found) {
            app.notify(
              'warning',
              `FlowFTP ${found.version} is available`,
              'Install it from the FlowFTP menu → Check for Updates…'
            );
          }
        })
        .catch(() => {
          /* offline or dev build: stay quiet */
        });
    }, 3_000);

    return () => {
      disposed = true;
      clearTimeout(timer);
      unlisten?.();
    };
  });
</script>

<Modal bind:open={app.updateDialogOpen} title="Software Update" center width="sm">
  <div class="min-w-0">
    {#if phase === 'checking'}
      <div class="flex flex-col items-center gap-3 py-6 text-fg-subtle">
        <IconLoader size={24} class="animate-spin opacity-70" />
        <p class="text-sm">Checking for updates…</p>
      </div>
    {:else if phase === 'current'}
      <div class="flex flex-col items-center gap-3 py-6">
        <span class="grid h-12 w-12 place-items-center rounded-full bg-success/15 text-success">
          <IconCheckCircle size={24} />
        </span>
        <div class="text-center">
          <p class="text-sm font-medium text-fg">FlowFTP is up to date</p>
          <p class="mt-1 text-xs text-fg-subtle">Version {runningVersion}</p>
        </div>
      </div>
    {:else if phase === 'available' && info}
      <div class="flex items-start gap-3">
        <span class="grid h-10 w-10 shrink-0 place-items-center rounded-full bg-accent/15 text-accent-text">
          <IconDownload size={18} />
        </span>
        <div class="min-w-0 flex-1">
          <p class="text-sm font-medium text-fg">
            FlowFTP {info.version} is available
          </p>
          <p class="mt-0.5 text-xs text-fg-subtle">
            You have {info.currentVersion || runningVersion}
          </p>
          {#if info.notes}
            <div
              class="mt-3 max-h-44 overflow-y-auto whitespace-pre-line rounded-lg border border-border bg-bg/60 p-3 text-xs leading-relaxed text-fg-muted"
            >
              {info.notes}
            </div>
          {/if}
        </div>
      </div>
    {:else if phase === 'downloading'}
      <div class="flex flex-col gap-3 py-2">
        <div class="flex items-center gap-3">
          <IconLoader size={18} class="animate-spin text-accent-text" />
          <p class="text-sm font-medium text-fg">Downloading FlowFTP update…</p>
        </div>
        <div class="h-1.5 overflow-hidden rounded-full bg-bg-active">
          {#if percent !== null}
            <div
              class="h-full rounded-full bg-accent transition-[width] duration-200"
              style={`width: ${percent}%`}
            ></div>
          {:else}
            <div class="h-full w-1/3 animate-pulse rounded-full bg-accent/60"></div>
          {/if}
        </div>
        <p class="font-mono text-xs text-fg-subtle">
          {#if percent !== null}
            {formatBytes(progress.received)} / {formatBytes(progress.contentLength ?? 0)} · {Math.round(percent)}%
          {:else}
            {formatBytes(progress.received)} downloaded
          {/if}
        </p>
      </div>
    {:else if phase === 'installed'}
      <div class="flex flex-col items-center gap-3 py-4">
        <span class="grid h-12 w-12 place-items-center rounded-full bg-success/15 text-success">
          <IconCheckCircle size={24} />
        </span>
        <div class="text-center">
          <p class="text-sm font-medium text-fg">Update installed</p>
          <p class="mt-1 max-w-64 text-xs text-fg-subtle">
            Restart FlowFTP to finish switching to the new version. Resumable
            transfers keep their progress.
          </p>
        </div>
      </div>
    {:else if phase === 'error'}
      <div class="flex items-start gap-3">
        <span class="grid h-10 w-10 shrink-0 place-items-center rounded-full bg-danger/15 text-danger">
          <IconAlert size={18} />
        </span>
        <div class="min-w-0">
          <p class="text-sm font-medium text-fg">Update check failed</p>
          <p class="mt-1 break-words text-xs leading-relaxed text-fg-subtle">{error}</p>
          <p class="mt-2 text-xs text-fg-faint">{devBuildHint}</p>
        </div>
      </div>
    {/if}
  </div>

  {#snippet footer()}
    {#if phase === 'checking' || phase === 'current' || phase === 'error'}
      <Button variant="ghost" onclick={() => (app.updateDialogOpen = false)}>
        {phase === 'checking' ? 'Cancel' : phase === 'error' ? 'Close' : 'OK'}
      </Button>
    {:else if phase === 'available'}
      <Button variant="ghost" onclick={() => (app.updateDialogOpen = false)}>
        Later
      </Button>
      <Button variant="default" onclick={startInstall}>
        <IconRefresh size={13} /> Download &amp; Install
      </Button>
    {:else if phase === 'installed'}
      <Button variant="default" onclick={() => void relaunchApp()}>
        Restart Now
      </Button>
    {:else if phase === 'downloading'}
      <Button variant="ghost" onclick={() => (app.updateDialogOpen = false)}>
        Hide
      </Button>
    {/if}
  {/snippet}
</Modal>
