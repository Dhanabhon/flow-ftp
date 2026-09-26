import type { Connection, RemoteFile, Transfer, View } from '$lib/types';
import { mockConnections, mockLocalFiles, mockRemoteFiles, mockTransfers } from '$lib/mock';

/** A transient notification. */
export interface Toast {
  id: string;
  kind: 'success' | 'warning' | 'danger';
  title: string;
  detail?: string;
}

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
  /**
   * Which entry point opened the connect form: an ad-hoc Quick Connect
   * (nothing is saved) or a New Connection (profile persisted on success).
   */
  quickConnectMode = $state<'quick' | 'new'>('quick');
  /** Fields prefilling the Quick Connect form (from a saved profile). */
  quickConnectPrefill = $state<{
    protocol: Connection['protocol'];
    host: string;
    port: number;
    username: string;
  } | null>(null);

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

  // ── Inspector ─────────────────────────────────────────────────
  /** The pane + entry the inspector is showing, null when nothing selected. */
  inspector = $state<{ side: 'local' | 'remote'; name: string } | null>(null);

  // ── Toasts ────────────────────────────────────────────────────
  /** Transient notifications, bottom-right per DESIGN.md. */
  toasts = $state<Toast[]>([]);

  /** Push a toast; auto-dismisses after 4 seconds. */
  notify(kind: Toast['kind'], title: string, detail?: string) {
    const id = crypto.randomUUID();
    this.toasts = [...this.toasts, { id, kind, title, detail }];
    setTimeout(() => this.dismissToast(id), 4_000);
  }

  dismissToast(id: string) {
    this.toasts = this.toasts.filter((t) => t.id !== id);
  }

  // ── Transfers ───────────────────────────────────────────────
  transfers = $state<Transfer[]>(mockTransfers);
  /** Bumped after create/rename/delete so listing effects refetch. */
  refreshTick = $state(0);
  /** Local pane share of the two-pane area (0.15-0.85); 0.5 = even split. */
  paneRatio = $state(0.5);

  /** Clamp and apply a new local-pane ratio. */
  setPaneRatio(ratio: number) {
    this.paneRatio = Math.min(0.85, Math.max(0.15, ratio));
  }
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

  /**
   * Disconnect the active session: tear down the backend connection, reset
   * the remote pane, and update the connection list.
   */
  async disconnectActive() {
    const id = this.activeConnectionId;
    if (!id) return;
    const { disconnect } = await import('$lib/ipc');
    await disconnect(id).catch(() => {});
    this.markDisconnected(id);
    this.remoteFiles = [];
    this.remoteSelected = new Set();
    this.errors.remote = null;
    if (this.inspector?.side === 'remote') this.inspector = null;
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
    this.inspector = next.size > 0 ? { side: 'local', name } : null;
  }

  selectRemote(name: string, additive = false) {
    const next = additive ? new Set(this.remoteSelected) : new Set<string>();
    if (next.has(name)) next.delete(name);
    else next.add(name);
    this.remoteSelected = next;
    this.inspector = next.size > 0 ? { side: 'remote', name } : null;
  }

  setView(v: View) {
    this.view = v;
  }

  openCommandPalette() {
    this.commandPaletteOpen = true;
  }
}

export const app = new AppState();
