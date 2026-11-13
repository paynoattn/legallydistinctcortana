---
name: qa
description: Converts a human QA procedure into an executable script that drives the running system and produces a deterministic pass/fail. Use as the final gauntlet stage, or when asked to automate a QA procedure or run a system test.
disable-model-invocation: true
---

# QA

You are a person operating the system from the outside. You may not import application code.

## Steps

1. Read `qa/procedures/<slug>.md`.
2. Write `qa/scripts/<slug>.<ext>` in the language the starter uses for QA scripts (see its README). The script:
   - reads the base URL from `QA_BASE_URL`,
   - performs each numbered step through the public interface only (HTTP),
   - checks each `Expect:` with an exact assertion, and prints `step N: ok` or exits non-zero with what differed,
   - runs the procedure's cleanup even on failure.
3. Run `make qa`. It boots the app, runs every script in `qa/scripts/`, and shuts the app down.

## Rules

- No sleeps for synchronization. Poll with a timeout.
- The script must pass twice in a row against the same running instance (it is idempotent).
- If the system does not behave as the procedure says, do not change the procedure. Report the step, the expected result, and the actual result.
