import type { Connection, RemoteFile, Transfer } from './types';

const now = Date.now();
const min = 60_000;
const hr = 3_600_000;
const day = 86_400_000;

export const mockConnections: Connection[] = [
  {
    id: 'c1',
    name: 'Production Server',
    protocol: 'sftp',
    host: 'prod.example.com',
    port: 22,
    username: 'tom',
    status: 'connected',
    keychain: true,
    favorite: true,
    lastConnected: now - 4 * min
  },
  {
    id: 'c2',
    name: 'Staging Box',
    protocol: 'sftp',
    host: 'staging.internal',
    port: 22,
    username: 'deploy',
    status: 'disconnected',
    keychain: true,
    favorite: true,
    lastConnected: now - 2 * day
  },
  {
    id: 'c3',
    name: 'Legacy FTP',
    protocol: 'ftp',
    host: 'legacy-archive.net',
    port: 21,
    username: 'anonymous',
    status: 'disconnected',
    keychain: false,
    favorite: false,
    lastConnected: now - 9 * day
  },
  {
    id: 'c4',
    name: 'CDN Upload',
    protocol: 'ftps',
    host: 'upload.cdn.io',
    port: 990,
    username: 'flowftp',
    status: 'error',
    keychain: true,
    favorite: false,
    lastConnected: now - 6 * hr
  }
];

export const mockLocalFiles: RemoteFile[] = [
  { name: '..', kind: 'directory', size: 0, modified: 0 },
  { name: 'Projects', kind: 'directory', size: 0, modified: now - 2 * day, permissions: 'rwxr-xr-x' },
  { name: 'Screenshots', kind: 'directory', size: 0, modified: now - 5 * hr, permissions: 'rwxr-xr-x' },
  { name: 'Documents', kind: 'directory', size: 0, modified: now - 1 * day, permissions: 'rwxr-xr-x' },
  { name: 'design-spec.fig', kind: 'file', size: 8_400_000, modified: now - 30 * min, permissions: 'rw-r--r--' },
  { name: 'budget-2026.xlsx', kind: 'file', size: 124_000, modified: now - 3 * day, permissions: 'rw-r--r--' },
  { name: 'meeting-notes.md', kind: 'file', size: 4_200, modified: now - 8 * hr, permissions: 'rw-r--r--' },
  { name: 'logo-final.png', kind: 'file', size: 540_000, modified: now - 1 * day, permissions: 'rw-r--r--' },
  { name: 'archive.zip', kind: 'file', size: 240_000_000, modified: now - 12 * day, permissions: 'rw-r--r--' },
  { name: '.DS_Store', kind: 'file', size: 6_148, modified: now - 1 * hr, permissions: 'rw-r--r--' }
];

export const mockRemoteFiles: RemoteFile[] = [
  { name: '..', kind: 'directory', size: 0, modified: 0 },
  { name: 'public_html', kind: 'directory', size: 0, modified: now - 4 * hr, permissions: 'rwxr-xr-x', owner: 'tom', group: 'www-data' },
  { name: 'logs', kind: 'directory', size: 0, modified: now - 25 * min, permissions: 'rwxr-xr-x', owner: 'root', group: 'syslog' },
  { name: 'backups', kind: 'directory', size: 0, modified: now - 1 * day, permissions: 'rwxr-x---', owner: 'tom', group: 'tom' },
  { name: 'config', kind: 'directory', size: 0, modified: now - 6 * day, permissions: 'rwxr-xr-x', owner: 'tom', group: 'tom' },
  { name: '.bashrc', kind: 'file', size: 3_800, modified: now - 40 * day, permissions: 'rw-r--r--', owner: 'tom', group: 'tom' },
  { name: 'deploy.sh', kind: 'file', size: 12_400, modified: now - 2 * hr, permissions: 'rwxr-xr-x', owner: 'tom', group: 'tom' },
  { name: 'nginx.conf', kind: 'file', size: 8_200, modified: now - 5 * hr, permissions: 'rw-r--r--', owner: 'root', group: 'root' },
  { name: 'access.log', kind: 'file', size: 84_000_000, modified: now - 3 * min, permissions: 'rw-r-----', owner: 'root', group: 'syslog' },
  { name: 'error.log', kind: 'file', size: 2_100_000, modified: now - 3 * min, permissions: 'rw-r-----', owner: 'root', group: 'syslog' },
  { name: 'db-dump.sql.gz', kind: 'file', size: 1_200_000_000, modified: now - 1 * day, permissions: 'rw-r--r--', owner: 'tom', group: 'tom' },
  { name: 'release-v2.4.tar.gz', kind: 'file', size: 48_000_000, modified: now - 8 * hr, permissions: 'rw-r--r--', owner: 'tom', group: 'tom' }
];

export const mockTransfers: Transfer[] = [
  {
    id: 't1',
    fileName: 'release-v2.4.tar.gz',
    direction: 'upload',
    size: 48_000_000,
    transferred: 31_200_000,
    speed: 12_400_000,
    status: 'active',
    connectionId: 'c1',
    connectionName: 'Production Server',
    remotePath: '/var/www/releases/',
    localPath: '~/Downloads/',
    startedAt: now - 12,
    resumable: true
  },
  {
    id: 't2',
    fileName: 'db-dump.sql.gz',
    direction: 'download',
    size: 1_200_000_000,
    transferred: 1_200_000_000,
    speed: 0,
    status: 'completed',
    connectionId: 'c1',
    connectionName: 'Production Server',
    remotePath: '/backups/',
    localPath: '~/Downloads/',
    startedAt: now - 90 * 1000,
    finishedAt: now - 2 * 1000,
    resumable: true
  },
  {
    id: 't3',
    fileName: 'design-spec.fig',
    direction: 'upload',
    size: 8_400_000,
    transferred: 2_100_000,
    speed: 0,
    status: 'paused',
    connectionId: 'c2',
    connectionName: 'Staging Box',
    remotePath: '/uploads/',
    localPath: '~/Documents/',
    startedAt: now - 5 * min,
    resumable: true
  },
  {
    id: 't5',
    fileName: 'config.yaml',
    direction: 'upload',
    size: 2_400,
    transferred: 0,
    speed: 0,
    status: 'queued',
    connectionId: 'c1',
    connectionName: 'Production Server',
    remotePath: '/config/',
    localPath: '~/Projects/flow-ftp/'
  },
  {
    id: 't4',
    fileName: 'access.log',
    direction: 'download',
    size: 84_000_000,
    transferred: 0,
    speed: 0,
    status: 'failed',
    connectionId: 'c4',
    connectionName: 'CDN Upload',
    remotePath: '/logs/',
    localPath: '~/Downloads/',
    error: 'Connection reset by peer (ECONNRESET)',
    resumable: false
  }
];

/** Spotlight-style command palette commands. */
export interface Command {
  id: string;
  label: string;
  hint?: string;
  group: 'Actions' | 'Navigate' | 'Connections' | 'View';
  shortcut?: string[];
  icon?: string;
}

// Every command here is wired in CommandPalette.runCommand — no dead entries.
export const mockCommands: Command[] = [
  { id: 'quick-connect', label: 'Quick Connect…', group: 'Actions', shortcut: ['⌘', 'K'], icon: 'zap' },
  { id: 'new-conn', label: 'New Connection…', group: 'Actions', shortcut: ['⌘', 'N'], icon: 'plus' },
  { id: 'sync', label: 'Synchronize Folder…', group: 'Actions', shortcut: ['⌘', '⇧', 'S'], icon: 'refresh-cw' },
  { id: 'go-browser', label: 'Go to Browser', group: 'Navigate', shortcut: ['⌘', '1'], icon: 'plug' },
  { id: 'go-transfers', label: 'Go to Transfers', group: 'Navigate', shortcut: ['⌘', '2'], icon: 'arrow-up-down' },
  { id: 'toggle-hidden', label: 'Toggle Hidden Files', group: 'View', shortcut: ['⌘', '⇧', '.'], icon: 'eye' },
  { id: 'toggle-theme', label: 'Toggle Theme', group: 'View', icon: 'eye' }
];

/** Synthetic preview payload for the right-hand Preview panel. */
export interface PreviewData {
  name: string;
  kind: 'image' | 'text' | 'archive' | 'generic';
  size: number;
  modified: number;
  meta: Record<string, string>;
}

export const mockPreview: PreviewData = {
  name: 'design-spec.fig',
  kind: 'image',
  size: 8_400_000,
  modified: now - 30 * min,
  meta: {
    Format: 'Figma Design',
    Dimensions: '1920 × 1080',
    Pages: '4',
    'Last edited': 'by Tom, 30m ago',
    Permission: 'rw-r--r--'
  }
};
