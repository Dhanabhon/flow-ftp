# FlowFTP

> The most beautiful and enjoyable open-source FTP/SFTP client for macOS.

FlowFTP is a **Remote Workspace** designed for developers on macOS. It combines the calm of Finder, the speed of Raycast, the polish of Linear, and the power of VSCode into one native-feeling file transfer application.

Three principles guide every decision: **Calm · Fast · Predictable.**

---

## ✨ Features

- **Dual-pane file browser** — Finder-style local ↔ remote navigation with drag & drop, breadcrumbs, and sort.
- **Multi-protocol** — FTP, FTPS, and SFTP with first-class support.
- **Smart Sync** — Preview every upload, download, delete, and conflict *before* it happens.
- **Transfer queue** — Pause, resume, retry, and live speed/ETA. Resumable transfers out of the box.
- **Command Palette** (⌘K) — Spotlight-style search for every action.
- **Quick Look preview** — Inspect images, text, JSON, YAML without downloading.
- **macOS Keychain** — Credentials never touch disk; stored securely in the OS keychain.
- **Keyboard-first** — Every action reachable from the keyboard.
- **Native macOS feel** — Dark theme, traffic-light spacing, thin scrollbars, glass blur.

> **Status:** Early development. The UI is a working mock built on the design language in [`DESIGN.md`](./DESIGN.md).

---

## 🧱 Tech Stack

**Frontend**
- [Tauri 2](https://v2.tauri.app/) — native shell, Rust backend
- [SvelteKit 2](https://kit.svelte.dev/) + [Svelte 5](https://svelte.dev/) (runes)
- [Tailwind CSS v4](https://tailwindcss.com/) — CSS-first theming
- shadcn-style components (`tailwind-variants`, `tailwind-merge`, `bits-ui`)
- [TanStack Query](https://tanstack.com/query/latest) — async state
- [lucide-svelte](https://lucide.dev/) — icons

**Backend / Core (Rust)**
- [Tokio](https://tokio.rs/) async runtime
- [`suppaftp`](https://crates.io/crates/suppaftp) — FTP / FTPS
- [`russh`](https://crates.io/crates/russh) / `async-ssh2-tokio` — SFTP
- [`keyring-rs`](https://crates.io/crates/keyring) — macOS Keychain
- [`notify`](https://crates.io/crates/notify) — local folder watching
- [`walkdir`](https://crates.io/crates/walkdir) — file tree scanning
- `serde` + `toml` / `json` — configuration

**macOS Integration**
- Keychain credential storage · Native menu bar · Finder-style drag & drop · Quick Look preview · Spotlight-style command palette · Notarization + DMG packaging

---

## 📁 Project Structure

This is a **pnpm + Cargo monorepo**. The frontend and Rust core live side by side so the Tauri app can call directly into the protocol crates.

```
flow-ftp/
├── apps/
│   └── desktop/          # Tauri 2 app (SvelteKit frontend)
├── crates/
│   ├── core/             # domain logic
│   ├── protocols/        # ftp, sftp, ftps
│   ├── transfer/         # queue, retry, resume
│   ├── sync/             # folder compare / sync
│   └── keychain/         # secure credential storage
├── packages/
│   └── ui/               # shared UI components
├── DESIGN.md             # design language & UX principles
├── pnpm-workspace.yaml
└── package.json
```

---

## 🚀 Getting Started

### Prerequisites

- **Node.js** ≥ 20 and **pnpm** ≥ 9
- **Rust** (stable) via [rustup](https://rustup.rs/)
- **Tauri 2 prerequisites** for macOS — see the [Tauri setup guide](https://v2.tauri.app/start/prerequisites/)

### Install & run

```bash
# from the repo root — installs the whole workspace
pnpm install

# start the frontend dev server (Vite on :1420)
pnpm dev
```

Then open <http://localhost:1420> to view the UI mock.

### Build for production

```bash
pnpm build           # builds the frontend
```

### Type-check

```bash
pnpm check           # svelte-check across the workspace
```

### Run as a native app (Tauri)

> Requires the Rust crates and `src-tauri/` config to be scaffolded first.

```bash
pnpm tauri dev       # native window with hot reload
pnpm tauri build     # notarized .app / .dmg
```

---

## ⌨️ Keyboard Shortcuts

| Action | Shortcut |
| --- | --- |
| Command Palette | `⌘ K` |
| Quick Connect | `⌘ K` → "Quick Connect" |
| Synchronize Folder | `⌘ ⇧ S` |
| Upload files | `⌘ ⇧ U` |
| Download selected | `⌘ ⇧ D` |
| Toggle hidden files | `⌘ ⇧ .` |
| Switch view (Connections / Transfers / Sync / History) | `⌘ 1` – `⌘ 4` |
| Settings | `⌘ ,` |
| Reconnect | `⌘ R` |

---

## 🎨 Design Language

FlowFTP follows a deliberate design philosophy — *Calm, Fast, Predictable* — built on the Apple Human Interface Guidelines with influences from Linear and Raycast.

- **Content first** — the files are the product; UI never competes with them.
- **Progressive disclosure** — beginners see Connect · Browse · Upload · Download; power users discover Sync, Compare, Terminal, and the Command Palette.
- **Zero fear** — dangerous actions always show *Preview → Confirm → Execute*.
- **Never intrusive** — small toasts, no unnecessary modals, no raw protocol errors.

The full specification — colors, typography, spacing scale, window layout, motion timing, and UX anti-patterns — lives in [`DESIGN.md`](./DESIGN.md).

---

## 📦 Workspace Scripts

| Command | Description |
| --- | --- |
| `pnpm dev` | Start the desktop frontend dev server |
| `pnpm build` | Production build |
| `pnpm preview` | Preview the production build |
| `pnpm check` | Run `svelte-check` type checking |
| `pnpm tauri` | Pass-through to the Tauri CLI |

Individual workspace packages can be targeted with `pnpm --filter @flow-ftp/desktop <script>`.

---

## 🗺 Roadmap

- Remote Terminal & SSH sessions
- Folder Compare & Diff Viewer
- Git status overlay
- Cloud storage backends (S3, R2, Backblaze, Dropbox, Google Drive)
- Multiple tabs & workspace profiles
- Deploy pipelines & remote code editor
- Plugin marketplace

---

## 📄 License

See [`LICENSE`](./LICENSE).
