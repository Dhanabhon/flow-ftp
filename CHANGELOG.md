# Changelog

All notable changes to FlowFTP are documented here.

## [1.1.0] - 2026-10-17

### Added

- Loading indicators for remote folder navigation: rows dim with a spinner while a listing travels, and every remote operation has a 30-second timeout with actionable errors instead of stuck spinners.
- Finder drops onto a remote folder row now upload into that folder, matching in-app drag behavior.
- FTP wire transcript log at `~/Library/Logs/com.flowftp.app/ftp.log` for diagnosing misbehaving servers (never contains passwords).

### Fixed

- Folder navigation on shared-hosting FTP servers: commands now run relative to a CWD into the target directory, the pattern CyberDuck and FileZilla use.
- Remote listings can no longer desync from the breadcrumb when navigating quickly after connect (latest-wins guard).
- Timed-out sessions are evicted and the connection is marked lost, so follow-up commands never read desynced responses.
- Tooltips no longer clip at the window's side edges; compact confirmation dialogs sit centered.

## [1.0.0] - 2026-09-26

### Added

- Dual-pane local and remote file browsing with drag and drop.
- FTP, FTPS, and SFTP connections with saved profiles and macOS Keychain credentials.
- Transfer queue with pause, resume, retry, progress, and bandwidth limiting.
- Smart Sync previews with conflict resolution, plus remote file editing.
- FlowFTP v5 Tidal Flow app icon and in-app brand mark.
