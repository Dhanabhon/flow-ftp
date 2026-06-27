# Contributing to FlowFTP

Thanks for your interest in FlowFTP. This project aims to be the best open-source FTP/SFTP client for developers, and every contribution helps.

> Read these docs before opening a PR: [`PRODUCT.md`](./PRODUCT.md), [`ARCHITECTURE.md`](./ARCHITECTURE.md), [`DESIGN.md`](./DESIGN.md), [`AGENTS.md`](./AGENTS.md).

---

## Open Source Philosophy

- **Code should be understandable by contributors.**
- **Document public APIs.**
- **Keep modules independent.**
- **Prefer explicit code over magic.**
- **Every feature should have documentation.**
- **Every major module should have unit tests.**

---

## Prerequisites

- **Node.js** ≥ 20 and **pnpm** ≥ 9
- **Rust** (stable) via [rustup](https://rustup.rs/)
- **macOS** (primary target) with [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/)

## Setup

```bash
git clone https://github.com/Dhanabhon/flow-ftp.git
cd flow-ftp
pnpm install
pnpm dev          # frontend dev server on :1420
```

To run as a native app:

```bash
pnpm tauri dev
```

---

## Development Workflow

1. **Branch** off `main`: `git checkout -b feat/my-feature`
2. **Implement** following the standards in [`AGENTS.md`](./AGENTS.md).
3. **Verify** before committing (see Verification below).
4. **Commit** using Conventional Commits.
5. **Open a PR** with a focused summary and verification evidence.

### Verification

Frontend changes:
```bash
pnpm check        # svelte-check (type safety) — must be green
pnpm build        # production build — must succeed
```

Rust changes:
```bash
cargo check       # from apps/desktop/src-tauri or a crate
cargo clippy      # must be warning-clean
cargo test        # must pass
```

---

## Testing Guidelines

- **Name** frontend tests `*.test.ts` / `*.spec.ts`, placed near the code they cover.
- **Name** Rust tests `#[cfg(test)]` modules in the file they test, or in a `tests/` directory for integration tests.
- **Domain crate** (`crates/core`) — pure unit tests, no I/O.
- **Protocols** — integration tests against a local FTP/SFTP test server.
- **Every major module should have unit tests.** New logic without tests is incomplete.

No dedicated frontend test runner is configured yet. Until one is added, run `pnpm check` + `pnpm build` as the baseline gate for UI changes.

---

## Commit Message Convention

Use **Conventional Commits**:

```
<type>(<scope>): <imperative summary>

- bullet describing the change
```

- **Types:** `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `chore`, `style`
- **Scope:** `desktop`, a crate name (`core`, `protocols`, `transfer`, `sync`, `keychain`), or `docs`
- **Summary:** imperative, lowercase, no trailing period

Examples:
```
feat(desktop): add dark/light theme toggle
fix(protocols): resume SFTP download from last offset
docs: add ARCHITECTURE.md and PRODUCT.md
test(core): add diff unit tests for folder compare
```

---

## Coding Standards

- All code and comments in **English**.
- **Meaningful names** — never abbreviate unnecessarily.
- Prefer **immutable data**.
- Keep **functions small and focused**.
- Strong typing wherever the language allows.
- Match the existing project style — consistency wins.

**Rust:** `rustfmt` + `clippy` clean. `Result<T, E>` everywhere; no `unwrap()` outside tests.
**TypeScript/Svelte:** `strict: true`. Svelte 5 runes. `svelte-check` green.

---

## Pull Request Checklist

- [ ] Branch is up to date with `main`
- [ ] `pnpm check` / `cargo check` passes
- [ ] `cargo clippy` is warning-clean (for Rust changes)
- [ ] Tests added or updated for new logic
- [ ] Public APIs documented
- [ ] Commit messages follow Conventional Commits
- [ ] PR description includes verification commands run
- [ ] Screenshots/recordings for visible UI changes

---

## Reporting Issues

- Search existing issues before opening a new one.
- Include: macOS version, FlowFTP version, steps to reproduce, expected vs actual behavior, and logs/screenshots.
- For crashes, attach the backtrace if available.

---

## Code of Conduct

Be respectful and constructive. This is a community project — assume good intent, give good intent. Personal attacks, harassment, or discrimination are not tolerated.

---

Thank you for helping make FlowFTP better. 🚀
