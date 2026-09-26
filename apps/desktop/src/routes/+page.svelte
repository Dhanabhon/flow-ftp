<script lang="ts">
  import { app } from '$lib/stores/app.svelte';
  import {
    deleteRemote,
    enqueueTransfer,
    IS_TAURI,
    IpcError,
    listLocal,
    listRemote,
    localHome,
    pickDownloadDirectory,
    pickFilesToUpload,
    profileList,
    remoteEditOpen
  } from '$lib/ipc';
  import { localDelete, localMkdir, localRename, mkdirRemote, renameRemote } from '$lib/ipc';
  import { untrack } from 'svelte';
  import { cn } from '$lib/utils';
  import type { TransferDirection } from '$lib/types';
  import { browser } from '$app/environment';
  import Modal from '$lib/components/ui/modal.svelte';
  import Button from '$lib/components/ui/button.svelte';
import { IconAlert, IconDownload, IconUpload } from '$lib/components/icons';
  import Header from '$lib/components/shell/Header.svelte';
  import Sidebar from '$lib/components/shell/Sidebar.svelte';
  import FilePane from '$lib/components/shell/FilePane.svelte';
  import TransferQueue from '$lib/components/shell/TransferQueue.svelte';
  import PreviewPanel from '$lib/components/shell/PreviewPanel.svelte';
  import CommandPalette from '$lib/components/overlays/CommandPalette.svelte';
  import QuickConnect from '$lib/components/overlays/QuickConnect.svelte';
  import SyncPreview from '$lib/components/overlays/SyncPreview.svelte';
  import ToastHost from '$lib/components/shell/ToastHost.svelte';
  import QuitConfirmDialog from '$lib/components/shell/QuitConfirmDialog.svelte';

  let activeFilePane: 'local' | 'remote' | null = null;

  function setActiveFilePane(target: EventTarget | null) {
    const pane = target instanceof Element
      ? target.closest<HTMLElement>('[data-file-pane]')?.dataset.filePane
      : undefined;
    activeFilePane = pane === 'local' || pane === 'remote' ? pane : null;
  }

  function selectAllInActivePane() {
    if (!activeFilePane) return;
    const files = activeFilePane === 'local' ? app.localFiles : app.remoteFiles;
    const names = files
      .filter((file) => file.name !== '..' && (app.showHidden || !file.name.startsWith('.')))
      .map((file) => file.name);
    if (activeFilePane === 'local') app.localSelected = new Set(names);
    else app.remoteSelected = new Set(names);
  }

  // Global keyboard shortcuts — every binding advertised in the UI exists.
  function onKeydown(e: KeyboardEvent) {
    const mod = e.metaKey || e.ctrlKey;
    if (mod && e.key.toLowerCase() === 'a') {
      const target = e.target;
      if (!activeFilePane || (target instanceof HTMLElement && target.closest('input, textarea, select, [contenteditable="true"]'))) return;
      e.preventDefault();
      selectAllInActivePane();
    } else if (mod && e.key === '1') {
      e.preventDefault();
      app.setView('connections');
    } else if (mod && e.key === '2') {
      e.preventDefault();
      app.setView('transfers');
    } else if (mod && e.key.toLowerCase() === 'n') {
      e.preventDefault();
      app.quickConnectMode = 'new';
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

  // Restore the pane split from the previous session (old unclamped key
  // from the first splitter build is ignored).
  if (browser) {
    const saved = Number(localStorage.getItem('flowftp:pane-ratio-v2'));
    if (!Number.isNaN(saved) && saved >= 0.15 && saved <= 0.85) app.setPaneRatio(saved);
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

  let localListSeq = 0;

  async function loadLocal(path = app.localPath) {
    const seq = ++localListSeq;
    app.localLoading = true;
    try {
      const files = await listLocal(path);
      if (seq !== localListSeq) return;
      app.localFiles = files;
      app.errors.local = null;
    } catch (e) {
      if (seq !== localListSeq) return;
      app.errors.local = e instanceof Error ? e.message : String(e);
    } finally {
      if (seq === localListSeq) app.localLoading = false;
    }
  }

  // Local listing follows the local path (and refreshes after mutations).
  $effect(() => {
    if (!IS_TAURI) return;
    const path = app.localPath;
    void app.refreshTick;
    untrack(() => {
      void loadLocal(path);
    });
  });

  // Remote listing is EXPLICIT: navigate lists first and only commits the
  // new path on success, so a failure leaves you in the current folder with
  // its listing intact (Finder behavior). Effects only handle connection
  // changes and manual refreshes.

  // Latest-wins guard for in-flight listings: the initial listing after a
  // connect can still be travelling when the user double-clicks into a
  // folder. Without the guard the slower response commits last and paints
  // the parent directory's rows under the child's breadcrumb — and the next
  // double-click then targets a doubled path that doesn't exist.
  let remoteListSeq = 0;

  async function loadRemote() {
    const connectionId = app.activeConnectionId;
    const path = app.remotePath;
    if (!connectionId) return;
    const seq = ++remoteListSeq;
    app.remoteLoading = true;
    try {
      const files = await listRemote(connectionId, path);
      if (seq !== remoteListSeq) return;
      app.remoteFiles = files;
      app.errors.remote = null;
    } catch (e) {
      if (seq !== remoteListSeq) return;
      app.errors.remote = e instanceof Error ? e.message : String(e);
      app.remoteFiles = [];
      app.remoteSelected = new Set();
      if (app.inspector?.side === 'remote') app.inspector = null;
      reactToRemoteOpError(e, connectionId);
    } finally {
      if (seq === remoteListSeq) app.remoteLoading = false;
    }
  }

  /** List `target`; commit the path only when the listing succeeds. */
  async function goRemote(target: string) {
    const connectionId = app.activeConnectionId;
    if (!connectionId || target === app.remotePath) return;
    const seq = ++remoteListSeq;
    app.remoteLoading = true;
    try {
      const files = await listRemote(connectionId, target);
      if (seq !== remoteListSeq) return;
      app.remotePath = target;
      app.remoteFiles = files;
      app.remoteSelected = new Set();
      app.errors.remote = null;
      if (app.inspector?.side === 'remote') app.inspector = null;
    } catch (e) {
      if (seq !== remoteListSeq) return;
      // Stay in the current folder; surface why the target is unavailable.
      app.errors.remote = e instanceof Error ? e.message : String(e);
      reactToRemoteOpError(e, connectionId);
    } finally {
      if (seq === remoteListSeq) app.remoteLoading = false;
    }
  }

  /** A timeout closes the server-side session (see `OP_TIMEOUT` in the
   * bridge), so the client must drop its connected state too — otherwise
   * every later op fails on an already-evicted connection. */
  function reactToRemoteOpError(e: unknown, connectionId: string) {
    if (e instanceof IpcError && e.code === 'timeout') {
      app.markDisconnected(connectionId);
      app.notify('danger', 'Connection lost', e.message);
    }
  }

  /** Double-click navigation: '..' goes up, a directory enters. */
  function navigateRemote(name: string) {
    if (name === '..') {
      const parts = app.remotePath.split('/').filter(Boolean);
      parts.pop();
      void goRemote('/' + parts.join('/'));
    } else {
      void goRemote(joinPath(app.remotePath, name));
    }
  }

  // Connection changes: fresh session starts at root with a clean slate;
  // disconnected clears the remote pane and breadcrumb. Stale rows from a
  // previous session must not linger while the first listing travels.
  $effect(() => {
    if (!IS_TAURI) return;
    const connectionId = app.activeConnectionId;
    app.remoteFiles = [];
    app.remoteSelected = new Set();
    app.errors.remote = null;
    app.remoteLoading = false;
    untrack(() => {
      if (app.inspector?.side === 'remote') app.inspector = null;
    });
    app.remotePath = '/';
    if (!connectionId) return;
    untrack(() => {
      void loadRemote();
    });
  });

  // Manual refreshes (mutations bump refreshTick).
  $effect(() => {
    void app.refreshTick;
    if (!IS_TAURI) return;
    untrack(() => {
      if (app.activeConnectionId) void loadRemote();
    });
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

  // Use pointer events for transfers between the two panes. Tauri's macOS
  // native file-drop destination also claims WebKit drag sessions, so HTML
  // DataTransfer drops do not reliably reach the pane handlers.
  type InternalDropTarget = {
    side: 'local' | 'remote';
    folder: string | null;
    targetDir: string;
  };
  let internalFileDrag = $state<{
    pointerId: number;
    side: 'local' | 'remote';
    names: string[];
    startX: number;
    startY: number;
    pointerX: number;
    pointerY: number;
    active: boolean;
  } | null>(null);
  let internalDropTarget = $state<InternalDropTarget | null>(null);
  let suppressClickAfterFileDrag = false;

  function startInternalFileDrag(e: PointerEvent) {
    if (!e.isPrimary || e.button !== 0) return;
    const entry = e.target instanceof Element
      ? e.target.closest<HTMLElement>('[data-drag-entry]')
      : null;
    const pane = entry?.closest<HTMLElement>('[data-file-pane]');
    const name = entry?.dataset.dragEntry;
    const side = pane?.dataset.filePane;
    if (!entry || !name || (side !== 'local' && side !== 'remote')) return;
    if (side === 'remote' && !app.activeConnectionId) return;

    const sourceFiles = side === 'local' ? app.localFiles : app.remoteFiles;
    const selected = side === 'local' ? app.localSelected : app.remoteSelected;
    const fileNames = new Set(sourceFiles.filter((file) => file.kind !== 'directory').map((file) => file.name));
    const names = selected.has(name)
      ? [...selected].filter((selectedName) => fileNames.has(selectedName))
      : [name];
    if (names.length === 0) return;

    internalFileDrag = {
      pointerId: e.pointerId,
      side,
      names,
      startX: e.clientX,
      startY: e.clientY,
      pointerX: e.clientX,
      pointerY: e.clientY,
      active: false
    };
  }

  function findInternalDropTarget(
    clientX: number,
    clientY: number,
    sourceSide: 'local' | 'remote'
  ): InternalDropTarget | null {
    const hovered = document.elementFromPoint(clientX, clientY);
    const pane = hovered?.closest<HTMLElement>('[data-file-pane]');
    const side = pane?.dataset.filePane;
    if (!pane || (side !== 'local' && side !== 'remote') || side === sourceSide) return null;
    if ((side === 'remote' || sourceSide === 'remote') && !app.activeConnectionId) return null;

    const folderRow = hovered?.closest<HTMLElement>('[data-drop-folder]');
    const folder = folderRow?.dataset.dropFolder ?? null;
    const currentPath = pane.dataset.filePath ?? '/';
    return {
      side,
      folder,
      targetDir: folder ? joinPath(currentPath, folder) : currentPath
    };
  }

  function moveInternalFileDrag(e: PointerEvent) {
    const drag = internalFileDrag;
    if (!drag || drag.pointerId !== e.pointerId) return;
    let activeDrag = drag;
    if (!drag.active) {
      if (Math.hypot(e.clientX - drag.startX, e.clientY - drag.startY) < 6) return;
      activeDrag = { ...drag, active: true };
    }
    e.preventDefault();
    activeDrag = { ...activeDrag, pointerX: e.clientX, pointerY: e.clientY };
    internalFileDrag = activeDrag;
    const target = findInternalDropTarget(e.clientX, e.clientY, activeDrag.side);
    internalDropTarget = target;
  }

  function finishInternalFileDrag(e: PointerEvent) {
    const drag = internalFileDrag;
    if (!drag || drag.pointerId !== e.pointerId) return;
    internalFileDrag = null;
    internalDropTarget = null;
    if (!drag.active) return;

    suppressClickAfterFileDrag = true;
    setTimeout(() => (suppressClickAfterFileDrag = false), 0);
    const target = findInternalDropTarget(e.clientX, e.clientY, drag.side);
    if (!target) return;

    if (drag.side === 'local') {
      handleDropLocalFiles(
        drag.names.map((fileName) => ({
          fileName,
          localPath: joinPath(app.localPath, fileName)
        })),
        target.targetDir
      );
    } else {
      handleDropRemoteEntries(
        drag.names.map((name) => ({ name, isDir: false })),
        target.targetDir
      );
    }
  }

  function cancelInternalFileDrag(e: PointerEvent) {
    if (internalFileDrag?.pointerId !== e.pointerId) return;
    internalFileDrag = null;
    internalDropTarget = null;
  }

  function suppressDraggedFileClick(e: MouseEvent) {
    if (!suppressClickAfterFileDrag) return;
    e.preventDefault();
    e.stopPropagation();
    suppressClickAfterFileDrag = false;
  }

  // ── Pane splitter ─────────────────────────────────────────────────────────

  /** Inspector column (w-72 = 288px) plus its border, and the 6px splitter. */
  const INSPECTOR_WIDTH = 289;
  const SPLITTER_WIDTH = 6;
  let paneRow = $state<HTMLDivElement | null>(null);
  let draggingPane = $state(false);

  function startPaneDrag(e: PointerEvent) {
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    draggingPane = true;
  }

  function movePaneDrag(e: PointerEvent) {
    if (!draggingPane || !paneRow) return;
    const rect = paneRow.getBoundingClientRect();
    const area = rect.width - INSPECTOR_WIDTH - SPLITTER_WIDTH;
    if (area <= 0) return;
    app.setPaneRatio((e.clientX - rect.left) / area);
  }

  function endPaneDrag() {
    if (!draggingPane) return;
    draggingPane = false;
    localStorage.setItem('flowftp:pane-ratio-v2', String(app.paneRatio));
  }

  /** Double-click the splitter to restore the even split. */
  function resetPaneSplit() {
    app.setPaneRatio(0.5);
    localStorage.setItem('flowftp:pane-ratio-v2', '0.5');
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

<svelte:window
  onkeydown={onKeydown}
  onpointerdown={(e) => {
    setActiveFilePane(e.target);
    startInternalFileDrag(e);
  }}
  onfocusin={(e) => setActiveFilePane(e.target)}
  onpointermove={moveInternalFileDrag}
  onpointerup={finishInternalFileDrag}
  onpointercancel={cancelInternalFileDrag}
  onclickcapture={suppressDraggedFileClick}
/>

<div class="flex h-full w-full flex-col overflow-hidden bg-bg" class:flowftp-file-dragging={internalFileDrag?.active}>
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
      <div
        class="flex min-h-0 flex-1"
        bind:this={paneRow}
      >
        <div
          class="flex min-h-0"
          style="flex: 0 1 calc((100% - {INSPECTOR_WIDTH + 6}px) * {app.paneRatio}); min-width: 300px"
        >
        <FilePane
          side="local"
          title="Local"
          files={app.localFiles}
          path={app.localPath}
          selected={app.localSelected}
          onSelect={app.selectLocal.bind(app)}
          showHidden={app.showHidden}
          loading={app.localLoading}
          onNavigate={navigateLocal}
          onNavigateTo={(path) => (app.localPath = path)}
          onRefresh={() => void loadLocal()}
          onUpload={handleUpload}
          onCreateFolder={(name) => handleCreateFolder('local', name)}
          onRename={(from, to) => handleRename('local', from, to)}
          onDelete={(names) => handleDelete('local', names)}
          internalDropTarget={internalDropTarget}
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
        <div class="flex min-w-0 flex-1" style="min-width: 260px" bind:this={remotePaneEl}>
        <FilePane
          side="remote"
          title="Remote"
          files={app.remoteFiles}
          path={app.remotePath}
          selected={app.remoteSelected}
          onSelect={app.selectRemote.bind(app)}
          showHidden={app.showHidden}
          loading={app.remoteLoading}
          connectionName={app.activeConnection?.name}
          onNavigate={navigateRemote}
          onNavigateTo={(path) => void goRemote(path)}
          onRefresh={() => void loadRemote()}
          onDownload={handleDownload}
          onEdit={handleEdit}
          disconnected={!app.activeConnectionId}
          onConnect={() => {
            app.quickConnectMode = 'quick';
            app.quickConnectOpen = true;
          }}
          onCreateFolder={(name) => handleCreateFolder('remote', name)}
          onRename={(from, to) => handleRename('remote', from, to)}
          onDelete={(names) => handleDelete('remote', names)}
          internalDropTarget={internalDropTarget}
          onDisconnect={() => app.disconnectActive()}
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

{#if internalFileDrag?.active}
  <div
    class="pointer-events-none fixed z-[100] flex max-w-72 items-center gap-2.5 rounded-xl border border-accent/40 bg-bg-elevated px-3 py-2 shadow-lg ring-1 ring-accent/15"
    style={`left: ${Math.max(8, Math.min(internalFileDrag.pointerX + 16, window.innerWidth - 288))}px; top: ${Math.max(8, Math.min(internalFileDrag.pointerY + 16, window.innerHeight - 76))}px`}
    aria-hidden="true"
  >
    <span class="grid h-8 w-8 shrink-0 place-items-center rounded-lg bg-accent-solid text-accent-fg shadow-sm">
      {#if internalFileDrag.side === 'local'}
        <IconUpload size={15} />
      {:else}
        <IconDownload size={15} />
      {/if}
    </span>
    <span class="min-w-0 flex-1">
      <span class="block text-[10px] font-semibold uppercase tracking-wider text-accent-text">
        {internalFileDrag.side === 'local' ? 'Upload' : 'Download'} · Copy
      </span>
      <span class="block max-w-52 truncate text-xs font-medium text-fg">
        {internalFileDrag.names.length === 1
          ? internalFileDrag.names[0]
          : `${internalFileDrag.names.length} files`}
      </span>
      <span class="block max-w-52 truncate text-[10px] text-fg-subtle">
        {internalDropTarget
          ? `Drop in ${internalDropTarget.targetDir}`
          : `Drop in ${internalFileDrag.side === 'local' ? 'Remote' : 'Local'} pane`}
      </span>
    </span>
  </div>
{/if}

<!-- Global overlays -->
<CommandPalette />
<QuickConnect />
<SyncPreview />
<ToastHost />
<QuitConfirmDialog />

<!-- Delete confirmation: Preview → Confirm → Execute (DESIGN.md) -->
<Modal
  open={deleteConfirm !== null}
  title="Delete {deleteConfirm?.names.length ?? 0}
    item{deleteConfirm?.names.length === 1 ? '' : 's'}?"
  center
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
  center
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
