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
pub trait RemoteFs: Send {
    async fn connect(&mut self, creds: &Credentials) -> CoreResult<()>;
    async fn disconnect(&mut self) -> CoreResult<()>;
    async fn list(&mut self, path: &FilePath) -> CoreResult<Vec<RemoteFile>>;
    async fn stat(&mut self, path: &FilePath) -> CoreResult<RemoteFile>;
    async fn mkdir(&mut self, path: &FilePath) -> CoreResult<()>;
    async fn rename(&mut self, from: &FilePath, to: &FilePath) -> CoreResult<()>;
    async fn delete(&mut self, path: &FilePath) -> CoreResult<()>;
    // Transfers stream: open_read/open_write hand the engine a raw
    // AsyncRead/AsyncWrite at a byte offset, so resume and progress live
    // in crates/transfer, protocol-agnostic.
    async fn open_read(&mut self, remote: &FilePath, offset: u64)
        -> CoreResult<Box<dyn AsyncRead + Send + Unpin>>;
    async fn open_write(&mut self, remote: &FilePath, offset: u64)
        -> CoreResult<Box<dyn AsyncWrite + Send + Unpin>>;
}
```

`crates/protocols` provides three implementations:
- `FtpFs` (via `suppaftp`) — MLSD with LIST fallback; every command runs relative to a `CWD` into the target/parent directory, because some shared-hosting servers mishandle absolute paths
- `FtpsFs` (TLS via `suppaftp` + `rustls`, explicit and implicit)
- `SftpFs` (via `russh` + `russh-sftp`)

The Application Layer selects an adapter by `Protocol` enum; the rest of the system never knows which protocol is active.

---

## Application Layer Services

`apps/desktop/src-tauri` owns the live sessions and orchestration:

- **`ConnectionRegistry`** — one adapter per connection id, behind a tokio mutex. Protocol sessions handle one command at a time, so per-connection serialization is by design.
- **Operation timeout** — every registry op is bounded (30s). A timed-out session is evicted: mid-command protocol state can't be trusted, and follow-up commands would read desynced responses. The UI marks the connection lost and asks the user to reconnect.
- **Transfer engine** (`crates/transfer`) — dial-per-transfer workers (fresh connections, so browse and transfer never fight over one control stream), byte-offset resume, retry with backoff, token-bucket rate limit; progress flows to the UI as `transfer:update` events.
- **Quit guard** — on window close, pending transfers + live sessions are counted; if anything is at stake the app emits `quit-confirm` and the themed in-app dialog asks. The frontend answers via the `confirm_quit` command.
- **Diagnostics** — suppaftp and the adapters log a wire transcript (CWD/PWD, listing method + outcome, logins) to `~/Library/Logs/com.flowftp.app/ftp.log`, truncated each launch. It never contains passwords. This is how misbehaving servers get diagnosed — from real transcripts, not guesses.

---

## Async-First Architecture

All I/O is async, built on **Tokio**.

- Protocol calls → `async fn`, cancellation-safe where possible.
- Transfers stream progress via Tauri **events** (`transfer:update`, `edit:update`); the UI subscribes with the Tauri event API and updates rune stores.
- Blocking work (keychain reads, large directory scans via `walkdir`) runs on `tokio::task::spawn_blocking`.

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
                          ConnectionRegistry / TransferEngine
                                   │
                                   ▼
                          Infrastructure adapter
                          (protocols / keychain / …)
                                   │
                                   ▼
                          Events emitted back to UI
                          (transfer:update, quit-confirm, …)
                                   │
                                   ▼
                          Rune stores update
```

- **UI state** lives in Svelte runes stores (`app.svelte.ts`, `theme.svelte.ts`).
- **Domain state** (connections, transfer queue) lives in Application services on the Rust side.
- **Remote navigation is deterministic**: the frontend lists the target first and commits the path only on success (Finder behavior — a failure keeps you where you were, listing intact). In-flight listings are latest-wins, so a slow response can never overwrite a newer one.
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
- **Protocols**: unit tests on parsing/mapping; integration tests dial real servers and are skipped unless `FLOW_FTP_*` / `FLOW_SFTP_*` env vars are set (`cargo test -- --ignored`).
- **Application bridge**: mock-adapter tests prove registry dispatch, timeout eviction, and error shaping without a network.
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
| SFTP client | `russh` + `russh-sftp` | Pure-Rust SSH, async, no libssh2 C dep |
| Credential store | `keyring-rs` | Native macOS Keychain, no plaintext |
| Async runtime | Tokio | De facto Rust async standard |
| Frontend state | Svelte runes stores | One state model, no query-cache layer to keep in sync |
| FTP command style | CWD into directory, then relative names | Shared-hosting servers mishandle absolute paths in LIST/RETR/SIZE; matches what CyberDuck/FileZilla do |
| Op timeout + eviction | 30s ceiling per registry op | A stalled server surfaces as an actionable error; a mid-command session is never reused |
| FTP transcript log | `log` crate → file, truncated per launch | Diagnose real servers from real transcripts; credential-free by construction |
