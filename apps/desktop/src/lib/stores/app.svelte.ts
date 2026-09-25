import type { Connection, RemoteFile, Transfer, View } from '$lib/types';
import { mockConnections, mockLocalFiles, mockRemoteFiles, mockTransfers } from '$lib/mock';

/**
 * Global app store using Svelte 5 runes ($state).
 * In production these would be populated via TanStack Query + Tauri invoke();
 * for the mock we seed them directly.
 */
class AppState {
  // ── Navigation ──────────────────────────────────────────────
  view = $state<View>('connections');
  commandPaletteOpen = $state(false);
  quickConnectOpen = $state(false);
  syncOpen = $state(false);

  // ── Connections ─────────────────────────────────────────────
  connections = $state<Connection[]>(mockConnections);
  activeConnectionId = $state<string | null>('c1');

  get activeConnection() {
    return this.connections.find((c) => c.id === this.activeConnectionId) ?? null;
  }

  // ── File browser (dual pane) ────────────────────────────────
  localFiles = $state<RemoteFile[]>(mockLocalFiles);
  remoteFiles = $state<RemoteFile[]>(mockRemoteFiles);
  localPath = $state('/Users/tom/Documents');
  remotePath = $state('/home/tom');
  localSelected = $state<Set<string>>(new Set());
  remoteSelected = $state<Set<string>>(new Set());
  showHidden = $state(false);
  /** Last pane-level error per side, null when clear. */
  errors = $state<{ local: string | null; remote: string | null }>({
    local: null,
    remote: null
  });

  // ── Transfers ───────────────────────────────────────────────
  transfers = $state<Transfer[]>(mockTransfers);
  queueCollapsed = $state(false);

  get activeTransfers() {
    return this.transfers.filter((t) => t.status === 'active');
  }
  get queuedCount() {
    return this.transfers.filter((t) => t.status === 'queued').length;
  }
  get failedCount() {
    return this.transfers.filter((t) => t.status === 'failed').length;
  }

  // ── Actions ─────────────────────────────────────────────────
  /** Register (or update) a connection and make it active. */
  upsertConnection(connection: Connection) {
    const index = this.connections.findIndex((c) => c.id === connection.id);
    if (index >= 0) {
      this.connections[index] = connection;
    } else {
      this.connections = [...this.connections, connection];
    }
    this.activeConnectionId = connection.id;
  }

  /** Mark a connection disconnected and drop it from the active slot. */
  markDisconnected(connectionId: string) {
    this.connections = this.connections.map((c) =>
      c.id === connectionId ? { ...c, status: 'disconnected' as const } : c
    );
    if (this.activeConnectionId === connectionId) {
      this.activeConnectionId = null;
    }
  }

  toggleHidden() {
    this.showHidden = !this.showHidden;
  }

  toggleQueue() {
    this.queueCollapsed = !this.queueCollapsed;
  }

  selectLocal(name: string, additive = false) {
    const next = additive ? new Set(this.localSelected) : new Set<string>();
    if (next.has(name)) next.delete(name);
    else next.add(name);
    this.localSelected = next;
  }

  selectRemote(name: string, additive = false) {
    const next = additive ? new Set(this.remoteSelected) : new Set<string>();
    if (next.has(name)) next.delete(name);
    else next.add(name);
    this.remoteSelected = next;
  }

  setView(v: View) {
    this.view = v;
  }

  openCommandPalette() {
    this.commandPaletteOpen = true;
  }
}

export const app = new AppState();
