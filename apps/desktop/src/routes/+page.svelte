<script lang="ts">
  import { app } from '$lib/stores/app.svelte';
  import {
    enqueueTransfer,
    IS_TAURI,
    listLocal,
    listRemote,
    localHome,
    pickDownloadDirectory,
    pickFilesToUpload
  } from '$lib/ipc';
  import Header from '$lib/components/shell/Header.svelte';
  import Sidebar from '$lib/components/shell/Sidebar.svelte';
  import FilePane from '$lib/components/shell/FilePane.svelte';
  import TransferQueue from '$lib/components/shell/TransferQueue.svelte';
  import PreviewPanel from '$lib/components/shell/PreviewPanel.svelte';
  import CommandPalette from '$lib/components/overlays/CommandPalette.svelte';
  import QuickConnect from '$lib/components/overlays/QuickConnect.svelte';
  import SyncPreview from '$lib/components/overlays/SyncPreview.svelte';
  import ToastHost from '$lib/components/shell/ToastHost.svelte';

  // Global keyboard shortcuts beyond command palette (handled in Header).
  function onKeydown(e: KeyboardEvent) {
    const mod = e.metaKey || e.ctrlKey;
    if (mod && ['1', '2', '3', '4'].includes(e.key)) {
      e.preventDefault();
      const views = ['connections', 'transfers', 'sync', 'history'] as const;
      app.setView(views[Number(e.key) - 1]);
    } else if (mod && e.key === ',') {
      e.preventDefault();
      app.setView('settings');
    } else if (mod && e.shiftKey && e.key.toLowerCase() === '.') {
      e.preventDefault();
      app.toggleHidden();
    }
  }

  // ── Live data wiring (Tauri only; browser dev stays on mocks) ─────────────

  // Initialize the local pane at the user's home directory.
  if (IS_TAURI) {
    localHome()
      .then((home) => (app.localPath = home))
      .catch(() => {});
  }

  // Local listing follows the local path.
  $effect(() => {
    if (!IS_TAURI) return;
    const path = app.localPath;
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

  /** Upload the selected local files (or picked ones) to the remote path. */
  async function handleUpload() {
    if (!app.activeConnectionId) return;
    const selected = [...app.localSelected].filter((n) => n !== '..');
    const files =
      selected.length > 0
        ? selected.map((name) => joinPath(app.localPath, name))
        : await pickFilesToUpload();
    for (const file of files) {
      const fileName = basename(file);
      await enqueueTransfer({
        id: crypto.randomUUID(),
        connectionId: app.activeConnectionId,
        direction: 'upload',
        remotePath: joinPath(app.remotePath, fileName),
        localPath: file,
        fileName
      });
    }
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

    <!-- Main content area: dual pane + preview -->
    <main class="flex min-w-0 flex-1 flex-col">
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
          error={app.errors.remote}
        />
        <PreviewPanel />
      </div>

      <TransferQueue />
    </main>
  </div>
</div>

<!-- Global overlays -->
<CommandPalette />
<QuickConnect />
<SyncPreview />
<ToastHost />
