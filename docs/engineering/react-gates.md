> Copied from the starter this repo was built from. In this repo: gates run through the combined root `Makefile` with `-rs` and `-ui` halves (`make lint-rs`, `make coverage-ui`, ...); Rust features live in `features/core/`, UI features in `features/ui/`; QA scripts in `qa/scripts/{core,ui}/`; the scripts are `scripts/{mutate,qa}-{rs,ui}.sh`; the Tauri shell is `crates/desktop`. See `AGENTS.md`.

# react-vite-ts starter

React 19 + Vite 8 + TypeScript 6 starter where quality and performance rules are gates, not guidelines. Every target in the shared gate contract exists with the same name, so the agent skills in `.cursor/skills` work unchanged.

On top of the shared gates, this starter encodes the React lessons of Lauren Tan ([poteto](https://x.com/poteto), React core team, now at SpaceX AI). The headline one: **`useEffect` is banned and CI fails if you use it.** See [`../docs/poteto-react.md`](poteto-react.md) for what she said, where, and which gate enforces each lesson.

## Setup

Requires Node 26 (`.nvmrc`) and pnpm 12 (`packageManager` in `package.json`; `corepack enable` or install pnpm yourself).

```sh
make setup   # pnpm install --frozen-lockfile, Chromium headless shell for Playwright, git hooks when this is the repo root
make check   # what CI, the pre-push hook, and the agent stop hook run
make dev     # Vite dev server
```

On Linux, Chromium also needs system libraries: `node_modules/.bin/playwright install-deps chromium` (CI does this).

## Targets

| Target            | What it runs                                                                                                                                              |
| ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `make fmt`        | `prettier --write .`                                                                                                                                      |
| `make fmt-check`  | `prettier --check .`                                                                                                                                      |
| `make lint`       | `eslint . --max-warnings=0`, then `knip`                                                                                                                  |
| `make typecheck`  | `tsc --noEmit` twice: `src/tsconfig.json` (browser) and `tsconfig.json` (features, qa, scripts on Node)                                                   |
| `make test`       | `vitest run` in jsdom (`src/**/*.spec.{ts,tsx}`)                                                                                                          |
| `make acceptance` | `cucumber-js` over `features/`: an in-process Vite dev server (React Compiler on) driven by Playwright Chromium                                           |
| `make coverage`   | `vitest run --coverage`, thresholds 90% lines/branches/functions/statements, writes `coverage/coverage-final.json`                                        |
| `make crap`       | `node scripts/crap.ts`, fails if any function's CRAP > `CRAP_MAX` (default 6)                                                                             |
| `make arch`       | `depcruise src features qa scripts`                                                                                                                       |
| `make mutate`     | Stryker over `src/` (break at 100%). `make mutate SCOPE=changed [BASE=origin/main]` mutates only `src` files changed vs BASE                              |
| `make build`      | `vite build` with React Compiler at `panicThreshold: 'all_errors'`, then `scripts/bundle-budget.ts` (gzip JS ≤ `BUNDLE_BUDGET_KB`, default 80)            |
| `make qa`         | `build`, then `vite preview` on a free port, waits for `/`, runs `qa/scripts/*.ts` (Playwright, including a long-animation-frame check), stops the server |
| `make check`      | `fmt-check lint typecheck arch coverage crap acceptance`                                                                                                  |
| `make gauntlet`   | `check mutate qa`                                                                                                                                         |

`make crap` reads the existing coverage report and refuses to run if any `src` file is newer than it; run `make coverage` first (`make check` does). `SCOPE=changed` behaves as in the other starters: it diffs committed, staged, unstaged, and untracked files against `merge-base(BASE, HEAD)`, and falls back to a full run if `BASE` does not resolve.

## Where things live

| Concern               | File                                                                                                                  |
| --------------------- | --------------------------------------------------------------------------------------------------------------------- |
| Formatting            | `.prettierrc`, `.prettierignore`                                                                                      |
| Lint rules            | `eslint.config.mjs`, `eslint.local-rules.mjs` (the `no-comments` rule)                                                |
| Unused code/deps      | `knip.json`                                                                                                           |
| Compiler              | `tsconfig.base.json` (shared strict options), `src/tsconfig.json` (browser), `tsconfig.json` (Node side)              |
| Build, React Compiler | `vite.config.mjs`, `index.html`                                                                                       |
| Unit tests / coverage | `vite.config.mjs` (`test`), `vitest.setup.ts`                                                                         |
| CRAP                  | `scripts/crap.ts`                                                                                                     |
| Bundle budget         | `scripts/bundle-budget.ts`                                                                                            |
| Architecture          | `.dependency-cruiser.cjs`                                                                                             |
| Mutation              | `stryker.config.json`, `scripts/mutate.sh`                                                                            |
| Acceptance            | `cucumber.json`, `features/*.feature`, `features/steps/*.ts`                                                          |
| QA                    | `qa/procedures/*.md` (human steps), `qa/scripts/*.ts` (Playwright, run with Node's type stripping), `scripts/qa.sh`   |
| Protected gate files  | `.gates-protected` (agents must ask a human before editing these)                                                     |
| Agent skills & hooks  | `.cursor/`, `.claude/`, `scripts/agent-hooks/`, `.githooks/` (synced from the monorepo's `shared/`; do not edit here) |

## Layout

```
index.html                Vite entry
src/
  main.tsx                composition root: builds the store from adapters, renders the app (excluded from coverage and mutation)
  styles.css
  tasks/
    domain/               Task, TaskTitle and TaskId brands, parse functions. Pure: imports nothing else
    application/          TaskStore (use cases + external store for useSyncExternalStore), TaskRepository and IdGenerator ports. No React
    infrastructure/       LocalStorageTaskRepository (parses with zod/mini at the boundary), CryptoIdGenerator
    ui/                   TaskApp, NewTaskForm, TaskList. Receives the store through props, never imports infrastructure
features/                 Gherkin + steps (black box: may not import src/)
qa/                       procedure + Playwright script against the production build
```

The example app is the shared `tasks` feature as a UI: add a task (title trimmed, 1–200 characters, otherwise an inline error), list in creation order, complete (final, so completing twice is a no-op), and persistence across reloads. A static SPA has no `/health`, so `make qa` waits for `/` instead.

## No effects

`useEffect`, `useLayoutEffect`, `useInsertionEffect`, and `useEffectEvent` cannot be imported (`no-restricted-imports`), `import * as React` and the default import are banned so `React.useEffect` cannot sneak in, and the ban cannot be disabled inline (`eslint-comments/no-restricted-disable`). What to write instead:

| You wanted an effect to...              | Write this                                                                                                       |
| --------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| compute a value from props or state     | compute it during render; React Compiler memoizes it                                                             |
| respond to a click, submit, or keypress | do it in that event handler (`NewTaskForm` saves and resets in `onSubmit`)                                       |
| subscribe to something outside React    | `useSyncExternalStore(store.subscribe, store.getSnapshot)` (`TaskApp` + `TaskStore`)                             |
| reset state when an id changes          | render the component with `key={id}`                                                                             |
| load data                               | pass a promise down and read it with `use(promise)` under `<Suspense>`, or load in the event that needs the data |
| touch a DOM node when it mounts         | a ref callback (React 19 ref callbacks can return a cleanup)                                                     |
| sync two pieces of state                | keep one; derive the other                                                                                       |

`useMemo`, `useCallback`, and `memo` are banned too: React Compiler memoizes every component and hook, and `make build`/`make acceptance` fail if it has to skip one (`panicThreshold: 'all_errors'`). The compiler's own diagnostics run at lint time through `eslint-plugin-react-hooks` (`purity`, `refs`, `immutability`, `set-state-in-render`, and the rest, all errors).

## Why these choices

- **Gates for poteto's lessons, not prose.** Her ordering is: make the mistake impossible in the codebase, then static analysis and CI, then rules and skills, and only last a human style guide. Every React rule here is a lint, type, architecture, build, or QA gate. `AGENTS.md` lists them in one line because the lint messages carry the instructions.
- **No comments (`local/no-comments`).** From her talk: agents copy comments and use them "as justification for why it wasn't going to solve the actual problem". Only lint directives (which must carry `-- reason`), `@ts-expect-error` (which `ban-ts-comment` requires a description for), and `prettier-ignore` are allowed. Root `*.mjs`/`*.cjs` config files are exempt. Every starter has this gate (ESLint here and in `node-nest-ts`, `scripts/no_comments.py` in Python, `cargo xtask comments` in Rust).
- **No `as` (`consistent-type-assertions: never`).** From pstack's `typescript-best-practices`: "every `as` is a runtime crash waiting". Brands come from verifying type guards (`isTaskId`, `isTaskTitle`), and external data goes through a schema (`zod/mini` in the repository, and for the coverage report in `scripts/crap.ts`). `as const` is still allowed.
- **Store outside React, read with `useSyncExternalStore`.** `TaskStore` keeps an immutable snapshot and preserves the identity of unchanged tasks, so the snapshot is stable between renders and memoized rows skip work. The application layer has no React import (`make arch` enforces it), so the store is tested without rendering.
- **React Compiler off under Vitest.** The compiler adds cache branches that v8 maps back to source lines, which made branch coverage and mutation testing measure compiler output instead of our code (UI files dropped to 83–89% branch coverage). Unit tests verify behavior on plain React; acceptance and QA run the compiled app, and the compiler's rules run at lint time.
- **Two tsconfigs.** `src/` is browser code (DOM lib, `vite/client` types, no Node types); everything else runs on Node. Node's type stripping runs the steps and QA scripts directly, so imports end in `.ts` (`allowImportingTsExtensions`) and `erasableSyntaxOnly` is on everywhere.
- **`vite.config.mjs`, not `.ts`.** With `skipLibCheck: false`, `babel-plugin-react-compiler`'s declarations fail to type-check against the mix of Babel 7 (the compiler) and Babel 8 (Stryker's instrumenter) types in the tree. `node-nest-ts` makes its Vitest config `.mjs` for the same reason.
- **Babel 7 for the compiler, not the Rust port.** `@vitejs/plugin-react` 6 offers `oxc-transform-react`, but its README marks it experimental.
- **`zod/mini`.** The functional API tree-shakes; switching from `zod` cut the bundle from 89 KB to 76 KB gzip. The bundle budget (80 KB) keeps it there; raise it in the Makefile only as a human decision.
- **Long animation frames in QA.** Her frame budget: "every frame that gets drawn has to be done in 16 milliseconds". Browsers only report frames over 50 ms as long animation frames, so step 9 of the QA procedure adds 100 tasks against the production build and fails if any frame took 50 ms or more.
- **Architecture rules beyond the layers.** `only-react-in-ui` keeps React out of domain, application, and infrastructure. `no-node-builtins-in-the-browser` is the web version of her Electron main/renderer import boundary. Specs may import infrastructure, so UI tests run against the real `localStorage` adapter instead of mocks ("don't mock what you can run").
- **Lint presets promoted to errors.** `@eslint-react` `strict-type-checked`, `react-hooks` `recommended-latest`, `jsx-a11y` `strict`, `@vitest/eslint-plugin`, and `testing-library/react` all ship some rules as `warn`; `asErrors` turns those into errors so nothing is a guideline. `@eslint-react`'s copies of the compiler rules are disabled in favor of `react-hooks`.
- **Acceptance in a real browser.** Steps start one Vite dev server and one Chromium per run, and a fresh browser context per scenario (so `localStorage` starts empty). They cannot import `src/`. A scenario fails if the browser logs any `console.error` or uncaught exception, which turns React's dev-mode warnings (missing keys, bad hook calls) into failures. `BeforeAll` loads the app once first, so a compiler bail-out or broken import fails the run in seconds with the browser's error, instead of timing out scenario by scenario.
- **Dependency hygiene.** pnpm 12's minimum release age is respected rather than excluded (Vite is pinned to 8.3.7 because 8.3.8 was hours old), and `allowBuilds` approves no install scripts.
- **`make setup` only sets `core.hooksPath` when the starter is the repo root**, so running it inside the starters monorepo does not hijack the monorepo's hooks.
- **CI runs `make check qa`**, not only `check`: the production build (compiler bail-outs, bundle budget) and the frame check only run in `qa`.
