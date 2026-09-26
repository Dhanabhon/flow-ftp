<script lang="ts">
  import { app } from '$lib/stores/app.svelte';
  import { cn } from '$lib/utils';
  import { connect, IS_TAURI, profileSave } from '$lib/ipc';
  import type { Protocol } from '$lib/types';
  import Modal from '$lib/components/ui/modal.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import Badge from '$lib/components/ui/badge.svelte';
  import { IconZap, IconLock, IconGlobe, IconServer, IconAlert, IconLoader, IconEye, IconEyeOff } from '$lib/components/icons';

  let protocol = $state<Protocol>('sftp');
  let host = $state('');
  let port = $state(22);
  let username = $state('');
  let password = $state('');
  let saveToKeychain = $state(true);
  let saveProfile = $state(true);
  let showPassword = $state(false);
  let connecting = $state(false);
  let errorMessage = $state<string | null>(null);

  const protocols: { id: Protocol; label: string; desc: string; port: number; secure: boolean }[] = [
    { id: 'sftp', label: 'SFTP', desc: 'SSH File Transfer', port: 22, secure: true },
    { id: 'ftps', label: 'FTPS', desc: 'FTP over TLS/SSL', port: 990, secure: true },
    { id: 'ftp', label: 'FTP', desc: 'Plain FTP', port: 21, secure: false }
  ];

  function chooseProtocol(p: (typeof protocols)[number]) {
    protocol = p.id;
    port = p.port;
  }

  // Prefill from a saved profile when the modal opens via the sidebar.
  $effect(() => {
    if (!app.quickConnectOpen) return;
    const prefill = app.quickConnectPrefill;
    if (prefill) {
      protocol = prefill.protocol;
      host = prefill.host;
      port = prefill.port;
      username = prefill.username;
      app.quickConnectPrefill = null;
    }
  });

  /** Establish the session (real backend under Tauri, mock in the browser). */
  async function submit() {
    if (!host.trim() || connecting) return;
    connecting = true;
    errorMessage = null;
    try {
      // Reuse the profile id when reconnecting to an existing saved one,
      // so the keychain entry and profile stay stable.
      const existing = app.connections.find(
        (c) => c.host === host.trim() && c.port === port && c.username === (username.trim() || 'anonymous')
      );
      const id = existing?.id ?? crypto.randomUUID();
      const connection = await connect({
        id,
        protocol,
        host: host.trim(),
        port,
        username: username.trim() || 'anonymous',
        password,
        saveKeychain: saveToKeychain
      });
      app.upsertConnection(connection);
      // Persist the profile only when this is a New Connection (and wanted).
      // A Quick Connect stays ad-hoc: active now, gone from the sidebar later.
      if (IS_TAURI && app.quickConnectMode === 'new' && saveProfile) {
        profileSave(connection)
          .then((profiles) => (app.connections = profiles))
          .catch(() => {});
      }
      app.remotePath = '/';
      app.quickConnectOpen = false;
      // Reset the form for the next connect.
      host = '';
      username = '';
      password = '';
    } catch (e) {
      errorMessage = e instanceof Error ? e.message : String(e);
    } finally {
      connecting = false;
    }
  }
</script>

<Modal
  bind:open={app.quickConnectOpen}
  title={app.quickConnectMode === 'new' ? 'New Connection' : 'Quick Connect'}
  description={app.quickConnectMode === 'new'
    ? 'Set up a saved connection. The profile is kept for reuse; the password goes into macOS Keychain.'
    : 'Connect right now without saving the connection. The password can still go into macOS Keychain.'}
  width="md"
>
  <!-- Protocol selector -->
  <div class="mb-4 grid grid-cols-3 gap-2">
    {#each protocols as p (p.id)}
      <button
        class={cn(
          'flex flex-col items-start rounded-lg border p-3 text-left transition-colors',
          protocol === p.id
            ? 'border-accent bg-accent/10 text-fg'
            : 'border-border bg-bg-panel text-fg-muted hover:border-border-strong hover:bg-bg-hover'
        )}
        onclick={() => chooseProtocol(p)}
      >
        <div class="flex w-full items-center justify-between">
          <span class="text-sm font-semibold">{p.label}</span>
          {#if p.secure}
            <IconLock size={12} class="text-success" />
          {/if}
        </div>
        <span class="mt-0.5 text-[11px] text-fg-subtle">{p.desc}</span>
        <span class="mt-1 font-mono text-[10px] text-fg-faint">:{p.port}</span>
      </button>
    {/each}
  </div>

  <!-- Host / port -->
  <div class="mb-3 grid grid-cols-[1fr_100px] gap-2">
    <label class="block">
      <span class="mb-1 block text-xs font-medium text-fg-muted">Host</span>
      <div class="flex items-center gap-2 rounded-md border border-border bg-bg px-3 py-2 focus-within:border-accent">
        <IconGlobe size={14} class="text-fg-subtle" />
        <input
          class="flex-1 bg-transparent text-sm text-fg outline-none placeholder:text-fg-faint"
          placeholder="ftp.example.com"
          bind:value={host}
        />
      </div>
    </label>
    <label class="block">
      <span class="mb-1 block text-xs font-medium text-fg-muted">Port</span>
      <input
        type="number"
        class="w-full rounded-md border border-border bg-bg px-3 py-2 text-sm text-fg outline-none focus:border-accent"
        bind:value={port}
      />
    </label>
  </div>

  <!-- Username / password -->
  <div class="mb-3 grid grid-cols-2 gap-2">
    <label class="block">
      <span class="mb-1 block text-xs font-medium text-fg-muted">Username</span>
      <div class="flex items-center gap-2 rounded-md border border-border bg-bg px-3 py-2 focus-within:border-accent">
        <IconServer size={14} class="text-fg-subtle" />
        <input
          class="flex-1 bg-transparent text-sm text-fg outline-none placeholder:text-fg-faint"
          placeholder="username"
          bind:value={username}
        />
      </div>
    </label>
    <label class="block">
      <span class="mb-1 block text-xs font-medium text-fg-muted">Password</span>
      <div class="flex items-center gap-1 rounded-md border border-border bg-bg pr-1 pl-3 focus-within:border-accent">
        <input
          type={showPassword ? 'text' : 'password'}
          class="min-w-0 flex-1 bg-transparent py-2 text-sm text-fg outline-none placeholder:text-fg-faint"
          placeholder="••••••••"
          bind:value={password}
        />
        <button
          type="button"
          class="shrink-0 rounded p-1.5 text-fg-subtle transition-colors hover:bg-bg-hover hover:text-fg"
          aria-label={showPassword ? 'Hide password' : 'Show password'}
          title={showPassword ? 'Hide password' : 'Show password'}
          onclick={() => (showPassword = !showPassword)}
        >
          {#if showPassword}
            <IconEyeOff size={14} />
          {:else}
            <IconEye size={14} />
          {/if}
        </button>
      </div>
    </label>
  </div>

  <!-- Save profile: New Connection mode only (Quick Connect is ad-hoc) -->
  {#if app.quickConnectMode === 'new'}
    <label class="mb-3 flex cursor-pointer items-center gap-2.5 rounded-md border border-border bg-bg-panel p-3">
      <input type="checkbox" bind:checked={saveProfile} class="h-4 w-4 accent-[var(--color-accent)]" />
      <div class="flex-1">
        <div class="flex items-center gap-1.5">
          <IconServer size={13} class="text-accent-text" />
          <span class="text-sm font-medium text-fg">Save this connection</span>
        </div>
        <p class="text-[11px] text-fg-subtle">Keep it in the sidebar for one-click reconnects.</p>
      </div>
    </label>
  {/if}

  <!-- Keychain toggle -->
  <label class="flex cursor-pointer items-center gap-2.5 rounded-md border border-border bg-bg-panel p-3">
    <input type="checkbox" bind:checked={saveToKeychain} class="h-4 w-4 accent-[var(--color-accent)]" />
    <div class="flex-1">
      <div class="flex items-center gap-1.5">
        <IconLock size={13} class="text-success" />
        <span class="text-sm font-medium text-fg">Save to Keychain</span>
      </div>
      <p class="text-[11px] text-fg-subtle">Encrypt and store credentials securely via macOS Keychain.</p>
    </div>
    {#if saveToKeychain}
      <Badge variant="success" dot>Secured</Badge>
    {/if}
  </label>

  <!-- Connection error, with suggested fix per the UX error rules -->
  {#if errorMessage}
    <div role="alert" class="mt-3 flex items-start gap-2 rounded-md border border-danger/40 bg-danger/10 p-2.5">
      <IconAlert size={14} class="mt-0.5 shrink-0 text-danger" />
      <span class="text-xs text-danger">{errorMessage}</span>
    </div>
  {/if}

  {#snippet footer()}
    <Button variant="ghost" onclick={() => (app.quickConnectOpen = false)}>Cancel</Button>
    <Button onclick={submit} disabled={connecting || !host.trim()}>
      {#if connecting}
        <IconLoader size={14} class="animate-spin" /> Connecting…
      {:else}
        <IconZap size={14} /> Connect
      {/if}
    </Button>
  {/snippet}
</Modal>
