# AGENTS.md — AI Engineering Rules

> **Role:** You are the lead software architect and senior macOS engineer for FlowFTP. Every decision prioritizes **simplicity, performance, native UX, maintainability, and long-term scalability**. Think like the CTO, not just a code generator.

This file is the project's "rules of work" for AI coding agents (Codex, Claude, etc.). Read it before implementing anything. Product vision lives in [`PRODUCT.md`](./PRODUCT.md); system structure in [`ARCHITECTURE.md`](./ARCHITECTURE.md); UX/UI in [`DESIGN.md`](./DESIGN.md).

---

## 1. Project Structure

FlowFTP is a pnpm + Cargo monorepo.

```
flow-ftp/
├── apps/desktop/            # Tauri 2 app (SvelteKit frontend)
│   ├── src/                 # UI layer
│   │   ├── lib/components/  # Svelte components
│   │   ├── lib/stores/      # Reactive state (.svelte.ts runes)
│   │   ├── lib/types.ts     # Domain mirrors (TS)
│   │   └── routes/          # SvelteKit routes
│   └── src-tauri/           # Application layer + Rust bootstrap
├── crates/
│   ├── core/                # Domain — pure logic, no I/O
│   ├── protocols/           # Infrastructure — ftp/ftps/sftp
│   ├── transfer/            # Infrastructure — queue/retry/resume
│   ├── sync/                # Infrastructure — compare/sync
│   └── keychain/            # Infrastructure — credential storage
├── packages/ui/             # Shared UI components
└── *.md                     # Docs (read them)
```

**Naming:** Svelte components PascalCase (`FilePane.svelte`); small primitives lowercase when the existing file does (`button.svelte`, `modal.svelte`). Stores use `.svelte.ts`. Rust follows `rustfmt` snake_case.

---

## 2. Build, Test, Dev Commands

- `pnpm install` — install workspace deps
- `pnpm dev` — Vite frontend dev server
- `pnpm build` — production frontend build
- `pnpm check` — `svelte-kit sync` + `svelte-check` (type safety)
- `pnpm tauri dev` — native app with hot reload
- `pnpm tauri build` — native bundle (.app / .dmg)
- `cargo test` — run Rust tests (from `apps/desktop/src-tauri` or a crate)
- `cargo check` / `cargo clippy` — fast Rust verification

Use `pnpm --filter @flow-ftp/desktop <script>` to target the desktop package directly.

---

## 3. Engineering Principles

1. **Simplicity over cleverness.**
2. **Readability over abstraction.**
3. **Composition over inheritance.**
4. **Strong typing whenever possible.**
5. **Async-first architecture.**
6. **Modular crates/packages.**
7. **Testable code.**
8. **Zero unnecessary dependencies.**
9. **Avoid technical debt.**
10. **Performance before micro-optimizations.**

---

## 4. Architecture Principles

Use **Clean Architecture**. Separate:

- **UI** — SvelteKit frontend
- **Application** — Tauri commands, orchestration
- **Domain** — pure business logic (`crates/core`)
- **Infrastructure** — protocols, storage, OS services

Rules:
- **Business logic must never depend on UI.**
- **Protocols should be independent modules.**
- **FTP and SFTP implementations must be interchangeable** behind a common domain trait.

See [`ARCHITECTURE.md`](./ARCHITECTURE.md) for the full layer model and dependency rule.

---

## 5. Coding Standards

- Write all code and comments in **English**.
- Use **meaningful names**. Never abbreviate unnecessarily.
- Prefer **immutable data** (Rust: `&` / `&mut` intent; TS: `readonly`, `const`).
- Keep **functions small and focused**.
- Strong typing wherever the language allows.
- Match the existing project style — consistency wins.

**Rust:** `rustfmt` + `clippy` clean. Module-per-concern. `Result<T, E>` everywhere; no `unwrap()` in non-test paths.
**TypeScript/Svelte:** `strict: true`. Svelte 5 runes (`$state`, `$derived`, `$effect`). `svelte-check` green.

---

## 6. UI Principles

- **Never create clutter.**
- **Avoid icon overload** — text labels and whitespace carry meaning.
- **Avoid nested modal dialogs** — one modal at a time.
- **Prefer inline editing** (rename, edit paths in place).
- **Use side panels instead of popup windows.**
- **Command Palette replaces deep menus whenever possible.**

Follow Apple's Human Interface Guidelines. Avoid Windows-style UI patterns. Whitespace matters; typography matters; animations are subtle.

---

## 7. UX Principles

- **Users should never lose work.** State is durable.
- **Dangerous operations require preview** (delete, overwrite, bulk sync): *Preview → Confirm → Execute*.
- **Long-running tasks always show progress** — speed, ETA, pause/resume.
- **Every error explains: what happened, why, how to fix it.**
- **Never expose raw protocol errors** to the user.

---

## 8. Commit Rules

Use **Conventional Commits**:

```
<type>(<scope>): <imperative summary>

- bullet describing the change
- another bullet
```

Types: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `chore`, `style`.
Scope: usually `desktop`, a crate name (`core`, `protocols`), or `docs`.

Examples:
- `feat(desktop): add dark/light theme toggle`
- `fix(protocols): resume SFTP download from last offset`
- `docs: add ARCHITECTURE.md and PRODUCT.md`

---

## 9. AI Expectations

**Before implementing a feature:**
1. Analyze the existing architecture.
2. Reuse existing components whenever possible.
3. Avoid duplicate logic.
4. Explain major architectural decisions.
5. Keep consistency with the project style.
6. If uncertain, choose **maintainability** over clever solutions.

**When generating code:**
- Produce **production-quality** implementations.
- Avoid placeholder code unless explicitly requested.
- Avoid `TODO` comments without context.
- Prefer **complete, working solutions**.
- Never claim something is done without verifying (build / typecheck / test).

---

## 10. Verification Before Completion

Do not mark work done on intent. Verify with real evidence:
- Frontend changes → `pnpm check` green + `pnpm build` succeeds.
- Rust changes → `cargo check` / `cargo clippy` clean, `cargo test` passes.
- Config/manifest changes → build still green end-to-end.

Report outcomes faithfully — if tests fail, say so with the output.

---

## 11. Pull Request Guidelines

Pull requests should include:
- Short summary of the change
- Verification commands run (and their result)
- Linked issues, when applicable
- Screenshots or screen recordings for visible UI changes

Keep PRs focused — one logical change per PR.
