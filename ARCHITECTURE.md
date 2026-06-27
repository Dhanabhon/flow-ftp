# FlowFTP — System Architecture

FlowFTP uses **Clean Architecture** with strict layer separation. Business logic never depends on UI. Protocols are independent modules, interchangeable behind a common trait.

---

## Layer Separation

```
┌─────────────────────────────────────────────────────┐
│  UI Layer          SvelteKit + Tauri webview        │
│  (apps/desktop)    components, stores, commands     │
├─────────────────────────────────────────────────────┤
│  Application       Use cases, orchestration,        │
│  Layer             Tauri commands (IPC bridge)      │
├─────────────────────────────────────────────────────┤
│  Domain Layer      Pure business logic.             │
│  (crates/core)     No I/O, no framework deps.       │
├─────────────────────────────────────────────────────┤
│  Infrastructure    Protocols, storage, OS services. │
│  Layer             crates/protocols, keychain, …    │
└─────────────────────────────────────────────────────┘
```

### Dependency rule
Dependencies point **inward** only.

- **UI** → Application → Domain
- **Infrastructure** → Domain (implements domain traits)
- **Domain** depends on nothing external.

Business logic must never depend on UI.

---

## Workspace Layout

```
flow-ftp/
├── apps/
│   └── desktop/                # Tauri 2 app
│       ├── src/                # UI Layer (SvelteKit frontend)
│       │   ├── lib/components/  # Svelte components
│       │   ├── lib/stores/      # Reactive state (runes)
│       │   ├── lib/types.ts     # Domain mirrors (TS)
│       │   └── routes/          # SvelteKit routes
│       └── src-tauri/          # Application + bootstrap
│           ├── src/             # Tauri commands, IPC bridge
│           └── Cargo.toml
├── crates/
│   ├── core/                   # Domain Layer — pure logic
│   ├── protocols/              # Infrastructure — ftp, sftp, ftps
│   ├── transfer/               # Infrastructure — queue, retry, resume
│   ├── sync/                   # Infrastructure — folder compare/sync
│   └── keychain/               # Infrastructure — credential storage
└── packages/
    └── ui/                     # Shared UI components
```

### Module responsibilities

| Crate / Package | Layer | Responsibility |
| --- | --- | --- |
| `apps/desktop` (frontend) | UI | Components, interaction, layout |
| `apps/desktop/src-tauri` | Application | Tauri commands, IPC, event bridge |
| `crates/core` | Domain | Connection, File, Transfer, Sync domain models + traits |
| `crates/protocols` | Infrastructure | FTP/FTPS/SFTP adapters implementing core traits |
| `crates/transfer` | Infrastructure | Queue, retry policy, resumable transfers |
| `crates/sync` | Infrastructure | Folder compare, diff, reconciliation |
| `crates/keychain` | Infrastructure | macOS Keychain credential storage |
| `packages/ui` | UI | Cross-app shared Svelte components |

---

## Protocols as Independent Modules

FTP and SFTP implementations are **interchangeable** behind a single trait defined in `crates/core`.

```rust
// crates/core/src/remote.rs (illustrative)
#[async_trait]
pub trait RemoteFs: Send + Sync {
    async fn connect(&mut self, creds: &Credentials) -> Result<()>;
    async fn list(&self, path: &Path) -> Result<Vec<RemoteFile>>;
    async fn download(&self, remote: &Path, local: &Path) -> Result<TransferHandle>;
    async fn upload(&self, local: &Path, remote: &Path) -> Result<TransferHandle>;
    async fn delete(&self, path: &Path) -> Result<()>;
    async fn rename(&self, from: &Path, to: &Path) -> Result<()>;
    async fn disconnect(&mut self) -> Result<()>;
}
```

`crates/protocols` provides three implementations:
- `FtpFs` (via `suppaftp`)
- `FtpsFs` (TLS via `suppaftp`)
- `SftpFs` (via `russh` / `async-ssh2-tokio`)

The Application Layer selects an adapter by `Protocol` enum; the rest of the system never knows which protocol is active.

---

## Async-First Architecture

All I/O is async, built on **Tokio**.

- Protocol calls → `async fn`, cancellation-safe where possible.
- Transfers stream progress via Tauri **events** (`transfer:progress`, `transfer:done`, …).
- The UI subscribes to events through TanStack Query + a thin Tauri event bridge.
- Blocking work (large directory scans via `walkdir`) runs on `tokio::task::spawn_blocking`.

The webview never blocks on a synchronous IPC call for long-running work. Commands return quickly; progress flows back as events.

---

## State & Data Flow

```
User action (click / shortcut)
        │
        ▼
Svelte store  ──invoke()──►  Tauri command (Application)
                                   │
                                   ▼
                          Domain service (crates/core)
                                   │
                                   ▼
                          Infrastructure adapter
                          (protocols / keychain / …)
                                   │
                                   ▼
                          Events emitted back to UI
                          (transfer:progress, …)
                                   │
                                   ▼
                          TanStack Query cache updates
```

- **UI state** lives in Svelte runes stores (`app.svelte.ts`, `theme.svelte.ts`).
- **Domain state** (connections, transfer queue) lives in Application services on the Rust side.
- **Persistence** (credentials) goes through `crates/keychain` only — never plain files.

---

## Error Handling

Errors flow as strongly-typed `Result`s. The boundary between layers converts infrastructure errors into domain errors, and the Application Layer converts domain errors into human-readable messages for the UI.

Every user-facing error explains:
1. **What happened**
2. **Why**
3. **How to fix it**

Raw protocol errors are never exposed to the user.

---

## Testing Strategy

- **Domain crate** (`crates/core`): pure unit tests, no I/O.
- **Protocols**: integration tests against a local FTP/SFTP test server.
- **Sync / Transfer**: table-driven tests on diff/retry logic.
- **UI**: `svelte-check` for type safety; component tests near the code they cover.

Every major module should have unit tests. See [`CONTRIBUTING.md`](./CONTRIBUTING.md) for test conventions.

---

## Decisions Record

| Decision | Choice | Rationale |
| --- | --- | --- |
| Frontend framework | SvelteKit 2 + Svelte 5 runes | Compile-step reactivity, small bundle, native-feeling perf |
| Native shell | Tauri 2 | Small footprint, Rust backend, macOS-native APIs |
| FTP client | `suppaftp` | Active maintenance, async-friendly, FTP+FTPS |
| SFTP client | `russh` | Pure-Rust SSH, async, no libssh2 C dep |
| Credential store | `keyring-rs` | Native macOS Keychain, no plaintext |
| Async runtime | Tokio | De facto Rust async standard |
| Frontend state | Svelte runes + TanStack Query | Local UI state + async server cache |
