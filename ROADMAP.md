# FlowFTP — Roadmap

This roadmap tracks the journey from UI mock to the best open-source FTP/SFTP client for developers. It is directional, not a contract — priorities shift with user feedback.

Status legend: ✅ Done · 🚧 In Progress · 📋 Planned · 🔬 Exploring

---

## Performance Goals

Every release must protect these targets. A regression here is a release blocker.

| Metric | Target | Status |
| --- | --- | --- |
| Application startup | < 500 ms | 🚧 |
| Connection | < 2 seconds | 📋 |
| Scrolling (10k+ files) | 60 FPS | 🚧 |
| Searching | Instant (< 50 ms) | 📋 |
| Memory at idle | < 150 MB | 📋 |

Large folders (10,000+ files) must remain responsive at all times.

---

## Accessibility

Accessibility is a **first-class feature**, tracked as a release gate, not a nice-to-have.

- [ ] **Keyboard navigation** — every action reachable without a mouse
- [ ] **VoiceOver** — all interactive elements labeled and announced
- [ ] **Reduced motion** — honor `prefers-reduced-motion` on every animation
- [ ] **High contrast** — theme tested at macOS high-contrast setting
- [ ] **Large text** — layout reflows cleanly at largest system text size

---

## Phased Delivery

### Phase 0 — Foundation ✅
- [x] Monorepo structure (pnpm + Cargo workspace)
- [x] SvelteKit 2 + Tauri 2 native shell
- [x] Dark premium theme tokens, macOS-native feel
- [x] Drag region + overlay titlebar working
- [x] Project documentation (`AGENTS.md`, `DESIGN.md`, `PRODUCT.md`, `ARCHITECTURE.md`)

### Phase 1 — UI Shell 🚧
- [x] Header with command search, quick connect, theme toggle
- [x] Sidebar: Connections / Transfers / Sync / History / Settings, Favorites, Recent
- [x] Dual-pane file browser (local + remote)
- [x] Transfer queue with live progress + status
- [x] Preview panel (Quick Look style)
- [x] Command Palette (⌘K)
- [x] Quick Connect modal
- [x] Sync Preview modal
- [x] Dark / Light theme toggle with system preference
- [ ] Wire UI to live mock data refresh (TanStack Query plumbing)

### Phase 2 — Real Protocols 📋
- [ ] `crates/core` domain models + `RemoteFs` trait
- [ ] `crates/protocols` FTP adapter (`suppaftp`)
- [ ] `crates/protocols` FTPS adapter (TLS)
- [ ] `crates/protocols` SFTP adapter (`russh`)
- [ ] `crates/keychain` credential storage (`keyring-rs`)
- [ ] Tauri command bridge: connect / list / download / upload / delete / rename
- [ ] Real connection list + favorites persistence

### Phase 3 — Transfer Engine 📋
- [ ] `crates/transfer` queue with priority
- [ ] Pause / resume / retry with backoff
- [ ] Resumable transfers (FTP `REST`, SFTP offset)
- [ ] Live progress events (speed, ETA)
- [ ] Bandwidth limiting
- [ ] Post-transfer actions (notifications, sounds)

### Phase 4 — Smart Sync 📋
- [ ] `crates/sync` folder compare (size, mtime, checksum)
- [ ] Diff preview (upload / download / delete / conflict)
- [ ] Direction modes: bidirectional, one-way
- [ ] Dry-run mode
- [ ] Saved sync profiles

### Phase 5 — Power Features 🔬
- [ ] Remote editing (edit remote → auto-upload on save)
- [ ] Folder Compare & Diff Viewer
- [ ] Remote Terminal & SSH sessions
- [ ] Git status overlay
- [ ] Spotlight-like global search across connections
- [ ] Command Palette file navigation

### Phase 6 — Ecosystem 🔬
- [ ] Cloud storage backends (S3, R2, Backblaze, Dropbox, Google Drive)
- [ ] Multiple tabs & workspace profiles
- [ ] Deploy pipelines & remote code editor integration
- [ ] Plugin marketplace & scripting API
- [ ] Cross-platform build (Windows, Linux) — **after** macOS UX is excellent

### Phase 7 — Polish & Distribution 📋
- [ ] Notarization + DMG packaging
- [ ] Auto-update (Tauri updater)
- [ ] Sparkle-style update UX
- [ ] Accessibility audit pass
- [ ] Performance benchmark suite (startup, scroll, search)
- [ ] Localization (English first, i18n-ready)

---

## Non-Goals (for now)

To stay focused, these are explicitly **out of scope** until macOS UX is excellent:

- Windows / Linux builds
- Mobile apps
- Built-in text/code editor (defer to the user's local editor via remote editing)
- Tor / proxy tunneling UI
- Legacy protocol support (FXP, WebDAV via FTP shim)

---

## Release Cadence

- **0.x** — foundation, breaking changes expected, no stability guarantee
- **1.0** — first stable release: real protocols + transfer engine + sync, macOS polished
- **1.x+** — power features, ecosystem, cross-platform

---

## How to Influence This Roadmap

Open a [GitHub Discussion](https://github.com/Dhanabhon/flow-ftp/discussions) for feature ideas, or an issue for bugs. Well-scoped PRs that match a roadmap item merge fastest. See [`CONTRIBUTING.md`](./CONTRIBUTING.md).
