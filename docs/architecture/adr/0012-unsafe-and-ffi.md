# 0012. `unsafe` and FFI

Status: proposed (needs human approval: it changes protected gate files)

## Context

The starter forbids `unsafe_code` workspace-wide and its comments gate (`cargo xtask comments`) rejects every non-doc comment. Clippy's `undocumented_unsafe_blocks` (also enabled) requires a `// SAFETY:` comment on every `unsafe` block. Together they make Win32 calls (hooks, UI Automation, DPAPI, `SendInput`), and the whisper.cpp and ONNX Runtime bindings, impossible to write, even though the product needs them.

## Decision

1. Workspace lint `unsafe_code` goes from `forbid` to `deny`. A crate may opt in only by putting `#![expect(unsafe_code, reason = "...")]` on **one FFI module**, never at the crate root.
2. A new `unsafe_allowed` list in `arch.toml` names the crates allowed to contain that `expect`: initially `platform-windows` and `infer-host`; the post-MVP `explorer-command` COM DLL (#058) joins when that work starts. `cargo xtask arch` fails if `unsafe` appears in any other crate.
3. `cargo xtask comments` accepts exactly one comment shape, `// SAFETY: <reason>`, placed directly above an `unsafe` block or `unsafe impl`, and only in crates on the `unsafe_allowed` list. Every other comment is still rejected.
4. `xtask arch` also gains a **direct-dependency** rule: only `platform-windows` (and later `explorer-command`) may list `windows` or `windows-sys` in its `Cargo.toml` (transitive use by `cpal`, `tauri`, `keyring` is fine).
5. FFI lives only in `src/ffi/` modules of the allowed crates, and stays thin: each function converts one raw call into a safe function returning `Result`, with no branching beyond error conversion. Logic around them lives in safe, tested code behind the platform ports. Because FFI code can only run on a real desktop, `src/ffi/` paths are excluded from coverage and mutation the same way composition roots are (per-function `#[coverage(off)]` is nightly-only). The thin-wrapper rule is checked in review and by clippy's complexity limits, and the code is exercised by `make qa` on Windows.

## Consequences

- `Cargo.toml`, `arch.toml`, and `xtask/**` are protected, so this needs human approval, then issue [#092](../../issues/092-unsafe-and-ffi-policy.md) implements it before #006 and #012.
- Unsafe code is visible in review: one module per crate, two crates in total, each block justified.
