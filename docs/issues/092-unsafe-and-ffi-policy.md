# 092. Unsafe and FFI policy gate (ADR 0012)

| Milestone      | Size | Depends on | Labels                                     |
| -------------- | ---- | ---------- | ------------------------------------------ |
| M0 Foundations | S    | #001       | area/foundation, area/security, type/chore |

## Story

As a maintainer, I want `unsafe` code allowed only in two named crates, only in thin `src/ffi/` modules, and with every block justified, so that the product can call Win32, whisper.cpp, and ONNX Runtime without letting unsafe code spread.

## Context

[ADR 0012](../architecture/adr/0012-unsafe-and-ffi.md): the starter forbids `unsafe_code`, and the comments gate rejects the `// SAFETY:` comments that clippy's `undocumented_unsafe_blocks` requires, so no FFI can be written today. [Overview: dependency rules](../architecture/overview.md#dependency-rules-added-to-archtoml-as-crates-land): only `platform-windows` depends directly on `windows`; `unsafe` only in the ADR's allowlist (`platform-windows`, `infer-host`). #006 and #012 are blocked on this issue.

## Scope

- `Cargo.toml`: `[workspace.lints.rust] unsafe_code = "deny"` (was `forbid`), so one module can opt in with `#![expect(unsafe_code, reason = "...")]`.
- `arch.toml`: a top-level `unsafe_allowed = ["platform-windows", "infer-host"]`, and a direct-dependency table `direct_only = { windows = ["platform-windows"], windows-sys = ["platform-windows"] }`.
- `cargo xtask arch`:
  - Fails if any crate not in `unsafe_allowed` contains the `unsafe` keyword (blocks, functions, impls, traits, extern blocks), found by tokenizing the sources as the comments gate does.
  - In allowed crates, fails if `unsafe` appears outside `src/ffi/`, if `expect(unsafe_code)` sits at the crate root, or if more than one module carries it.
  - Fails if a crate lists `windows` or `windows-sys` as a direct dependency (normal, dev, or build) without being named in `direct_only`. Transitive use through `cpal`, `tauri`, or `keyring` is fine.
- `cargo xtask comments`: accepts exactly `// SAFETY: <non-empty reason>` on the line directly above an `unsafe` block or `unsafe impl`, only in `src/ffi/` of an `unsafe_allowed` crate. Every other comment is still rejected, including a `SAFETY:` comment anywhere else.
- `Makefile` (`COVERAGE_IGNORE`) and `.cargo/mutants.toml` (`exclude_globs`): exclude `crates/*/src/ffi/**` from coverage and mutation, as composition roots are.
- `docs/engineering/rust-gates.md`: a short "FFI" section with the thin-wrapper rule.

Out of scope: the first FFI modules themselves (#006 DPAPI, #012 hotkeys), fuzzing (#078).

## Acceptance criteria

The gates are tested by xtask's own tests against small fixture workspaces in `xtask/tests/fixtures/`, and checked once by hand on the real tree.

```gherkin
Feature: Unsafe and FFI policy

  Scenario: Unsafe in a crate that is not allowed
    Given a fixture crate "reminders-domain" containing an unsafe block
    When "cargo xtask arch" runs
    Then it fails naming reminders-domain, the file, and the line

  Scenario: Unsafe outside src/ffi in an allowed crate
    Given platform-windows has an unsafe block in src/hotkeys.rs
    When "cargo xtask arch" runs
    Then it fails with "unsafe is allowed only in src/ffi/"

  Scenario: Crate-wide opt-in is refused
    Given platform-windows has #![expect(unsafe_code)] in src/lib.rs
    When "cargo xtask arch" runs
    Then it fails with "opt in on the ffi module, not the crate root"

  Scenario: A justified unsafe block passes
    Given platform-windows/src/ffi/mod.rs opts in and each unsafe block has "// SAFETY: <reason>" directly above it
    When "make lint-rs" and "make arch" run
    Then both pass

  Scenario: SAFETY comments elsewhere are still comments
    Given a "// SAFETY: fine" comment above a safe function in voice-domain
    When "cargo xtask comments" runs
    Then it fails and lists the line

  Scenario: Empty SAFETY reason
    Given "// SAFETY:" with no text above an unsafe block in platform-windows/src/ffi
    When "cargo xtask comments" runs
    Then it fails with "SAFETY comment needs a reason"

  Scenario: Direct windows dependency outside platform-windows
    Given voice-infrastructure lists "windows" in its Cargo.toml
    When "cargo xtask arch" runs
    Then it fails with "only platform-windows may depend directly on windows"
    And a crate that gets windows only through cpal passes

  Scenario: FFI code is excluded from coverage and mutation
    Given platform-windows/src/ffi/hook.rs has no unit tests
    When "make coverage-rs" and "make mutate-rs" run
    Then the file is not counted against the thresholds
```

## Design notes

- Every touched file is protected (`Cargo.toml`, `arch.toml`, `xtask/**`, `Makefile`, `.cargo/**`). The PR needs human approval, and ADR 0012 moves to `accepted` in the same PR.
- `xtask` is excluded from workspace coverage, so its new checks need direct unit tests plus the fixture tests above; keep each check function under CRAP 6.
- The thin-wrapper rule (one raw call per function, error conversion only) is not machine-checked beyond clippy's complexity limit. Reviewers check it; the `code-smells` and `architecture-guard` skills should mention it.
- Consider also denying `clippy::multiple_unsafe_ops_per_block` so each `SAFETY:` comment justifies one operation.
- No runtime behavior and no privacy data classes are involved.

## QA procedure

1. On a clean checkout with this change, run `make check`. It passes, with no crate yet containing `unsafe`.
2. Add `unsafe {}` to `crates/kernel/src/lib.rs` and run `make arch`. It fails naming kernel. Revert.
3. Create `crates/platform-windows/src/ffi/mod.rs` with the module-level `expect` and one `unsafe` block preceded by `// SAFETY: test`. Run `make lint-rs arch`. Both pass. Remove the comment. clippy fails on `undocumented_unsafe_blocks`.
4. Add `windows = "*"` to another crate's `Cargo.toml` and run `make arch`. It fails with the direct-dependency message. Revert.
5. Run `make coverage-rs` with an untested file under `src/ffi/`. The thresholds still pass.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed (`rust-gates.md`, ADR 0012 status)
