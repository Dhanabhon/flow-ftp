# FlowFTP — Product Vision

> FlowFTP is a remote workspace, not just an FTP client.

The application should make remote file management feel effortless. Users should spend time managing files — not fighting the software. The experience should feel calm, modern, lightweight, and intuitive.

---

## Primary Target Platform

- **macOS (Apple Silicon first)** — native desktop experience
- Cross-platform architecture is welcome, but **macOS UX always has priority**

## Target Users

| Audience | Why they care |
| --- | --- |
| Developers | Remote editing, SSH, keyboard-first workflows |
| DevOps Engineers | Bulk sync, deployment, log triage |
| System Administrators | Reliable transfers, permission management |
| Power Users | Command palette, automation, scripting hooks |
| Web Developers | Fast upload/download, folder compare |
| Open Source Contributors | Understandable, hackable codebase |

## Core Values

### Native
Every interaction should feel like a modern macOS application. Avoid Windows-style UI patterns. Follow Apple's Human Interface Guidelines whenever possible.

### Fast
Performance is a feature. Large folders (10,000+ files) should remain responsive. The application should launch quickly and avoid unnecessary background work.

### Beautiful
Whitespace matters. Typography matters. Animations should be subtle. Every screen should look intentional.

### Simple
Never expose complexity unless necessary. Progressive disclosure should always be preferred. Beginner users should never feel overwhelmed.

### Developer Friendly
Keyboard shortcuts everywhere. Command Palette. SSH support. Quick Sync. Remote editing. Powerful search.

---

## Design Language

**Inspired by**
- Finder
- Raycast
- Linear
- Arc Browser
- Transmit
- Notion

**Not inspired by**
- FileZilla
- WinSCP
- Legacy enterprise software

FlowFTP should feel like a combination of **Finder, Raycast, Linear, and Transmit** — not a clone of FileZilla or other legacy FTP applications.

---

## UX Principles

1. **Users should never lose work.** State is durable; nothing disappears on disconnect.
2. **Dangerous operations require preview.** Delete, overwrite, and bulk sync always show *Preview → Confirm → Execute*.
3. **Long-running tasks always show progress.** Speed, ETA, and pause/resume are non-negotiable.
4. **Every error explains: what happened, why, and how to fix it.** Never expose raw protocol errors directly.

## UI Principles

1. **Never create clutter.** Density is intentional, never accidental.
2. **Avoid icon overload.** Text labels and whitespace carry meaning.
3. **Avoid nested modal dialogs.** One modal at a time.
4. **Prefer inline editing.** Rename, edit paths, edit credentials in place.
5. **Use side panels instead of popup windows.**
6. **Command Palette replaces deep menus whenever possible.**

---

## Performance Goals

| Metric | Target |
| --- | --- |
| Application startup | < 500 ms |
| Connection | < 2 seconds |
| Scrolling | 60 FPS |
| Searching | Instant |

## Accessibility

Accessibility is a **first-class feature**, not an afterthought. Support:
- Keyboard navigation
- VoiceOver
- Reduced motion (`prefers-reduced-motion`)
- High contrast
- Large text

---

## Feature Specifications

### Connection management
- FTP, FTPS, SFTP first-class support
- Credentials stored in macOS Keychain (never on disk)
- Quick Connect (⌘K → host/user/pass) for ad-hoc sessions
- Favorites and recents

### Dual-pane file browser
- Finder-style local ↔ remote navigation
- Drag & drop between panes
- Breadcrumbs, sort, filter, hidden files
- Inline rename

### Transfer queue
- Pause / resume / retry / cancel
- Live speed + ETA
- Resumable transfers (REST for FTP, offset for SFTP)
- Bandwidth limiting

### Smart Sync
- Folder compare before any change
- Preview every upload, download, delete, conflict
- Direction modes: bidirectional, local→remote, remote→local
- Dry-run mode

### Command Palette (⌘K)
- Spotlight-style search for every action
- Fuzzy match across commands, connections, files
- Keyboard-navigable

### Quick Look preview
- Images, text, JSON, YAML without downloading
- Spacebar to preview (Finder parity)

### Remote editing
- Edit remote files in local editor; auto-upload on save
- Conflict detection on remote change

---

## Open Source Philosophy

- Code should be understandable by contributors.
- Document public APIs.
- Keep modules independent.
- Prefer explicit code over magic.
- Every feature should have documentation.
- Every major module should have unit tests.

See [`CONTRIBUTING.md`](./CONTRIBUTING.md) for contribution workflow, and [`ARCHITECTURE.md`](./ARCHITECTURE.md) for system structure.
