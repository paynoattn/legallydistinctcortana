# Agents

Local-first desktop voice assistant: Tauri 2 shell, Rust core, React 19 UI. Built from the `rust-axum` and `react-vite-ts` starters. Every rule that matters is a gate, so run it instead of reading about it.

## Commands

- `make setup` once (Rust toolchain, pinned cargo gate tools, pnpm deps, Chromium, git hooks). Needs `make`, `jq`, `pnpm`, Rust; on Windows run from Git Bash.
- `make check` before you stop (the stop hook and pre-push run it too). It is `fmt-check` plus `check-rs` and `check-ui`; run one half for faster feedback.
- Single gates, each with `-rs` and `-ui` halves: `lint typecheck test acceptance coverage crap arch mutate`. Then `qa` (boots `headless`, serves the built UI), `bundle` (UI build + budget), `dev` (Tauri + Vite hot reload), `build` (installer).
- `make gauntlet` = `check mutate qa`. While hardening: `make mutate SCOPE=changed`.
- One Rust test: `cargo test -p <crate> <test_name_substring>`. One Rust acceptance scenario: `cargo test -p app --test acceptance -- --name "<scenario regex>"`.
- One UI test: `pnpm exec vitest run src/<path>.spec.ts -t "<name>"`. One UI scenario: `pnpm exec cucumber-js features/ui/<file>.feature:<line>`.

## Layout

- `crates/<context>-{domain,application,infrastructure}`: Rust bounded contexts. Domain is pure; application holds use cases and port traits; infrastructure holds adapters. Allowed crate deps are in `arch.toml` (every crate must be listed).
- Composition roots only wire: `crates/desktop` (Tauri; the only crate allowed to depend on `tauri`), `crates/app` (headless HTTP host, becoming `headless`). Both are excluded from coverage and mutation, so logic placed there fails nothing yet still goes untested.
- `src/<feature>/{domain,application,infrastructure,ui}`: React features, wired only in `src/main.tsx`; cross-feature imports go through `src/<feature>/index.ts`. Rules: `.dependency-cruiser.cjs`.
- `features/core/` (cucumber-rs, in process against the router), `features/ui/` (cucumber-js + Playwright against Vite). `qa/procedures/` for humans, `qa/scripts/{core,ui}/` for their scripted form.
- `docs/`: product vision and MVP plan, architecture and ADRs, issue docs (`docs/issues/`, numbered, one story each), brand, design artboards.
- The `tasks-*`, `http-api`, and `app` crates and `src/tasks` are the starters' example slice, kept so gates stay meaningful until issue 003 replaces them.

## Rules

- Files in `.gates-protected` need human approval. Never loosen a gate to get green. Tauri's `tauri.conf.json` (CSP) and `capabilities/` are gates too.
- Rust: suppress only with `#[expect(lint, reason = "...")]`, never `#[allow]`. No comments except doc comments, no `as` casts, no `unwrap`/`expect`/`panic` outside tests.
- TS/React: no `useEffect`, no manual memoization, no comments, no `as`. Subscribe with `useSyncExternalStore`. Lint errors say what to do instead.
- Brand-neutral code: never put "Cortana" in shipped strings. The product name lives in `crates/desktop/tauri.conf.json` (and later `src/shared/brand.ts`).
- Privacy: anything that sends data off-device goes through a port and the model router's data-class policy (docs/architecture/assistant.md). Tools that act declare an effect (ADR 0009).
- Work issues through the `gauntlet` skill (specifier → coder → cleaner → hardener → qa). Skills live in `.claude/skills` and `.cursor/skills` (keep them identical).
