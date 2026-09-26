<script lang="ts">
  import { app } from '$lib/stores/app.svelte';
  import {
    deleteRemote,
    enqueueTransfer,
    IS_TAURI,
    listLocal,
    listRemote,
    localHome,
    pickDownloadDirectory,
    pickFilesToUpload,
    profileList,
    remoteEditOpen
  } from '$lib/ipc';
  import { localDelete, localMkdir, localRename, mkdirRemote, renameRemote } from '$lib/ipc';
  import { cn } from '$lib/utils';
  import type { TransferDirection } from '$lib/types';
  import { browser } from '$app/environment';
  import Modal from '$lib/components/ui/modal.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import { IconAlert } from '$lib/components/icons';
  import Header from '$lib/components/shell/Header.svelte';
  import Sidebar from '$lib/components/shell/Sidebar.svelte';
  import FilePane from '$lib/components/shell/FilePane.svelte';
  import TransferQueue from '$lib/components/shell/TransferQueue.svelte';
  import PreviewPanel from '$lib/components/shell/PreviewPanel.svelte';
  import CommandPalette from '$lib/components/overlays/CommandPalette.svelte';
  import QuickConnect from '$lib/components/overlays/QuickConnect.svelte';
  import SyncPreview from '$lib/components/overlays/SyncPreview.svelte';
  import ToastHost from '$lib/components/shell/ToastHost.svelte';

  // Global keyboard shortcuts — every binding advertised in the UI exists.
  function onKeydown(e: KeyboardEvent) {
    const mod = e.metaKey || e.ctrlKey;
    if (mod && e.key === '1') {
      e.preventDefault();
      app.setView('connections');
    } else if (mod && e.key === '2') {
      e.preventDefault();
      app.setView('transfers');
    } else if (mod && e.key.toLowerCase() === 'n') {
      e.preventDefault();
      app.quickConnectOpen = true;
    } else if (mod && e.shiftKey && e.key.toLowerCase() === 's') {
      e.preventDefault();
      app.syncOpen = true;
    } else if (mod && e.shiftKey && e.key.toLowerCase() === '.') {
      e.preventDefault();
      app.toggleHidden();
    }
  }

  // ── Live data wiring (Tauri only; browser dev stays on mocks) ─────────────

  // Restore the pane split from the previous session.
  if (browser) {
    const saved = Number(localStorage.getItem('flowftp:pane-ratio'));
    if (!Number.isNaN(saved) && saved > 0) app.setPaneRatio(saved);
  }

  // Load saved connection profiles (browser dev keeps mocks).
  $effect(() => {
    if (!IS_TAURI) return;
    profileList()
      .then((profiles) => {
        app.connections = profiles;
        // The mock seed's active connection does not exist here; start clean.
        if (app.activeConnectionId && !profiles.some((p) => p.id === app.activeConnectionId)) {
          app.activeConnectionId = null;
          app.remoteFiles = [];
          app.remoteSelected = new Set();
          app.inspector = null;
        }
      })
      .catch(() => {});
  });

  // Initialize the local pane at the user's home directory.
  if (IS_TAURI) {
    localHome()
      .then((home) => (app.localPath = home))
      .catch(() => {});
  }

  // Local listing follows the local path (and refreshes after mutations).
  $effect(() => {
    if (!IS_TAURI) return;
    const path = app.localPath;
    void app.refreshTick;
    listLocal(path)
      .then((files) => {
        app.localFiles = files;
        app.errors.local = null;
      })
      .catch((e: Error) => (app.errors.local = e.message));
  });

  // Remote listing follows the active connection + remote path.
  $effect(() => {
    if (!IS_TAURI) return;
    const connectionId = app.activeConnectionId;
    const path = app.remotePath;
    void app.refreshTick;
    if (!connectionId) {
      app.remoteFiles = [];
      app.remoteSelected = new Set();
      app.errors.remote = null;
      return;
    }
    listRemote(connectionId, path)
      .then((files) => {
        app.remoteFiles = files;
        app.errors.remote = null;
      })
      .catch((e: Error) => (app.errors.remote = e.message));
  });

  // ── Drag & drop ───────────────────────────────────────────────────────────

  /** Wrapper element around the remote pane: hit-test for Finder drops. */
  let remotePaneEl: HTMLDivElement | null = $state(null);
  /** Finder drag hovering over the remote pane. */
  let externalDragOver = $state(false);

  /**
   * Queue uploads, checking the destination directory for files that would
   * be replaced. Falls back to direct enqueue when the target is not the
   * currently listed folder (its contents are unknown).
   */
  function beginUploads(
    uploads: { localPath: string; fileName: string }[],
    targetDir: string = app.remotePath
  ) {
    if (!app.activeConnectionId || uploads.length === 0) return;
    const known =
      targetDir === app.remotePath
        ? new Set(app.remoteFiles.map((f) => f.name))
        : null;
    const overwrites = known
      ? uploads.filter((u) => known.has(u.fileName))
      : [];
    if (overwrites.length > 0) {
      overwriteConfirm = {
        direction: 'upload',
        targetDir,
        items: overwrites.map((u) => ({ localPath: u.localPath, name: u.fileName }))
      };
      return;
    }
    enqueueUploads(uploads, targetDir);
  }

  function enqueueUploads(
    uploads: { localPath: string; fileName: string }[],
    targetDir: string
  ) {
    for (const upload of uploads) {
      enqueueTransfer({
        id: crypto.randomUUID(),
        connectionId: app.activeConnectionId!,
        direction: 'upload',
        remotePath: joinPath(targetDir, upload.fileName),
        localPath: upload.localPath,
        fileName: upload.fileName
      }).catch((e: Error) => app.notify('danger', 'Upload failed to start', e.message));
    }
  }

  /** Queue downloads, confirming local replacements in the listed folder. */
  function beginDownloads(
    entries: { name: string; isDir: boolean }[],
    targetDir: string = app.localPath
  ) {
    const files = entries.filter((entry) => !entry.isDir);
    if (!files.length) return;
    const known =
      targetDir === app.localPath
        ? new Set(app.localFiles.map((f) => f.name))
        : null;
    const overwrites = known ? files.filter((f) => known.has(f.name)) : [];
    if (overwrites.length > 0) {
      overwriteConfirm = { direction: 'download', targetDir, items: files };
      return;
    }
    enqueueDownloads(files, targetDir);
  }

  function enqueueDownloads(files: { name: string }[], targetDir: string) {
    for (const file of files) {
      enqueueTransfer({
        id: crypto.randomUUID(),
        connectionId: app.activeConnectionId!,
        direction: 'download',
        remotePath: joinPath(app.remotePath, file.name),
        localPath: joinPath(targetDir, file.name),
        fileName: file.name
      }).catch((e: Error) => app.notify('danger', 'Download failed to start', e.message));
    }
  }

  /** Finder drop onto the remote pane: watch, hit-test, upload on drop. */
  $effect(() => {
    if (!IS_TAURI) return;
    let unlisten: (() => void) | null = null;
    let disposed = false;
    import('@tauri-apps/api/webview').then(({ getCurrentWebview }) =>
      getCurrentWebview().onDragDropEvent((event) => {
        if (disposed) return;
        const payload = event.payload;
        if (payload.type === 'leave') {
          externalDragOver = false;
          return;
        }
        const rect = remotePaneEl?.getBoundingClientRect();
        const scale = window.devicePixelRatio || 1;
        const x = payload.position.x / scale;
        const y = payload.position.y / scale;
        const inside =
          !!rect &&
          !!app.activeConnectionId &&
          x >= rect.left &&
          x <= rect.right &&
          y >= rect.top &&
          y <= rect.bottom;

        if (payload.type === 'enter' || payload.type === 'over') {
          externalDragOver = inside;
        } else {
          // drop
          externalDragOver = false;
          if (inside && payload.paths.length > 0) {
            beginUploads(
              payload.paths.map((localPath) => ({
                localPath,
                fileName: localPath.split('/').pop() ?? localPath
              }))
            );
          }
        }
      })
    ).then((un) => {
      if (disposed) un();
      else unlisten = un;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  });

  /** Drop callbacks from the panes. */
  function handleDropLocalFiles(
    files: { localPath: string; fileName: string }[],
    targetDir: string
  ) {
    beginUploads(files, targetDir);
  }

  function handleDropRemoteEntries(
    entries: { name: string; isDir: boolean }[],
    targetDir: string
  ) {
    beginDownloads(entries, targetDir);
  }

  // ── Pane splitter ─────────────────────────────────────────────────────────

  /** Width of the inspector column (w-72 = 288px) plus its border. */
  const INSPECTOR_WIDTH = 289;
  let paneRow = $state<HTMLDivElement | null>(null);
  let draggingPane = $state(false);

  function startPaneDrag(e: PointerEvent) {
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    draggingPane = true;
  }

  function movePaneDrag(e: PointerEvent) {
    if (!draggingPane || !paneRow) return;
    const rect = paneRow.getBoundingClientRect();
    const area = rect.width - INSPECTOR_WIDTH;
    if (area <= 0) return;
    app.setPaneRatio((e.clientX - rect.left) / area);
  }

  function endPaneDrag() {
    if (!draggingPane) return;
    draggingPane = false;
    localStorage.setItem('flowftp:pane-ratio', String(app.paneRatio));
  }

  /** Double-click the splitter to restore the even split. */
  function resetPaneSplit() {
    app.setPaneRatio(0.5);
    localStorage.setItem('flowftp:pane-ratio', '0.5');
  }

  function nudgePaneSplit(e: KeyboardEvent) {
    const step = e.shiftKey ? 0.1 : 0.02;
    if (e.key === 'ArrowLeft') {
      e.preventDefault();
      app.setPaneRatio(app.paneRatio - step);
    } else if (e.key === 'ArrowRight') {
      e.preventDefault();
      app.setPaneRatio(app.paneRatio + step);
    } else if (e.key === 'Home' || e.key === 'Enter') {
      e.preventDefault();
      resetPaneSplit();
    }
  }

  // ── Navigation handlers ───────────────────────────────────────────────────

  /** Navigate the local pane; `..` goes up one level. */
  function navigateLocal(name: string) {
    if (name === '..') {
      const parts = app.localPath.split('/').filter(Boolean);
      parts.pop();
      app.localPath = '/' + parts.join('/');
    } else {
      app.localPath = joinPath(app.localPath, name);
    }
    app.localSelected = new Set();
  }

  /** Navigate the remote pane; `..` goes up one level. */
  function navigateRemote(name: string) {
    if (name === '..') {
      const parts = app.remotePath.split('/').filter(Boolean);
      parts.pop();
      app.remotePath = '/' + parts.join('/');
    } else {
      app.remotePath = joinPath(app.remotePath, name);
    }
    app.remoteSelected = new Set();
  }

  /** POSIX join with root normalization. */
  function joinPath(base: string, child: string): string {
    if (base === '/') return `/${child}`;
    return `${base.replace(/\/+$/, '')}/${child}`;
  }

  // ── Transfer buttons ────────────────────────────────────────────────────

  const basename = (path: string) => path.split('/').pop() ?? path;

  /** Uploads queued behind an overwrite confirmation. */
  let overwriteConfirm = $state<{
    direction: TransferDirection;
    targetDir: string;
    items: { localPath?: string; name: string }[];
  } | null>(null);

  /** Upload the selected local files (or picked ones), confirming overwrites. */
  async function handleUpload() {
    if (!app.activeConnectionId) return;
    const selected = [...app.localSelected].filter((n) => n !== '..');
    const files =
      selected.length > 0
        ? selected.map((name) => joinPath(app.localPath, name))
        : await pickFilesToUpload();
    beginUploads(
      files.map((file) => ({ localPath: file, fileName: basename(file) }))
    );
  }

  // ── File management (create / rename / delete) ───────────────────────────

  /** Pending deletion awaiting confirmation: { side, names }. */
  let deleteConfirm = $state<{ side: 'local' | 'remote'; names: string[] } | null>(null);

  function handleCreateFolder(side: 'local' | 'remote', name: string) {
    if (side === 'local') {
      localMkdir(joinPath(app.localPath, name))
        .then(() => app.refreshTick++)
        .catch((e: Error) => app.notify('danger', 'Could not create folder', e.message));
    } else if (app.activeConnectionId) {
      mkdirRemote(app.activeConnectionId, joinPath(app.remotePath, name))
        .then(() => app.refreshTick++)
        .catch((e: Error) => app.notify('danger', 'Could not create folder', e.message));
    }
  }

  function handleRename(side: 'local' | 'remote', from: string, to: string) {
    if (side === 'local') {
      localRename(joinPath(app.localPath, from), joinPath(app.localPath, to))
        .then(() => app.refreshTick++)
        .catch((e: Error) => app.notify('danger', 'Could not rename', e.message));
    } else if (app.activeConnectionId) {
      renameRemote(app.activeConnectionId, joinPath(app.remotePath, from), joinPath(app.remotePath, to))
        .then(() => app.refreshTick++)
        .catch((e: Error) => app.notify('danger', 'Could not rename', e.message));
    }
  }

  function handleDelete(side: 'local' | 'remote', names: string[]) {
    deleteConfirm = { side, names };
  }

  async function confirmDelete() {
    const pending = deleteConfirm;
    if (!pending) return;
    deleteConfirm = null;
    const base = pending.side === 'local' ? app.localPath : app.remotePath;
    const remove = (name: string): Promise<void> =>
      pending.side === 'local'
        ? localDelete(joinPath(base, name))
        : app.activeConnectionId
          ? deleteRemote(app.activeConnectionId, joinPath(base, name))
          : Promise.reject(new Error('No active connection'));
    const results = await Promise.allSettled(pending.names.map(remove));
    const failures = results.filter((r) => r.status === 'rejected');
    if (failures.length > 0) {
      app.notify(
        'danger',
        `Could not delete ${failures.length} item${failures.length === 1 ? '' : 's'}`,
        String((failures[0] as PromiseRejectedResult).reason)
      );
    }
    app.refreshTick++;
  }

  /** Double-click a remote file → open it for remote editing. */
  function handleEdit(name: string) {
    if (!app.activeConnectionId) return;
    const path = joinPath(app.remotePath, name);
    remoteEditOpen(app.activeConnectionId, path).catch((e: Error) =>
      app.notify('danger', 'Could not open editor', e.message)
    );
  }

  /** Download the selected remote files into a chosen directory. */
  async function handleDownload() {
    if (!app.activeConnectionId) return;
    const selected = [...app.remoteSelected].filter((n) => n !== '..');
    if (selected.length === 0) return;
    const destination = await pickDownloadDirectory();
    if (!destination) return;
    beginDownloads(
      selected.map((name) => ({ name, isDir: false })),
      destination
    );
  }
</script>

<svelte:window on:keydown={onKeydown} />

<div class="flex h-full w-full flex-col overflow-hidden bg-bg">
  <Header />

  <div class="flex min-h-0 flex-1">
    <Sidebar />

    <!-- Main content area -->
    <main class="flex min-w-0 flex-1 flex-col">
      {#if app.view === 'transfers'}
        <div class="flex min-h-0 flex-1 flex-col">
          <TransferQueue expanded />
        </div>
      {:else}
      <div class="flex min-h-0 flex-1" bind:this={paneRow}>
        <div
          class="flex min-h-0 min-w-0"
          style="width: calc((100% - {INSPECTOR_WIDTH}px) * {app.paneRatio})"
        >
        <FilePane
          side="local"
          title="Local"
          files={app.localFiles}
          path={app.localPath}
          selected={app.localSelected}
          onSelect={app.selectLocal.bind(app)}
          showHidden={app.showHidden}
          onNavigate={navigateLocal}
          onNavigateTo={(path) => (app.localPath = path)}
          onUpload={handleUpload}
          onCreateFolder={(name) => handleCreateFolder('local', name)}
          onRename={(from, to) => handleRename('local', from, to)}
          onDelete={(names) => handleDelete('local', names)}
          onDropRemoteEntries={handleDropRemoteEntries}
          error={app.errors.local}
        />
        </div>
        <!-- ARIA APG splitter pattern: a focusable separator with arrow-key
             adjustment. svelte-check flags the tabindex; the role and value
             attributes above are the documented pattern. -->
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <div
          role="separator"
          aria-orientation="vertical"
          aria-label="Resize panes"
          aria-valuenow={Math.round(app.paneRatio * 100)}
          aria-valuemin="15"
          aria-valuemax="85"
          tabindex="0"
          class={cn(
            'group relative w-1.5 shrink-0 cursor-col-resize bg-transparent',
            'before:absolute before:inset-y-0 before:left-1/2 before:w-px before:-translate-x-1/2 before:bg-border',
            'hover:before:bg-accent/60',
            draggingPane && 'before:bg-accent'
          )}
          onpointerdown={startPaneDrag}
          onpointermove={movePaneDrag}
          onpointerup={endPaneDrag}
          onpointercancel={endPaneDrag}
          ondblclick={resetPaneSplit}
          onkeydown={nudgePaneSplit}
        ></div>
        <div class="flex min-w-0 flex-1" bind:this={remotePaneEl}>
        <FilePane
          side="remote"
          title="Remote"
          files={app.remoteFiles}
          path={app.remotePath}
          selected={app.remoteSelected}
          onSelect={app.selectRemote.bind(app)}
          showHidden={app.showHidden}
          connectionName={app.activeConnection?.name}
          onNavigate={navigateRemote}
          onNavigateTo={(path) => (app.remotePath = path)}
          onDownload={handleDownload}
          onEdit={handleEdit}
          disconnected={!app.activeConnectionId}
          onConnect={() => (app.quickConnectOpen = true)}
          onCreateFolder={(name) => handleCreateFolder('remote', name)}
          onRename={(from, to) => handleRename('remote', from, to)}
          onDelete={(names) => handleDelete('remote', names)}
          onDropLocalFiles={handleDropLocalFiles}
          externalDragOver={externalDragOver}
          error={app.errors.remote}
          />
        </div>
        <PreviewPanel />
      </div>

      <TransferQueue />
      {/if}
    </main>
  </div>
</div>

<!-- Global overlays -->
<CommandPalette />
<QuickConnect />
<SyncPreview />
<ToastHost />

<!-- Delete confirmation: Preview → Confirm → Execute (DESIGN.md) -->
<Modal
  open={deleteConfirm !== null}
  title="Delete {deleteConfirm?.names.length ?? 0}
    item{deleteConfirm?.names.length === 1 ? '' : 's'}?"
  description={deleteConfirm
    ? `${deleteConfirm.side === 'remote' ? `${app.activeConnection?.name ?? ''} · ` : ''}${deleteConfirm.side === 'remote' ? app.remotePath : app.localPath} — this cannot be undone.`
    : 'This cannot be undone.'}
  width="sm"
>
  <div class="max-h-48 space-y-1 overflow-y-auto rounded-md border border-border bg-bg-panel p-2.5">
    {#each deleteConfirm?.names ?? [] as name (name)}
      <div class="truncate font-mono text-xs text-fg">{name}</div>
    {/each}
  </div>
  {#snippet footer()}
    <Button variant="ghost" onclick={() => (deleteConfirm = null)}>Cancel</Button>
    <Button variant="danger" onclick={confirmDelete}>Delete</Button>
  {/snippet}
</Modal>

<!-- Overwrite confirmation for transfers: Preview -> Confirm -> Execute -->
<Modal
  open={overwriteConfirm !== null}
  title="Replace {overwriteConfirm?.items.length ?? 0}
    file{overwriteConfirm?.items.length === 1 ? '' : 's'}?"
  description={overwriteConfirm?.direction === 'upload'
    ? `These files already exist in ${app.activeConnection?.name ?? 'the server'}:${overwriteConfirm?.targetDir}. Uploading replaces the remote copies. The local files are not changed.`
    : `These files already exist in ${overwriteConfirm?.targetDir}. Downloading replaces the local copies. The remote files are not changed.`}
  width="sm"
>
  <div class="max-h-48 space-y-1 overflow-y-auto rounded-md border border-border bg-bg-panel p-2.5">
    {#each overwriteConfirm?.items ?? [] as item (item.name)}
      <div class="truncate font-mono text-xs text-fg">{item.name}</div>
    {/each}
  </div>
  {#snippet footer()}
    <Button variant="ghost" onclick={() => (overwriteConfirm = null)}>Cancel</Button>
    <Button
      variant="danger"
      onclick={() => {
        const pending = overwriteConfirm;
        overwriteConfirm = null;
        if (!pending) return;
        if (pending.direction === 'upload') {
          enqueueUploads(
            pending.items
              .filter((item) => item.localPath)
              .map((item) => ({ localPath: item.localPath!, fileName: item.name })),
            pending.targetDir
          );
        } else {
          enqueueDownloads(pending.items, pending.targetDir);
        }
      }}
    >
      Replace
    </Button>
  {/snippet}
</Modal>
