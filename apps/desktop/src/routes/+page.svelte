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
      return;
    }
    listRemote(connectionId, path)
      .then((files) => {
        app.remoteFiles = files;
        app.errors.remote = null;
      })
      .catch((e: Error) => (app.errors.remote = e.message));
  });

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

  /** Uploads queued behind an overwrite confirmation: file -> remote path. */
  let overwriteConfirm = $state<{ uploads: { localPath: string; fileName: string }[] } | null>(null);

  function enqueueUploads(uploads: { localPath: string; fileName: string }[]) {
    if (!app.activeConnectionId) return;
    for (const upload of uploads) {
      enqueueTransfer({
        id: crypto.randomUUID(),
        connectionId: app.activeConnectionId,
        direction: 'upload',
        remotePath: joinPath(app.remotePath, upload.fileName),
        localPath: upload.localPath,
        fileName: upload.fileName
      }).catch((e: Error) => app.notify('danger', 'Upload failed to start', e.message));
    }
  }

  /** Upload the selected local files (or picked ones), confirming overwrites. */
  async function handleUpload() {
    if (!app.activeConnectionId) return;
    const selected = [...app.localSelected].filter((n) => n !== '..');
    const files =
      selected.length > 0
        ? selected.map((name) => joinPath(app.localPath, name))
        : await pickFilesToUpload();
    if (files.length === 0) return;
    const uploads = files.map((file) => ({ localPath: file, fileName: basename(file) }));
    const remoteNames = new Set(app.remoteFiles.map((f) => f.name));
    const overwrites = uploads.filter((u) => remoteNames.has(u.fileName));
    if (overwrites.length > 0) {
      // Preview -> Confirm -> Execute (DESIGN.md): name what gets replaced.
      overwriteConfirm = { uploads };
    } else {
      enqueueUploads(uploads);
    }
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
    for (const name of selected) {
      await enqueueTransfer({
        id: crypto.randomUUID(),
        connectionId: app.activeConnectionId,
        direction: 'download',
        remotePath: joinPath(app.remotePath, name),
        localPath: joinPath(destination, name),
        fileName: name
      });
    }
  }
</script>

<svelte:window on:keydown={onKeydown} />

<div class="flex h-screen w-screen flex-col overflow-hidden bg-bg">
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
      <div class="flex min-h-0 flex-1">
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
          error={app.errors.local}
        />
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
          error={app.errors.remote}
        />
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

<!-- Overwrite confirmation for uploads: Preview -> Confirm -> Execute -->
<Modal
  open={overwriteConfirm !== null}
  title="Replace {overwriteConfirm?.uploads.length ?? 0}
    file{overwriteConfirm?.uploads.length === 1 ? '' : 's'} on {app.activeConnection?.name ?? 'the server'}?"
  description="These files already exist in {app.remotePath}. Uploading replaces the remote copies — the local files are not changed."
  width="sm"
>
  <div class="max-h-48 space-y-1 overflow-y-auto rounded-md border border-border bg-bg-panel p-2.5">
    {#each overwriteConfirm?.uploads ?? [] as upload (upload.localPath)}
      <div class="truncate font-mono text-xs text-fg">{upload.fileName}</div>
    {/each}
  </div>
  {#snippet footer()}
    <Button variant="ghost" onclick={() => (overwriteConfirm = null)}>Cancel</Button>
    <Button variant="danger" onclick={() => { const u = overwriteConfirm; overwriteConfirm = null; if (u) enqueueUploads(u.uploads); }}>Replace</Button>
  {/snippet}
</Modal>
