/** Domain types shared across the UI. These mirror the Rust core crate. */

export type Protocol = 'ftp' | 'ftps' | 'sftp';

export type ConnectionStatus =
  | 'connected'
  | 'connecting'
  | 'disconnected'
  | 'error';

export interface Connection {
  id: string;
  name: string;
  protocol: Protocol;
  host: string;
  port: number;
  username: string;
  status: ConnectionStatus;
  /** Whether credentials are stored in OS keychain. */
  keychain?: boolean;
  favorite?: boolean;
  lastConnected?: number; // epoch ms
}

export type FileKind = 'file' | 'directory' | 'symlink';

export interface RemoteFile {
  name: string;
  kind: FileKind;
  size: number; // bytes (0 for directories)
  modified: number; // epoch ms
  permissions?: string; // e.g. 'rwxr-xr-x'
  owner?: string;
  group?: string;
}

export type TransferDirection = 'upload' | 'download';
export type TransferStatus =
  | 'queued'
  | 'active'
  | 'paused'
  | 'completed'
  | 'failed'
  | 'canceled';

export interface Transfer {
  id: string;
  fileName: string;
  direction: TransferDirection;
  size: number;
  transferred: number;
  speed: number; // bytes/sec (smoothed)
  status: TransferStatus;
  connectionId: string;
  connectionName: string;
  remotePath: string;
  localPath: string;
  startedAt?: number;
  finishedAt?: number;
  error?: string;
  /** Supports resume (REST command for FTP). */
  resumable?: boolean;
}

export type SyncDirection = 'both' | 'local-to-remote' | 'remote-to-local';

export interface SyncDiff {
  path: string;
  direction: TransferDirection;
  size: number;
  reason: 'newer' | 'missing' | 'larger' | 'conflict';
}

export type View = 'connections' | 'transfers' | 'sync' | 'history' | 'settings';
