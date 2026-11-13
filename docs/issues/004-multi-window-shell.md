# 004. Multi-window shell with per-entry bundle budgets

| Milestone      | Size | Depends on | Labels                |
| -------------- | ---- | ---------- | --------------------- |
| M0 Foundations | M    | #001       | area/ui, type/feature |

## Story

As a user, I want the overlay, Today, panel, and confirmation windows to appear instantly and load only what they need, so that the assistant feels native and stays light in the tray.

## Context

[UI: windows and entry points](../architecture/ui.md#windows-and-entry-points) defines five windows, each its own Vite entry with its own gzip JS budget. The overlay is pre-created hidden so the 50 ms budget holds ([os-integration: overlay](../architecture/os-integration.md#overlay-window)). Capabilities are scoped per window ([security: Tauri hardening](../architecture/security.md#tauri-hardening)).

## Scope

- Vite multi-page build: `index.html` → `src/main.tsx`, `overlay.html` → `src/overlay-main.tsx`, `today.html` → `src/today-main.tsx`, `panel.html` → `src/panel-main.tsx`, `confirm.html` → `src/confirm-main.tsx`.
- Tauri windows in `crates/desktop/tauri.conf.json`: `main` (normal, Mica later), `overlay` (created at startup, hidden: transparent, undecorated, always on top, skip taskbar, not focusable, no shadow, 160×160), `today`, `panel`, and `confirm` (created on demand, hidden until shown).
- Per-window capability files in `crates/desktop/capabilities/`: `main.json`, `overlay.json`, `today.json`, `panel.json`, `confirm.json`. The overlay gets `core:event` only: no file system, no shell, no window management of other windows.
- `scripts/bundle-budget.ts` checks each entry's own chunk graph against its budget: main 120 KB, overlay 25 KB, today 80 KB, panel 40 KB, confirm 30 KB (gzip JS, lazy routes excluded from main's initial budget).
- `src/shared/design/tokens.css` with placeholder tokens and the `Mark` primitive (static), so every entry renders something real.
- Remove the starter's `src/tasks` example and `features/ui/tasks.feature`. The shell's own acceptance scenarios replace them.

Out of scope: typed IPC (#008), overlay states and animation (#021), the Today content (#066), the confirm flow (#049), Win32 extended styles on the overlay (#021).

## Acceptance criteria

`features/ui/shell.feature` (Chromium, each entry served by Vite) and a bundle-budget check:

```gherkin
Feature: Multi-window shell

  Scenario Outline: Each entry renders its own root
    When I open "<page>"
    Then I see the "<landmark>" landmark
    And no module from another entry's feature folders was loaded

    Examples:
      | page          | landmark     |
      | /index.html   | main         |
      | /overlay.html | status       |
      | /today.html   | today        |
      | /panel.html   | answer panel |
      | /confirm.html | confirmation |

  Scenario: The overlay entry stays inside its budget
    Given a production build
    When the bundle budget runs
    Then it prints one line per entry with its gzip size and budget
    And the overlay entry is at most 25 KB

  Scenario: An entry over budget fails the build
    Given the overlay entry imports a 30 KB dependency
    When "make bundle" runs
    Then it fails naming the overlay entry, its size, and its 25 KB budget

  Scenario: The overlay capability grants events only
    Given the capability files in crates/desktop/capabilities
    When the capability check reads overlay.json
    Then it targets only the "overlay" window and grants only core:event permissions
    And no capability file targets every window with a wildcard
```

## Design notes

- Protected files changed (human approval): `vite.config.mjs`, `scripts/bundle-budget.ts`, `crates/desktop/tauri.conf.json`, `crates/desktop/capabilities/**`, `.dependency-cruiser.cjs`, `knip.json`, `Makefile` (`BUNDLE_BUDGET_KB` becomes a per-entry map in the script).
- dependency-cruiser: `COMPOSITION_ROOT` becomes `^src/[a-z]+-main\.tsx$|^src/main\.tsx$`. Add rules: `src/shared/` must not import any feature; entries may import only their own features plus `shared`; features import each other only through `src/<feature>/index.ts`.
- Coverage and mutation exclude all `src/*-main.tsx` files, so they must stay wiring-only.
- The `main` window is hidden at startup under `--background` (#009) and shown under a normal launch.
- CSP is unchanged and applies to every window.
- Performance: the overlay entry must import nothing from `src/shared/ipc` beyond the event subscription, and no zod at the top level (zod's size alone would threaten 25 KB; use `zod/mini` or hand-written guards).

## QA procedure

1. Run `make dev`. The main window opens; the overlay does not appear and has no taskbar button.
2. Open `http://localhost:1420/overlay.html` in Edge. A transparent page with the static mark renders.
3. Run `make bundle`. The output lists five entries with sizes under their budgets.
4. Add `import 'zod'` to `src/overlay-main.tsx` and run `make bundle`. It fails naming the overlay budget. Revert.
5. In the running app, open DevTools on the overlay (debug build) and call a settings command. It is rejected.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed (`react-gates.md` layout, `ui.md` if budgets move)
