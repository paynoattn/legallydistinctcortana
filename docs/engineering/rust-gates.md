> Copied from the starter this repo was built from. In this repo: gates run through the combined root `Makefile` with `-rs` and `-ui` halves (`make lint-rs`, `make coverage-ui`, ...); Rust features live in `features/core/`, UI features in `features/ui/`; QA scripts in `qa/scripts/{core,ui}/`; the scripts are `scripts/{mutate,qa}-{rs,ui}.sh`; the Tauri shell is `crates/desktop`. See `AGENTS.md`.

# rust-axum starter

A small Rust HTTP service (an in-memory task list) built in layers, with deterministic quality gates
that agents and humans must pass: zero-warning lints, coverage, CRAP, architecture rules, mutation
testing, Gherkin acceptance tests, and a black-box QA run.

## Setup

Prerequisites: [rustup](https://rustup.rs), GNU make, git, curl, jq.

```sh
make setup   # installs the pinned toolchain and cargo tools, points git at .githooks
make check   # the gate agents must pass before they stop
cargo run -p app   # serves on http://127.0.0.1:3000 (PORT and HOST env vars override)
```

`make setup` installs, if missing: the toolchain in `rust-toolchain.toml` (with rustfmt, clippy,
llvm-tools-preview), then `cargo-llvm-cov`, `cargo-mutants`, and `cargo-deny` at the versions in
the Makefile's `CARGO_TOOLS`. The first `cargo install` takes a few minutes. It only sets
`core.hooksPath` when this directory is the root of its git repository, so running it inside the
starters monorepo does not change the monorepo's hooks.

## Targets

| Target                   | What it runs                                                                                                | Threshold                                                                    |
| ------------------------ | ----------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| `make fmt` / `fmt-check` | `cargo fmt` (rewrite / check)                                                                               |                                                                              |
| `make lint`              | `cargo clippy --all-targets --all-features -- -D warnings`, `cargo xtask comments`, then `cargo deny check` | zero warnings; cognitive complexity ≤ 6; no comments other than doc comments |
| `make typecheck`         | `cargo check --all-targets`, then `cargo doc` with `-D warnings` (broken doc links fail)                    |                                                                              |
| `make test`              | unit tests (`cargo test --lib --bins`), including xtask's own tests                                         |                                                                              |
| `make acceptance`        | `features/*.feature` via cucumber against the composed app, in process                                      | all scenarios pass                                                           |
| `make coverage`          | `cargo llvm-cov` over unit tests, writes `target/lcov.info`                                                 | `COVERAGE_MIN=90` for lines, regions, functions                              |
| `make crap`              | `coverage`, then `cargo xtask crap`                                                                         | `CRAP_MAX=6`                                                                 |
| `make arch`              | `cargo xtask arch` against `arch.toml`                                                                      | zero violations                                                              |
| `make mutate`            | `cargo mutants`, then `cargo xtask mutation-score`                                                          | `MUTATION_MIN=100`                                                           |
| `make qa`                | builds and boots `app` on a free port, runs `qa/scripts/*`, sends SIGTERM                                   | all pass, clean shutdown                                                     |
| `make check`             | `fmt-check lint typecheck arch coverage crap acceptance`                                                    |                                                                              |
| `make gauntlet`          | `check mutate qa`                                                                                           |                                                                              |

`make mutate SCOPE=changed` only mutates code changed since `BASE` (default `origin/main`),
including uncommitted and untracked `.rs` files. If `BASE` does not exist (no remote, shallow
clone) it falls back to the full run. A full run takes under a minute today; it grows with the code.

## Layout

```
crates/
  tasks-domain/          Task, TaskId, TaskTitle::parse (trimmed, 1-200 chars). Pure: std + thiserror.
  tasks-application/     TaskService use cases; ports: TaskRepository, IdGenerator.
  tasks-infrastructure/  InMemoryTaskRepository (RwLock<Vec>), UuidGenerator (v4).
  http-api/              axum Router, JSON DTOs, error -> status mapping, /health.
  app/                   composition root (lib.rs), binary (main.rs), tests/acceptance.rs.
xtask/                   repo tooling: crap, arch, comments, mutation-score, free-port.
features/                Gherkin specs.          qa/procedures/  human QA steps.
qa/scripts/              executable QA (bash + curl + jq; reads QA_BASE_URL).
scripts/                 mutate.sh and qa.sh (called by make), agent-hooks/ (synced).
```

Dependency direction: `http-api → application → domain`, `infrastructure → application, domain`,
and only `app` depends on `infrastructure`. Crate boundaries make wrong-way imports a compile error
once a dependency is missing from `Cargo.toml`; `arch.toml` makes the intended graph explicit so
adding the wrong dependency fails `make arch`.

## Where each config lives, and why

All of these are listed in `.gates-protected`; agents must ask before editing them.

**`rust-toolchain.toml`**: pins the exact compiler (`1.99.1`), not just `stable`. Clippy adds lints
in every release, so an unpinned toolchain can turn a green build red overnight. Bump it
deliberately and fix the new findings in the same commit. `rustup` reads this file automatically.

**`Cargo.toml` (root)**: the workspace. Holds shared versions (`[workspace.dependencies]`) and the
lint policy (`[workspace.lints]`). Each crate opts in with `[lints] workspace = true`.

- `resolver = "3"` is the edition-2024 resolver; it respects each dependency's `rust-version`.
- **rustc lints at `warn`, clippy at `deny`.** `make lint` passes `-D warnings`, so every warning
  fails the gate anyway. rustc lints stay at warn because they also fire during plain builds,
  including the per-mutant builds in `cargo mutants`; a deny there would turn mutants that only
  leave an unused variable into "unviable" and hide real survivors. `missing_docs` is warn for
  the same reason and applies to every crate, binaries included.
- `unsafe_code = "forbid"` cannot be overridden by any attribute in the code.
- `clippy::pedantic` and `clippy::cargo` are denied as groups (priority -1 so single lints can
  override them). `multiple_crate_versions` is allowed because duplicate transitive versions are
  not fixable from here; `cargo deny` checks them instead, with a reasoned skip list.
- Restriction lints (opt-in by design) picked for agent code: no `unwrap`/`expect`/`panic!`/
  `todo!`/indexing outside tests (errors must be handled), no `as` casts (use `From`/`TryFrom`),
  no `println!`, no `#[allow]` at all (`allow_attributes`): suppressions must be
  `#[expect(lint, reason = "...")]`, and an `#[expect]` that no longer suppresses anything fails
  the build (`unfulfilled_lint_expectations`). That is how "suppressions need a reason, and
  unused ones fail" is enforced. All lints asked for in the starter brief are on; none were dropped.
- `cognitive_complexity` is a nursery lint (heuristic, may change). It counts code produced by
  macros, which is why axum's default `tower-log` feature is off: it makes each `tracing::info!`
  call score 8. If a macro you add trips the lint, prefer splitting the function.
- `[profile.mutants]` builds with no debug info, used only by cargo-mutants.

**`clippy.toml`**: thresholds. Cognitive complexity 6 (the backstop for CRAP), 40 lines per
function, 3 arguments (group the rest into a struct). The `allow-*-in-tests` keys let
`#[cfg(test)]` code use `unwrap`, `expect`, `panic!`, indexing, and printing. The acceptance
runner in `crates/app/tests` is not `cfg(test)` (it is a `harness = false` binary), so it follows
the production rules; it carries one reasoned `#[expect]` for cucumber's owned-argument steps.

**`rustfmt.toml`**: stable options only (`max_width = 100`, Unix newlines, `use_field_init_shorthand`,
`use_try_shorthand`, `reorder_imports`, style edition 2024). Useful nightly-only options, if you
switch to a nightly rustfmt: `imports_granularity = "Module"`, `group_imports = "StdExternalCrate"`,
`wrap_comments = true`, `format_code_in_doc_comments = true`.

**`deny.toml`** (cargo-deny): fails on RustSec advisories (vulnerable, unmaintained, yanked),
licenses outside a permissive allowlist, duplicate crate versions (one reasoned skip: `syn` 2,
while the ecosystem moves to `syn` 3), wildcard versions, and any registry or git source other
than crates.io. The advisory check downloads the RustSec database, so `make lint` needs network.

**`arch.toml`**: per crate, the workspace crates it may depend on (`allow`) and external crates it
must not use (`ban`, checked directly and transitively). For example `tasks-domain` bans `axum`,
`tokio`, `serde_json`, `uuid`; `tasks-application` bans HTTP and runtime crates. Nothing may depend
on `app` or `xtask`, and every workspace crate must be declared. Fix violations structurally (see
the `architecture-guard` skill).

**`.cargo/config.toml`**: the `cargo xtask` alias. The [xtask pattern](https://github.com/matklad/cargo-xtask)
keeps repo tooling in Rust, so it needs no Python or Node and passes the same lints as product code.

**`.cargo/mutants.toml`**: cargo-mutants settings. Excludes `xtask/` (tooling) and
`crates/app/src/main.rs` (env, bind, and signal wiring with no logic; `make qa` covers it).

**`Makefile`, `scripts/mutate.sh`, `scripts/qa.sh`**: the gate contract. Tool versions are pinned
in `CARGO_TOOLS`.

## How the custom gates work

**CRAP** (`xtask/src/crap.rs`, `xtask/src/complexity.rs`). For every function in `crates/*/src`,
`syn` parses the file and counts cyclomatic complexity: 1, plus 1 per `if`/`else if`, `while`,
`for`, `&&`, `||`, `let ... else`, match arm after the first, and match guard. `?` and `loop` are
not counted, and neither is code inside macro calls (`syn` cannot see into them). Closures count
toward their enclosing function. Test code is skipped. Coverage is the share of the function's
instrumented lines with hits in `target/lcov.info`. A function with no coverage data (for example
in `main.rs`, which coverage ignores) counts as 0% covered, so excluded files stay simple.
`CRAP = c² × (1 − cov)³ + c`; a fully covered function scores its complexity, so CRAP ≤ 6 also caps
complexity at 6.

The same command fails any function named `new` that has a branch. cargo-mutants never mutates
functions named `new` (hard-coded, no opt-out), so logic in a `new` would escape `make mutate`.
Keep `new` as plain construction and put validation in a named constructor, as
`TaskTitle::parse` does.

**Coverage.** Stable Rust has line, region, and function coverage; branch coverage
(`-Z coverage-options=branch`) is nightly-only. Regions are finer-grained than lines (each side of
an `if`, each match arm, each `&&` operand is a region), so `--fail-under-regions 90` stands in for
the brief's branch threshold. `llvm-cov` also counts the in-module `#[cfg(test)]` code, which
raises the totals; CRAP and mutation testing are the stricter checks per function.

**Mutation score.** cargo-mutants fails on any surviving mutant and has no percentage threshold,
so `scripts/mutate.sh` accepts its "some missed" exit codes and `cargo xtask mutation-score` reads
`mutants.out/outcomes.json`: score = caught / (caught + missed + timeout). Unviable mutants (ones
that do not compile, such as returning `Default::default()` for a type without `Default`) are left
out; timeouts count as survivors. Survivors are listed from `mutants.out/missed.txt`. Each mutant
runs the tests of the crate it lives in, so unit tests in that crate must kill it. The minimum is
100%: any survivor fails. A genuinely equivalent mutant is excluded by a human in
`.cargo/mutants.toml` (protected), not tolerated by a lower score.

**Comments** (`xtask/src/comments.rs`). Fails on any `//` or `/* */` comment in `crates/` or
`xtask/`. Agents copy comments and use them to justify workarounds (see `docs/poteto-react.md`
in the starters repo). Doc comments (`///`, `//!`) stay because `missing_docs` requires them.
clippy has no such lint, so the xtask tokenizes each file with `proc_macro2`, which keeps doc
comments as `#[doc]` tokens with their source spans and drops every other comment; any
non-whitespace text outside all token spans is a comment. Strings, raw strings, and char
literals are tokens, so `"// x"` never counts. The no-casts half of the rule was already here:
`clippy::as_conversions` is denied.

**Architecture.** `cargo xtask arch` reads `cargo metadata` and checks every dependency edge
(normal, dev, and build) against `arch.toml`. Transitive bans use Cargo's workspace-wide feature
resolution, so they can over-report but never miss.

## Design notes

- Ports are synchronous `dyn` traits, which is enough for in-memory storage and simple to read.
  When you add a real database, make them async (`async fn` in traits with a `Send` bound, or the
  `async-trait` crate for `dyn` use) and keep the adapters in `tasks-infrastructure`.
- Repository errors map to `500 {"error":"internal server error"}` and are logged; the cause is not
  sent to clients. Invalid titles and malformed JSON bodies map to `400`, unknown ids to `404`.
- The acceptance runner lives in `crates/app/tests/` rather than in `http-api`, so it tests the real
  composition (the same `app::router()` the binary serves) and `http-api` never needs
  `tasks-infrastructure`, not even as a dev-dependency.
- `HOST` defaults to `127.0.0.1`. Set `HOST=0.0.0.0` in a container.
