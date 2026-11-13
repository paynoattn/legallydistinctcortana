# 008. Typed IPC with tauri-specta, freshness gate, and fake adapter

| Milestone      | Size | Depends on | Labels                          |
| -------------- | ---- | ---------- | ------------------------------- |
| M0 Foundations | M    | #004       | area/ipc, area/ui, type/feature |

## Story

As a developer, I want every Rust command and event typed end to end, with a fake I can drive in Chromium, so that UI and core cannot drift apart and the whole UI is testable without Rust.

## Context

[Overview: cross-context communication](../architecture/overview.md#cross-context-communication) and [UI: IPC](../architecture/ui.md#ipc): commands and events are typed with `tauri-specta`, bindings go to `src/shared/ipc/bindings.ts`, a gate checks freshness (`cargo xtask ipc-bindings --check`), adapters validate with zod at the boundary, and acceptance runs against a fake chosen by the composition root (`?ipc=fake`).

## Scope

- `tauri-specta` + `specta` in `desktop` (verify the release compatible with the pinned Tauri 2 version). Commands are thin: parse the input into domain newtypes, call a use case, map the error.
- A first real command pair to prove the path: `app_info() -> AppInfo { version, build_flavor, data_dir_kind }` and `ping(n) -> n`.
- Error contract: every command returns `Result<T, IpcError>` with `IpcError { code, message, fix: Option<FixAction> }`. `code` is a closed string enum the UI maps to copy.
- Binding generation: an `export-bindings` binary in `crates/desktop` writes `src/shared/ipc/bindings.ts`. `cargo xtask ipc-bindings` runs it; `--check` fails when the file on disk differs. `make lint-rs` runs the check.
- UI side, `src/shared/ipc/`:
  - `bindings.ts` (generated, excluded from lint rules that do not apply to generated code, never edited by hand).
  - `IpcPort` interface consumed by feature infrastructure adapters.
  - `TauriIpc` adapter (wraps bindings, parses results with zod or `zod/mini`).
  - `FakeIpc` adapter with a scriptable in-memory state that acceptance steps control through `window.__ipcFake`.
- Composition roots choose the adapter: `?ipc=fake` loads `FakeIpc` through a dynamic `import()` only when the build allows fakes (`VITE_IPC_FAKE=1`, set for dev, acceptance, and QA builds, never for release).

Out of scope: domain events and their forwarding (#011), settings commands (#010), per-feature commands (each feature issue).

## Acceptance criteria

```gherkin
Feature: Typed IPC

  Scenario: The main window shows app info over IPC
    Given the main window is opened with "?ipc=fake"
    And the fake returns version "0.1.0" and flavor "cpu-vulkan"
    When I open About
    Then I see "0.1.0" and "cpu-vulkan"

  Scenario: A malformed response is rejected at the boundary
    Given the fake returns app_info with version = 42
    When I open About
    Then I see "The app sent an unexpected response" with a "Copy details" button
    And the component tree did not receive the malformed value

  Scenario: Typed errors reach the UI with their fix action
    Given ping fails with code "core_unavailable" and fix "restart_core"
    When the UI calls ping
    Then the error banner offers "Restart"

  Scenario: Stale bindings fail the gate
    Given a command's return type changed in Rust
    And bindings.ts was not regenerated
    When "make lint-rs" runs
    Then it fails with "IPC bindings are stale: run cargo xtask ipc-bindings"

  Scenario: Release builds cannot load the fake
    Given a production build without VITE_IPC_FAKE
    When the main window is opened with "?ipc=fake"
    Then the Tauri adapter is used and no fake module appears in dist/assets
```

## Design notes

- `arch.toml`: `desktop` keeps `tauri`, `tauri-specta`, and `specta`. If domain newtypes need `specta::Type`, put the derive behind a `specta` feature in the domain crate, enabled only by `desktop`, and keep `serde_json` out of domain crates.
- Protected files (human approval): `xtask/**` (new subcommand), `Makefile`, `knip.json`, `eslint.config.mjs` (generated-file overrides), `.dependency-cruiser.cjs` (`src/shared/ipc` may be imported only by `infrastructure` layers and composition roots).
- The freshness gate must run on Linux CI, so binding export must not need a window or WebView; verify `tauri-specta` export works headless.
- Security: commands are allow-listed per window in capability files (from #004). Every command validates its input in Rust; the UI's zod parse defends the webview, not the core.
- Performance: zod at the boundary must not push the overlay or confirm entries over budget; those entries use hand-written guards or `zod/mini`.

## QA procedure

1. Run `cargo xtask ipc-bindings`. `src/shared/ipc/bindings.ts` is regenerated with no diff on a clean tree.
2. Change `AppInfo.version` to a number in Rust and run `make lint-rs`. The stale-bindings error appears. Revert.
3. Run `make dev` and open About. The real version and flavor appear.
4. Run `pnpm exec vite build` without `VITE_IPC_FAKE` and search `dist/assets` for `FakeIpc`. Nothing is found.
5. Open `http://localhost:1420/?ipc=fake` in Edge. The app renders with fake data and no Rust running.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed (`ui.md` IPC section, `react-gates.md`)
