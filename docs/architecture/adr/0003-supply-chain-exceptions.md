# 0003. cargo-deny exceptions required by Tauri

Status: accepted (2026-11-14, at repository setup)

## Context

The `rust-axum` starter's `deny.toml` denies duplicate crate versions, allows only Apache-2.0, BSD-3-Clause, MIT, and Unicode-3.0, and fails on any unmaintained advisory. Adding Tauri 2 broke all three. Its tree carries:

- duplicate versions (gtk3-rs, Servo CSS crates, `windows-sys`, `phf`)
- MPL-2.0 and Zlib crates
- the unmaintained `proc-macro-error`, pulled in through gtk 0.18 on Linux

## Decision

Narrow exceptions only, each with a reason in `deny.toml`:

- `bans.skip-tree` for `tauri@2` and `tauri-build@2`: duplicates _inside_ Tauri's tree are ignored, and duplicates we introduce still fail.
- The license allow-list adds **MPL-2.0** (file-level copyleft; we do not modify those files) and **Zlib**. Only `target-lexicon` (build-time) may use `Apache-2.0 WITH LLVM-exception`.
- `clipboard-win` and `error-code` alone may use **BSL-1.0** (the Boost license: permissive, no attribution needed in binaries). They are `arboard`'s Windows clipboard backend, used by read-aloud selection capture. Added 2026-11-14.
- The advisory ignore covers **RUSTSEC-2024-0370** (`proc-macro-error`, Linux-only, via gtk 0.18).
- `libloading` alone may use **ISC** (MIT-equivalent). It is the Linux tray backend, through `libappindicator-sys` ← `tray-icon` ← Tauri's `tray-icon` feature. Added 2026-11-18 with the tray icon.
- **RUSTSEC-2026-0370, -0371, -0372** (`zvariant`, `zbus_names`, `zvariant_derive` are now part of `zbus` itself and no longer maintained on their own) are ignored. They are Linux-only, pulled by `zbus` ← `tauri-plugin-single-instance`, which uses D-Bus there. No plugin release drops them yet. Added 2026-11-18.
- **RUSTSEC-2026-0542** (`humantime`, unmaintained because it wraps `chrono`) is ignored. It comes in only as a dev-dependency through `cucumber` 0.23, the latest release, so it never ships. Added 2026-11-18.

## Consequences

- Re-check on every Tauri minor upgrade and drop exceptions that are no longer needed (cargo-deny warns about unused skips).
- Any new exception requires an update to this ADR and human approval (`deny.toml` is in `.gates-protected`).
