# 077. Performance budget bench job

| Milestone  | Size | Depends on | Labels                                  |
| ---------- | ---- | ---------- | --------------------------------------- |
| M8 Release | M    | #020, #026 | area/performance, area/ci, type/feature |

## Story

As a maintainer, I want the voice latency and idle-cost budgets measured automatically on reference hardware, with a failing job when a change breaks one, so that "fast" stays a property of the product and not a claim in the README.

## Context

Principle 5 in the [vision](../product/vision.md#principles) and the budget table in [voice-and-speech: budgets](../architecture/voice-and-speech.md#budgets), which says latency budgets are asserted in a separate `bench` job, not in `make check` ([overview: testing notes](../architecture/overview.md#quality-gates)). Text injection ([#020](020-text-injection.md)) and the speech queue ([#026](026-speech-queue.md)) are the last pieces of the paths it measures.

## Scope

- `make bench` and a `bench` CI job that measure, as p50 and p95 over at least 30 runs:

  | Path                                                   | Budget (p95)                                                   |
  | ------------------------------------------------------ | -------------------------------------------------------------- |
  | Hotkey down → overlay visible                          | 50 ms                                                          |
  | Hotkey down → first audio frame buffered               | 30 ms                                                          |
  | Release → text inserted (≤ 15 s utterance, no cleanup) | 500 ms GPU, 1.5 s CPU                                          |
  | Release → text inserted (local LLM cleanup)            | 1.2 s GPU                                                      |
  | "Read aloud" → first audio                             | 300 ms                                                         |
  | Today snapshot built                                   | 150 ms                                                         |
  | Idle, models unloaded, after 10 minutes                | < 1% CPU, < 300 MB RAM (all app processes, including WebView2) |

- Budgets live in `bench/budgets.toml`, added to `.gates-protected`; a change needs a human's approval.
- Results as JSON plus a Markdown summary artifact; a history file on a `bench-results` branch so trends are visible.
- Runs on a self-hosted Windows 11 runner with an RTX 4060 (GPU paths) and on a hosted Windows runner (CPU paths and idle).

Out of scope: wake-word false-accept rates (measured in spike #061 and #057's evaluation), NPU paths (#088), model quality metrics (ML track).

## Acceptance criteria

```gherkin
Feature: Performance budgets

  Scenario: All budgets met
    Given the reference runner and a build of main
    When "make bench" runs
    Then it reports p50 and p95 for every path in bench/budgets.toml
    And exits 0 when every p95 is within its budget

  Scenario: A regression fails the job
    Given a build where text insertion waits an extra 200 ms
    When "make bench" runs on the GPU runner
    Then it exits non-zero and names "release → text inserted (GPU)" with the measured p95 and the budget

  Scenario: Too few samples
    Given a path produced fewer than 30 valid samples
    When the report is built
    Then the job fails with "insufficient samples" for that path instead of passing

  Scenario: Idle cost includes every process
    Given the app has run for 10 minutes with models unloaded and no windows shown
    When idle cost is measured
    Then CPU and working-set memory are summed over the desktop, WebView2, and infer-host processes

  Scenario: Missing GPU
    Given the runner has no supported GPU
    When "make bench" runs
    Then GPU paths are reported as "skipped: no GPU" and CPU paths are still enforced
```

## Design notes

- **Measurement without a person:** a bench build feature (`--features bench`, never in release builds) adds a `FileAudioSource` adapter for `voice-application`'s capture port that plays a WAV in real time, and timestamps from `tracing` spans with a monotonic clock. Hotkeys are injected with `SendInput`; overlay visibility is observed with a WinEvent hook (`EVENT_OBJECT_SHOW`) on the overlay HWND; insertion is observed in a small target window the harness owns (its text-changed event).
- **Harness:** `xtask bench` (Rust) drives the `desktop` app and collects results; percentiles and budget comparison are pure functions in `xtask` with unit tests. `xtask/**` and the `Makefile` are protected: changes need a human's approval.
- **Runner security:** a self-hosted runner on a public repository must not run untrusted pull requests. The GPU job runs on pushes to `main`, release tags, and on a maintainer's label, never on fork PRs. The project needs that machine; this is a gap in the current CI plan.
- **Noise:** fixed power plan (High performance, on AC), warm-up runs discarded, models preloaded except in the idle test.
- **Idle cost risk:** hidden WebView2 windows (overlay, panel, confirm) are pre-created for latency and count toward 300 MB; if the budget fails, record the measured split per process to decide between lazy creation and the budget.

## QA procedure

1. On a Windows 11 laptop with an RTX 4060, plugged in, run `make bench`. Observe the summary table with p50/p95 per path and exit code 0.
2. Temporarily add a 200 ms sleep in the injection adapter (local branch), rerun, and observe the named failure and non-zero exit.
3. Run `make bench` on a machine without a discrete GPU; observe GPU paths skipped and CPU paths enforced.
4. Open Task Manager → Details during the idle test and compare the summed memory with the report.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check` (harness logic; the real measurements run in `bench`)
- [ ] `make mutate SCOPE=changed` at 100% for `xtask bench` logic
- [ ] QA procedure in `qa/procedures/` passes on the reference laptop
- [ ] gate-contract.md and voice-and-speech.md updated with `make bench` and the runner setup
