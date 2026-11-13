# 001. Repository bootstrap: combined starters, gates green on Windows and Linux

Status: done (2026-11-14)

| Milestone      | Size | Depends on | Labels                      |
| -------------- | ---- | ---------- | --------------------------- |
| M0 Foundations | M    | none       | area/foundation, type/chore |

## Story

As a contributor, I want one repository that holds the Rust core, the Tauri shell, and the React UI under the starters' gates, so that every later issue starts from a green `make check` on Windows and Linux.

## Context

[ADR 0001](../architecture/adr/0001-tauri-and-rust-core.md) chose Tauri 2 + Rust core + React 19 and said the repository combines the `rust-axum` and `react-vite-ts` starters and their gates. The gate contract is in [`../engineering/gate-contract.md`](../engineering/gate-contract.md). The target layout is in [`../architecture/overview.md`](../architecture/overview.md#repository-layout-target).

## What was done

- **Combined the starters** from `D:\www\starters` into one repo:
  - Rust workspace at the root: `crates/*` plus `xtask`. The starter's example crates (`tasks-domain`, `tasks-application`, `tasks-infrastructure`, `http-api`, `app`) stay until [#003](003-replace-starter-example.md).
  - React UI at the root: `src/`, `index.html`, `vite.config.mjs`. The starter's `src/tasks` example stays as the UI gate exerciser.
  - The Tauri crate moved to `crates/desktop` (package `desktop`, lib `desktop_lib`; the `_lib` suffix avoids cargo#8519 on Windows). `tauri.conf.json` and `capabilities/` live there and are gate-protected.
- **Agent tooling:** copied the skills (gauntlet, specifier, coder, cleaner, hardener, qa, code-smells, architecture-guard, encode-rule) to `.claude/skills` and `.cursor/skills`, the agent hooks (`stop-gate`, `protect-gates`) to `scripts/agent-hooks/`, the git hooks to `.githooks/`, and CI to `.github/workflows/ci.yml`.
- **One `Makefile`** with `-rs` and `-ui` halves for every gate target: `lint-rs`/`lint-ui`, `typecheck-*`, `arch-*`, `coverage-*`, `crap-*`, `acceptance-*`, `mutate-*`, `qa-*`, plus `check-rs`, `check-ui`, `check`, and `gauntlet`.
- **Acceptance split:** `features/core/` (cucumber-rs, run in process against `app`) and `features/ui/` (cucumber-js + Playwright against Vite). QA scripts split the same way: `qa/scripts/core/` (bash + curl) and `qa/scripts/ui/` (Playwright).
- **Windows fixes:**
  - `protect-gates.sh` normalizes `D:\...` paths before matching `.gates-protected`, and fails closed when `jq` is missing.
  - The Makefile calls `pnpm exec` on Windows, because GNU make's direct exec cannot run the `node_modules/.bin` shims.
  - Line endings are normalized to LF through `.gitattributes`.
  - `#[expect(clippy::unnecessary_wraps)]` on the non-unix `shutdown_signal`, which has nothing to fail on.
- **Supply chain:** the `cargo-deny` exceptions Tauri needs (skip-tree for `tauri@2`/`tauri-build@2`, MPL-2.0 and Zlib, RUSTSEC-2024-0370) are recorded in [ADR 0003](../architecture/adr/0003-supply-chain-exceptions.md).

Out of scope: replacing the example crates (#003), multiple windows and per-entry budgets (#004), the final product name (#005).

## Acceptance criteria

These were checked by hand at bootstrap. They stay true for every later issue, because `make check` enforces them.

```gherkin
Feature: Repository gates

  Scenario: Rust half is green on Windows
    Given a fresh clone on Windows 11 with the pinned toolchain from rust-toolchain.toml
    When the developer runs "make setup" and then "make check-rs"
    Then clippy pedantic, cargo-deny, the comments gate, arch, coverage, CRAP, and acceptance all pass

  Scenario: UI half is green on Windows
    Given a fresh clone on Windows 11 with the Node version from .nvmrc
    When the developer runs "make check-ui"
    Then ESLint, knip, tsc, dependency-cruiser, coverage, CRAP, and the Playwright acceptance pass

  Scenario: An agent cannot loosen a gate silently
    Given the Claude Code PreToolUse hook is installed
    When an agent edits "arch.toml" through a "D:\" style path
    Then protect-gates matches the file against .gates-protected and asks a human first

  Scenario: The hook fails closed without jq
    Given jq is not on PATH
    When an agent edits any file
    Then protect-gates blocks the edit and says jq is required
```

## Design notes

- Verified results on Windows: coverage 99.9% (Rust) and 100% (UI); CRAP ≤ 6 everywhere; 16 core and 13 UI acceptance scenarios.
- `crates/desktop` is excluded from coverage and mutation (`COVERAGE_IGNORE` in the Makefile and `.cargo/mutants.toml`), so it must stay wiring-only.
- CI runs `make check qa` plus `make mutate SCOPE=changed` on Linux and a clippy, test, and Tauri bundle job on `windows-latest`.

## Follow-ups (not done)

- `make mutate` and `make qa` have not been run on Windows yet.
- `scripts/qa-rs.sh` checks graceful shutdown with SIGTERM, which is Unix-specific. Windows needs a different stop signal (Ctrl+Break or a `/shutdown` endpoint on the local API) before `make qa-rs` can pass there. #003 is the natural place, because it replaces `app` with `headless`.

## QA procedure

1. On Windows 11, clone the repo and run `make setup`. It installs the pinned toolchains, the cargo gate tools, and Chromium, and points git at `.githooks`.
2. Run `make check`. Every gate prints a pass line and the command exits 0.
3. Run `make dev`. A window titled "Assistant" opens with the starter tasks UI.
4. Ask an agent to edit `deny.toml`. The hook asks for approval before the edit.

## Definition of done

- [x] `make check-rs` and `make check-ui` green on Windows 11
- [x] CI green on Linux
- [ ] `make mutate` and `make qa` run on Windows (follow-up)
- [x] ADR 0003 records the cargo-deny exceptions
