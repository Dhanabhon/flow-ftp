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

### Phase 1 — UI Shell ✅
- [x] Header with command search, quick connect, theme toggle
- [x] Sidebar: Connections / Transfers / Sync / History / Settings, Favorites, Recent
- [x] Dual-pane file browser (local + remote) with draggable pane splitter
- [x] Transfer queue with live progress + status
- [x] Preview panel (Quick Look style): local images and text, remote placeholder
- [x] Command Palette (⌘K)
- [x] Quick Connect modal (Quick vs New connection semantics, password reveal)
- [x] Sync Preview modal
- [x] Dark / Light theme toggle with system preference
- [x] Drag & drop: between panes and from Finder, with overwrite confirmation
- [x] Themed quit confirmation guarding live transfers and connections

### Phase 2 — Real Protocols ✅
- [x] `crates/core` domain models + `RemoteFs` trait
- [x] `crates/protocols` FTP adapter (`suppaftp`)
- [x] `crates/protocols` FTPS adapter (explicit + implicit TLS)
- [x] `crates/protocols` SFTP adapter (`russh` + `russh-sftp`)
- [x] `crates/keychain` credential storage (`keyring-rs`)
- [x] Tauri command bridge: connect / list / download / upload / delete / rename
- [x] Real connection list + favorites persistence (profiles in the app config dir)
- [x] CWD-relative command layer for shared-hosting FTP compatibility, MLSD→LIST fallback
- [x] Deterministic navigation (list-first, commit-after) with loading indicator and 30s op timeout

### Phase 3 — Transfer Engine ✅
- [x] `crates/transfer` queue with priority
- [x] Pause / resume / retry with backoff (deterministic errors skip retry)
- [x] Resumable transfers (FTP `REST`/`APPE`, SFTP offset)
- [x] Live progress events (speed, ETA) via `transfer:update`
- [x] Bandwidth limiting, changeable while transfers run
- [ ] Post-transfer actions (notifications, sounds)

### Phase 4 — Smart Sync ✅
- [x] `crates/sync` folder compare (size + mtime with tolerance)
- [x] Diff preview (upload / download / delete / conflict)
- [x] Direction modes: bidirectional, one-way
- [x] Preview → confirm → execute (the preview is the dry run)
- [ ] Checksum-based comparison
- [ ] Saved sync profiles

### Phase 5 — Power Features 🚧
- [x] Remote editing (edit remote → auto-upload on save)
- [x] Command Palette actions and navigation
- [ ] Folder Compare & Diff Viewer
- [ ] Remote Terminal & SSH sessions
- [ ] Git status overlay
- [ ] Spotlight-like global search across connections
- [ ] SFTP host-key pinning (currently accept-any, dev-only)


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
