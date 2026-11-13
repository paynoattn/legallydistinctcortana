# Gate contract

> In this repo every target runs a Rust half and a UI half; append `-rs` or `-ui` to run one (`make check-rs`, `make mutate-ui SCOPE=changed`). `make bundle` builds the UI and enforces the bundle budget; `make build` produces the installer.

Every starter exposes the same `make` targets, so the skills in `shared/skills` work unchanged across Node/Nest, Rust, Python/FastAPI, and React. Gates exit non-zero on failure and print what to fix.

| Target            | What it does                                                                                                               | Default threshold                                                          |
| ----------------- | -------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| `make setup`      | Install toolchain deps and point git at `.githooks`                                                                        |                                                                            |
| `make fmt`        | Rewrite formatting                                                                                                         |                                                                            |
| `make fmt-check`  | Fail on unformatted files                                                                                                  |                                                                            |
| `make lint`       | Strict lint, zero warnings; no comments except tool directives (and doc comments in Rust); no unchecked casts              | complexity ≤ 6                                                             |
| `make typecheck`  | Compiler / type checker in strictest mode                                                                                  |                                                                            |
| `make test`       | Unit tests                                                                                                                 |                                                                            |
| `make acceptance` | Gherkin features in `features/`                                                                                            |                                                                            |
| `make coverage`   | Unit tests with coverage thresholds                                                                                        | 90% lines/branches                                                         |
| `make crap`       | Per-function CRAP score from coverage + complexity                                                                         | CRAP ≤ 6                                                                   |
| `make arch`       | Dependency rules                                                                                                           | zero violations                                                            |
| `make mutate`     | Mutation testing                                                                                                           | score 100%; equivalent mutants are excluded by a human in protected config |
| `make qa`         | Boot the app, run `qa/scripts/*` against it (React: build, enforce the bundle budget, serve `dist/`, drive it in Chromium) | all pass                                                                   |
| `make check`      | `fmt-check lint typecheck arch coverage crap acceptance`                                                                   |                                                                            |
| `make gauntlet`   | `check mutate qa`                                                                                                          |                                                                            |

`make check` is what the stop hook, the pre-push hook, and CI run (`react-vite-ts` CI also runs `qa`, because the production build and the frame budget are only checked there). `make gauntlet` is what the hardener and QA stages finish with.

## Changing a threshold

Thresholds live in the files listed in each starter's `.gates-protected`. Agents are asked for human approval before editing those files. Lowering a threshold is a human decision, recorded in the commit message with the reason.
