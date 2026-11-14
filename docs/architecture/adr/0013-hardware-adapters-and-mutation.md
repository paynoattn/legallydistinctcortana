# 0013. Hardware-bound adapter modules named `os` are excluded from mutation testing

Status: accepted (2026-11-14, by the repository owner)

## Context

`make mutate` requires every mutant to be killed by a test. Some adapter code only does something against real hardware: opening the default audio output device and holding its stream, reading and writing the system clipboard, and sending synthetic key presses. CI runners have no speakers, no desktop session, and no clipboard, and unit tests cannot stand in for them without replacing the very code under test. Mutants in that code survive no matter what tests are written (21 of them held the read-aloud branch at 91.9%), so the only ways to reach the gate were loosening the threshold or writing tests that assert nothing.

## Decision

1. Code that can only run against real devices goes in modules whose file is named `os.rs` (`crates/<crate>/src/os.rs` or `crates/<crate>/src/<module>/os.rs`). `.cargo/mutants.toml` excludes exactly those files; no other path joins the list on these grounds.
2. `os` modules hold thin adapters only: each function makes one device or OS call and converts its result, with no decisions of its own. Every branch, buffer, retry, timeout, and format conversion lives in a sibling module that is mutation tested against fakes of the same port.
3. Coverage and CRAP gates still apply to `os` modules. An uncovered function's CRAP score equals complexity² + complexity, so under `CRAP_MAX = 6` each function stays at complexity ≤ 2. That keeps the thin-adapter rule mechanical rather than a review judgment.
4. `os` code is exercised on a real desktop by the manual procedures in `qa/procedures/` (for example [`read-aloud.md`](../../../qa/procedures/read-aloud.md)), since `make qa` runs the headless core without devices.

## Consequences

- The mutation gate stays at its threshold and measures only code a test can reach.
- Moving logic into an `os` module to dodge mutation is visible: the file name is the signal in review, and the CRAP gate fails once a function there grows past two branches.
- Bugs in the excluded adapters are caught by hardware QA, not CI, so they should stay small enough to read in one sitting.
- [ADR 0012](0012-unsafe-and-ffi.md) point 5 relies on this rule for its FFI modules.
