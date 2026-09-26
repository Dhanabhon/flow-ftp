# FlowFTP

A dual-pane FTP/SFTP client for macOS, built with Tauri 2, SvelteKit, and Rust. Open source, MIT licensed, and designed to feel like Finder rather than FileZilla.

## Status

Under active development. The core is real and working: FTP, FTPS, and SFTP connections, a resumable transfer queue, folder sync with conflict resolution, remote editing, and saved connection profiles. What you see in the app is wired to the Rust backend, not a mockup.

Not done yet: code signing and notarization, auto-update, cloud storage backends, tabs, and a remote terminal. See ROADMAP.md for the full list.

## Features

- Dual-pane file browser for local and remote folders, with a draggable pane splitter, breadcrumbs, sorting, and hidden-file toggle
- Drag & drop between panes and straight from Finder, with overwrite confirmation before anything moves
- FTP, FTPS (explicit and implicit TLS), and SFTP adapters behind one shared interface, so the rest of the app never cares which protocol is active
- Deterministic navigation: the target folder is listed before the app commits to it, so a failed open leaves you where you were — and in-flight listings are latest-wins
- Loading indicators for every remote listing, plus a 30-second operation timeout with actionable errors instead of stuck spinners
- Transfer queue with pause, resume from byte offsets, retry with backoff, live speed and ETA, and a bandwidth cap you can change while transfers run
- Smart Sync that compares two folder trees, shows every proposed change before anything moves, and lets you resolve conflicts by picking a side
- Remote editing: double-click a remote file, it opens in your system editor, and saving re-uploads it
- Saved connection profiles in the app config dir, with passwords in the macOS keychain, never in plain files
- Themed quit confirmation when transfers or server connections are still live
- Command palette (Cmd+K) for navigation and actions
- Light and dark theme, with a follow-system option

## Requirements

- Node.js 20 or newer, pnpm 9 or newer
- Rust stable, via rustup
- macOS 11 or newer (Apple Silicon first). Tauri's build prerequisites apply.

## Getting started

```sh
pnpm install
pnpm dev          # frontend only, in a browser, with mock data
pnpm tauri dev    # the real native app
```

`pnpm dev` runs the UI in a plain browser with mock data, which is handy for layout work. `pnpm tauri dev` builds the Rust side and opens the native window; everything is live there.

To check types and run the test suites:

```sh
pnpm check            # svelte-check
cargo test --workspace
```

Protocol integration tests dial real servers and are skipped by default. Set `FLOW_FTP_*` or `FLOW_SFTP_*` environment variables and run them with `-- --ignored` if you have a server to test against. See `crates/protocols/tests/` for the variable names.

## Diagnostics

The app writes an FTP wire transcript to `~/Library/Logs/com.flowftp.app/ftp.log` (truncated on every launch). It records every command round trip — connect, login, `CWD`/`PWD`, listings and their outcome — and never contains passwords. If a server misbehaves, that file is the first place to look.

## Troubleshooting a stubborn server

- Every remote operation has a 30-second ceiling; a stalled server surfaces as "No response from the server within 30s" and the session is closed cleanly. Reconnect and retry.
- FTP commands run relative to a `CWD` into the target directory — the pattern CyberDuck and FileZilla use — because some shared-hosting servers return wrong results for absolute paths in `LIST`/`RETR`/`SIZE`.

## Keyboard shortcuts

| Action | Shortcut |
| --- | --- |
| Command palette | Cmd K |
| New connection | Cmd N |
| Go to browser | Cmd 1 |
| Go to transfers | Cmd 2 |
| Synchronize folders | Cmd Shift S |
| Toggle hidden files | Cmd Shift . |

## Project layout

```
apps/desktop/       Tauri app: SvelteKit frontend plus the Rust command bridge
crates/core/        domain types and the RemoteFs trait
crates/protocols/   FTP, FTPS, and SFTP adapters
crates/transfer/    queue, retry, resume, progress, rate limiting
crates/sync/        folder compare
crates/keychain/    macOS keychain storage for credentials
packages/ui/        shared UI components (empty for now)
```

The dependency rule is one-way: UI calls the application layer, the application layer calls the domain, and adapters implement domain traits. ARCHITECTURE.md explains the layering, DESIGN.md holds the design language, and AGENTS.md documents the conventions for AI coding agents.

## Contributing

PRs welcome. Conventional Commits, `pnpm check` and `cargo test` green before you push. CONTRIBUTING.md has the details.

## License

MIT. See LICENSE.
