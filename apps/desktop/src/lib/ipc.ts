/**
 * Typed bridge to the Rust application layer.
 *
 * Every function works in both environments:
 *  - under Tauri → the real `invoke()` call to the command bridge
 *  - in a plain browser (`pnpm dev`) → the mock data fallback, so UI
 *    development never needs the native shell
 *
 * Error shape mirrors `bridge::IpcError` in `src-tauri/src/bridge.rs`.
 */
import { invoke } from '@tauri-apps/api/core';

import type { Connection, Protocol, RemoteFile, SyncDiff, SyncDirection, Transfer } from './types';
import { mockTransfers } from './mock';
import { mockLocalFiles, mockRemoteFiles } from './mock';

/** True when running inside the Tauri webview (vs a plain browser). */
export const IS_TAURI: boolean =
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

/** Machine-readable error class from the Rust `IpcError`. */
export type IpcErrorCode =
  | 'connection-failed'
  | 'auth-failed'
  | 'not-found'
  | 'permission'
  | 'protocol'
  | 'io'
  | 'invalid-path'
  | 'timeout'
  | 'transfer';

/** Structured error surfaced by every bridge call. */
export class IpcError extends Error {
  readonly code: IpcErrorCode;

  constructor(code: IpcErrorCode, message: string) {
    super(message);
    this.name = 'IpcError';
    this.code = code;
  }
}

interface RawIpcError {
  code?: string;
  message?: string;
}

/** Rehydrate thrown values into typed `IpcError`s. */
function normalizeError(e: unknown): IpcError {
  if (e instanceof IpcError) return e;
  const raw = (e ?? {}) as RawIpcError;
  return new IpcError(
    (raw.code as IpcErrorCode) ?? 'protocol',
    raw.message ?? String(e),
  );
}

/** Call a command, converting failures into typed `IpcError`s. */
async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (e) {
    throw normalizeError(e);
  }
}

/** Run the mock fallback when no native shell is present. */
function mockOr<T>(tauri: () => Promise<T>, mock: () => T): Promise<T> {
  if (IS_TAURI) return tauri();
  return Promise.resolve(mock());
}

// ─────────────────────────────────────────────────────────────────────────────
// Connections
// ─────────────────────────────────────────────────────────────────────────────

/** Parameters for establishing a remote session. */
export interface ConnectParams {
  /** Load the password from the OS keychain (saved profiles) instead. */
  useKeychainPassword?: boolean;
  /** Client-generated unique id (UUID). */
  id: string;
  protocol: Protocol;
  host: string;
  port: number;
  username: string;
  password: string;
  /** Persist the password in the OS keychain. */
  saveKeychain: boolean;
}

/** Establish a remote session. Returns the connection record to display. */
export function connect(params: ConnectParams): Promise<Connection> {
  return mockOr(
    () => call<Connection>('remote_connect', { request: { ...params, useKeychainPassword: params.useKeychainPassword ?? false } }),
    () => ({
      id: params.id,
      name: params.host,
      protocol: params.protocol,
      host: params.host,
      port: params.port,
      username: params.username,
      status: 'connected',
      keychain: params.saveKeychain,
      favorite: false,
      lastConnected: Date.now(),
    }),
  );
}

/** Tear down a remote session. */
export function disconnect(connectionId: string): Promise<void> {
  return mockOr(
    () => call<void>('remote_disconnect', { connectionId }),
    () => undefined,
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Saved connection profiles
// ─────────────────────────────────────────────────────────────────────────────

/** All saved profiles (passwords are never included). */
export function profileList(): Promise<Connection[]> {
  return mockOr(
    () => call<Connection[]>('profile_list'),
    () => [],
  );
}

/** Upsert a profile and persist; returns the fresh list. */
export function profileSave(profile: Connection): Promise<Connection[]> {
  return mockOr(
    () => call<Connection[]>('profile_save', { profile }),
    () => [profile],
  );
}

/** Delete a profile; returns the fresh list. */
export function profileDelete(id: string): Promise<Connection[]> {
  return mockOr(
    () => call<Connection[]>('profile_delete', { id }),
    () => [],
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Remote filesystem
// ─────────────────────────────────────────────────────────────────────────────

/** List a remote directory (parent `..` entry included off-root). */
export function listRemote(connectionId: string, path: string): Promise<RemoteFile[]> {
  return mockOr(
    () => call<RemoteFile[]>('remote_list', { connectionId, path }),
    () => mockRemoteFiles,
  );
}

/** Stat a single remote path (preview panel). */
export function statRemote(connectionId: string, path: string): Promise<RemoteFile> {
  return mockOr(
    () => call<RemoteFile>('remote_stat', { connectionId, path }),
    () => mockRemoteFiles[1],
  );
}

/** Create a remote directory. */
export function mkdirRemote(connectionId: string, path: string): Promise<void> {
  return mockOr(
    () => call<void>('remote_mkdir', { connectionId, path }),
    () => undefined,
  );
}

/** Rename/move a remote path. */
export function renameRemote(
  connectionId: string,
  from: string,
  to: string,
): Promise<void> {
  return mockOr(
    () => call<void>('remote_rename', { connectionId, from, to }),
    () => undefined,
  );
}

/** Delete a remote file or empty directory. */
export function deleteRemote(connectionId: string, path: string): Promise<void> {
  return mockOr(
    () => call<void>('remote_delete', { connectionId, path }),
    () => undefined,
  );
}

/** Upload a local file to a remote path. */
export function uploadFile(
  connectionId: string,
  localPath: string,
  remotePath: string,
): Promise<void> {
  return mockOr(
    () => call<void>('remote_upload', { connectionId, localPath, remotePath }),
    () => undefined,
  );
}

/** Download a remote file to a local path. */
export function downloadFile(
  connectionId: string,
  remotePath: string,
  localPath: string,
): Promise<void> {
  return mockOr(
    () => call<void>('remote_download', { connectionId, remotePath, localPath }),
    () => undefined,
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Local filesystem
// ─────────────────────────────────────────────────────────────────────────────

/** The user's home directory (local pane start location). */
export function localHome(): Promise<string> {
  return mockOr(
    () => call<string>('local_home'),
    () => '/Users/you/',
  );
}

/** List a local directory. */
export function listLocal(path: string): Promise<RemoteFile[]> {
  return mockOr(
    () => call<RemoteFile[]>('local_list', { path }),
    () => mockLocalFiles,
  );
}

/** Create a local directory (parents included). */
export function localMkdir(path: string): Promise<void> {
  return mockOr(
    () => call<void>('local_mkdir', { path }),
    () => undefined,
  );
}

/** Rename/move a local path. */
export function localRename(from: string, to: string): Promise<void> {
  return mockOr(
    () => call<void>('local_rename', { from, to }),
    () => undefined,
  );
}

/** Read the start of a local text file for the preview (null = not readable). */
export function localReadText(
  path: string
): Promise<{ content: string; truncated: boolean } | null> {
  return mockOr(
    () => call<{ content: string; truncated: boolean } | null>('local_read_text', { path }),
    () => null,
  );
}

/** Delete a local file or empty directory. */
export function localDelete(path: string): Promise<void> {
  return mockOr(
    () => call<void>('local_delete', { path }),
    () => undefined,
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Transfer queue
// ─────────────────────────────────────────────────────────────────────────────

/** Enqueue one file transfer on the engine's queue. */
export function enqueueTransfer(params: {
  id: string;
  connectionId: string;
  direction: Transfer['direction'];
  remotePath: string;
  localPath: string;
  fileName: string;
}): Promise<Transfer> {
  return mockOr(
    () => call<Transfer>('transfer_enqueue', { request: params }),
    () => ({
      id: params.id,
      fileName: params.fileName,
      direction: params.direction,
      size: 0,
      transferred: 0,
      speed: 0,
      status: 'queued',
      connectionId: params.connectionId,
      connectionName: '',
      remotePath: params.remotePath,
      localPath: params.localPath,
      resumable: true,
    }),
  );
}

/** Snapshot of every tracked transfer. */
export function transferList(): Promise<Transfer[]> {
  return mockOr(
    () => call<Transfer[]>('transfer_list'),
    () => [],
  );
}

/** Pause a transfer (running: stop at chunk boundary; queued: leave queue). */
export function pauseTransfer(id: string): Promise<void> {
  return mockOr(
    () => call<void>('transfer_pause', { id }),
    () => undefined,
  );
}

/** Resume a paused transfer from its byte offset. */
export function resumeTransfer(id: string): Promise<void> {
  return mockOr(
    () => call<void>('transfer_resume', { id }),
    () => undefined,
  );
}

/** Cancel a transfer. */
export function cancelTransfer(id: string): Promise<void> {
  return mockOr(
    () => call<void>('transfer_cancel', { id }),
    () => undefined,
  );
}

/** Clear completed/failed/canceled records from the queue list. */
export function clearFinishedTransfers(): Promise<void> {
  return mockOr(
    () => call<void>('transfer_clear_finished'),
    () => undefined,
  );
}

/** Set the engine's bandwidth budget in bytes/sec; 0 = unlimited. */
export function setRateLimit(bytesPerSec: number): Promise<void> {
  return mockOr(
    () => call<void>('transfer_set_rate_limit', { bytesPerSec }),
    () => undefined,
  );
}

/**
 * Subscribe to live transfer updates from the engine. Returns an
 * unsubscribe function. Browser mode never fires (no native events).
 */
export function onTransferUpdate(
  handler: (record: Transfer) => void,
): Promise<() => void> {
  if (!IS_TAURI) return Promise.resolve(() => {});
  return import('@tauri-apps/api/event').then(({ listen }) =>
    listen<Transfer>('transfer:update', (event) => handler(event.payload)).then(
      (unlisten) => () => unlisten(),
    ),
  );
}

/** Native file picker: choose one or more files to upload. */
export async function pickFilesToUpload(): Promise<string[]> {
  if (!IS_TAURI) return [];
  const { open } = await import('@tauri-apps/plugin-dialog');
  const selection = await open({ multiple: true });
  if (selection === null) return [];
  return Array.isArray(selection) ? selection : [selection];
}

/** Native directory picker: choose where downloads land. */
export async function pickDownloadDirectory(): Promise<string | null> {
  if (!IS_TAURI) return null;
  const { open } = await import('@tauri-apps/plugin-dialog');
  const selection = await open({ directory: true });
  return selection ?? null;
}

// ─────────────────────────────────────────────────────────────────────────────
// Remote editing
// ─────────────────────────────────────────────────────────────────────────────

/** Download a remote file to a temp location and open it in the system
 * editor. Returns the temp path. Saves re-upload automatically. */
export function remoteEditOpen(connectionId: string, remotePath: string): Promise<string> {
  return mockOr(
    () => call<string>('remote_edit_open', { connectionId, remotePath }),
    () => '',
  );
}

/**
 * Subscribe to remote-edit outcomes (save synced / failed). Browser mode
 * never fires.
 */
export function onEditUpdate(
  handler: (event: { kind: 'synced' | 'failed'; fileName: string; message?: string }) => void,
): Promise<() => void> {
  if (!IS_TAURI) return Promise.resolve(() => {});
  return import('@tauri-apps/api/event').then(({ listen }) =>
    listen<{ kind: string; fileName: string; message?: string }>('edit:update', (event) =>
      handler({
        kind: event.payload.kind === 'synced' ? 'synced' : 'failed',
        fileName: event.payload.fileName,
        message: event.payload.message,
      }),
    ).then((unlisten) => () => unlisten()),
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Smart Sync
// ─────────────────────────────────────────────────────────────────────────────

/** The two trees differ here; see flow-sync for the semantics. */
export function syncPreview(params: {
  connectionId: string;
  localDir: string;
  remoteDir: string;
  direction: SyncDirection;
}): Promise<SyncDiff[]> {
  return mockOr(
    () => call<SyncDiff[]>('sync_preview', params),
    () =>
      mockTransfers.slice(0, 5).map((t) => ({
        path: `${t.remotePath}${t.fileName}`,
        direction: t.direction,
        size: t.size,
        reason: t.status === 'failed' ? ('conflict' as const) : ('newer' as const),
      })),
  );
}

/** Execute a reviewed plan (conflicts are skipped server-side). Returns the
 * number of transfers enqueued. */
export function syncExecute(params: {
  connectionId: string;
  localRoot: string;
  remoteRoot: string;
  diffs: SyncDiff[];
}): Promise<number> {
  return mockOr(
    () => call<number>('sync_execute', params),
    () => params.diffs.length,
  );
}
