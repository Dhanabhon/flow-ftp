<script lang="ts">
  import { app } from '$lib/stores/app.svelte';
  import Header from '$lib/components/shell/Header.svelte';
  import Sidebar from '$lib/components/shell/Sidebar.svelte';
  import FilePane from '$lib/components/shell/FilePane.svelte';
  import TransferQueue from '$lib/components/shell/TransferQueue.svelte';
  import PreviewPanel from '$lib/components/shell/PreviewPanel.svelte';
  import CommandPalette from '$lib/components/overlays/CommandPalette.svelte';
  import QuickConnect from '$lib/components/overlays/QuickConnect.svelte';
  import SyncPreview from '$lib/components/overlays/SyncPreview.svelte';

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
