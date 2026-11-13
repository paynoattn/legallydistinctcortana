# UI architecture

React 19 + React Compiler + TypeScript 6 in Tauri webviews, under the React starter's gates: no `useEffect`, no manual memoization, no comments, no `as`, complexity ≤ 6, coverage ≥ 90%, CRAP ≤ 6, 100% mutation score on `make gauntlet`. Rationale: [`../engineering/poteto-react.md`](../engineering/poteto-react.md), [`../engineering/react-gates.md`](../engineering/react-gates.md).

## Windows and entry points

Each window is its own Vite entry, so the overlay does not load the settings UI. The bundle budget applies per entry.

| Window    | Entry                                   | Purpose                                                            | Budget (gzip JS)         |
| --------- | --------------------------------------- | ------------------------------------------------------------------ | ------------------------ |
| `main`    | `index.html` → `src/main.tsx`           | Home, conversation, reminders, accounts, models, privacy, settings | 120 KB, plus lazy routes |
| `overlay` | `overlay.html` → `src/overlay-main.tsx` | Bottom-right voice indicator and reading pill                      | 25 KB                    |
| `today`   | `today.html` → `src/today-main.tsx`     | Full-screen daily briefing                                         | 80 KB                    |
| `panel`   | `panel.html` → `src/panel-main.tsx`     | Floating assistant answer near the cursor                          | 40 KB                    |
| `confirm` | `confirm.html` → `src/confirm-main.tsx` | Action preview and confirmation (always on top, focusable)         | 30 KB                    |

Raising `BUNDLE_BUDGET_KB` into per-entry budgets is a gate change: `scripts/bundle-budget.ts` is protected, so a human must approve (issue [#004](../issues/004-multi-window-shell.md)).

## Feature layout

Each feature follows the starter's layers, enforced by dependency-cruiser:

```
src/<feature>/domain/          pure TS: view models, parsing, formatting (no React)
src/<feature>/application/     stores (useSyncExternalStore-compatible), use cases, ports (interfaces)
src/<feature>/infrastructure/  adapters: Tauri IPC commands and events, fakes for tests and acceptance
src/<feature>/ui/              components
src/<feature>/index.ts         public entry for other features
src/shared/                    design system (tokens, primitives), ipc bindings (generated), brand.ts
src/*-main.tsx                 composition roots, one per window: pick adapters, mount the app
```

Planned features: `dictation`, `overlay`, `speech`, `assistant` (conversation + panel), `reminders`, `calendar`, `mail`, `feeds`, `projects`, `today`, `accounts`, `models`, `privacy`, `devices` (sync and pairing), `onboarding`, `routines`, `settings`.

## IPC

- Rust commands and events are typed with `tauri-specta`. Generated bindings go to `src/shared/ipc/bindings.ts`. A gate checks that bindings are fresh (`cargo xtask ipc-bindings --check`).
- UI infrastructure adapters wrap the generated functions and convert results into domain types with zod at the boundary, because the webview must not trust the shape of anything it receives.
- Events (levels, state changes, digests) reach stores through a subscription port. Stores expose `subscribe` and `getSnapshot` for `useSyncExternalStore`. No component subscribes to Tauri directly.
- Acceptance tests (`features/ui/`) run the same UI in Chromium with the **fake IPC adapter** chosen by the composition root (`?ipc=fake`), so the whole UI can be driven without Rust.

## Design system

- Tokens (color, type, spacing, radius, motion) come from the chosen brand ([`../brand/`](../brand/)) and live in `src/shared/design/tokens.css` as CSS custom properties, with light and dark schemes following Windows (`prefers-color-scheme`) and a high-contrast mode (`forced-colors`).
- Primitives: Button, IconButton, Toggle, Select, Dialog, Toast, Card, List, Kbd, Meter (audio level), Mark (the animated brand mark used by the overlay and Today).
- Windows 11 integration: Mica backdrop on `main` (`window-vibrancy`), Segoe Fluent Icons only where we mirror OS chrome, and otherwise the brand icon set.
- Motion: one shared easing set. Every animation respects `prefers-reduced-motion` and the in-app "Reduce motion" setting.

## Accessibility

- jsx-a11y strict is already a lint gate. In addition: every window is fully keyboard-operable, focus is always visible, and the overlay announces state changes through an ARIA live region mirrored to the main window for screen-reader users (the overlay itself is not focusable).
- Narrator and NVDA pass on onboarding, reminders, and Today before release (QA procedure).
- Text scales with the Windows text-size setting; layouts are tested at 100%, 150%, and 200% scale.

## Today view content

The Today snapshot (from `briefing-application`) drives one full-screen layout:

1. **Now:** greeting, time, weather (optional connector), and the next event with a countdown and join button.
2. **Schedule:** today's timeline (events and timed reminders), with conflicts highlighted.
3. **Needs you:** mail that needs a reply (from triage), with a one-line summary each, and reminders due today.
4. **For you:** top news for your interests, three to seven items, each with a source and a one-line reason ("because you follow: local LLMs").
5. **Projects:** each watched project with branch, uncommitted changes, CI status, open PRs, and the last AI agent session (what it did and whether it finished).
6. **Start my day:** closes to the desktop. "Read it to me" speaks the briefing in the chosen voice.
