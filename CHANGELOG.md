# Changelog

All notable changes to FlowFTP are documented here.

## [1.2.2] - 2026-10-25

### Fixed

- Passwords are now remembered as expected. Quick Connect no longer silently skips the Keychain: the Save-to-Keychain toggle (on by default) is available in both modes, every successful connect persists the connection record so its Keychain entry stays reusable, and reconnecting to a remembered server no longer asks for the password again.
- A connection that reconnected through the Keychain now reports `keychain: true` to the UI, so the saved-password path survives app restarts.

## [1.2.1] - 2026-10-17

### Added

- In-app Software Update: check for new versions from this repository's GitHub Releases, download with live progress, install, and relaunch — from the FlowFTP menu, the command palette, or the automatic launch check.

### Notes

- Downloads still trigger Gatekeeper's "unverified developer" dialog on first open until Developer ID signing and notarization land (ROADMAP Phase 7). Use **Open Anyway** in Privacy & Security in the meantime.

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
