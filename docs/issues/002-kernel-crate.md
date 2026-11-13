# 002. Kernel crate: ids, clock, HLC, data classes, privacy levels, mood

| Milestone      | Size | Depends on | Labels                        |
| -------------- | ---- | ---------- | ----------------------------- |
| M0 Foundations | S    | #001       | area/foundation, type/feature |

## Story

As a developer of any context, I want one small, dependency-free crate with the shared value types, so that every context agrees on identity, time, privacy, and mood without depending on each other.

## Context

The `kernel` row of [overview: bounded contexts](../architecture/overview.md#bounded-contexts-and-crates): IDs, the `Clock` port, the hybrid logical clock, `DataClass`, `PrivacyLevel`, `Mood`, and error conventions. HLC ordering is defined in [sync](../architecture/sync.md#replication-protocol). Data classes come from [assistant: privacy policy](../architecture/assistant.md#model-router-models-context). Moods come from [voice-and-speech: moods](../architecture/voice-and-speech.md#moods).

## Scope

- `crates/kernel` with no workspace dependencies and no I/O.
- **Ids:** a generic `Id<T>` newtype over 16 bytes with `Display`/`FromStr` (lowercase hex or ULID-style text; pick one and test round-trip), plus `DeviceId`. Ids are created through an `IdSource` port; the kernel never generates randomness itself. Recurrence and time zones are not here; they live in the separate `kernel-time` crate (#031).
- **Time:** `Timestamp` (UTC milliseconds since the Unix epoch) and the `Clock` port (`now() -> Timestamp`). A `FixedClock` test double behind a `testkit` feature.
- **HLC:** `Hlc { wall_ms, counter, device }` with total ordering by (wall, counter, device). `HlcClock::tick(now)` for local events and `HlcClock::observe(remote, now)` for received ops, following the standard HLC rules. Clock drift: a remote wall time more than 5 minutes ahead of local time is rejected with an error, not adopted.
- **Privacy:** `DataClass = Voice | Mail | Calendar | Reminders | Location | Files | Memory | Feeds | Screen | Projects | Public` and `PrivacyLevel`, the destination tier a piece of data may reach: `OnDevice < Lan < Cloud`. The router (#047) compares them; the kernel only defines and orders them.
- **Mood:** `Neutral | Calm | Cheerful | Alert | Urgent | Angry | Mad | Whisper | Deadpan`, with stable lowercase string ids for IPC and the browser extension.
- **Untrusted content:** `Untrusted<T>`, the wrapper ADR 0009 taint tracking keys on; only the quarantined summarizer and the renderer may unwrap it (#049).
- **Audit:** the `AuditSink` port (`record(OffDeviceCall { host, task, data_classes, bytes })`) that every adapter making an off-device call reports through; the `privacy` context implements it (#074, #091).
- **Redaction:** `Redacted<T>` whose `Debug` and `Display` print `[redacted]`, for transcripts, mail text, and secrets in logs (#009 relies on it).
- **Errors:** one convention: each crate has its own `thiserror` enum; the kernel provides only `ParseError` for its own types.

Out of scope: the event envelope (#011), the router's policy logic (#047), the op format (#069).

## Acceptance criteria

The kernel has no system boundary of its own, so its acceptance lives in unit and property tests. These scenarios are the specification the specifier turns into property tests, not a `.feature` file.

```gherkin
Feature: Kernel value types

  Scenario: HLC is monotonic on one device
    Given an HLC clock on device "a"
    When the wall clock goes backwards by 3 seconds between two ticks
    Then the second timestamp still orders after the first

  Scenario: HLC merges a remote timestamp
    Given a local HLC at wall 1000 counter 2
    When a remote HLC at wall 1000 counter 5 from device "b" is observed at local wall 1000
    Then the next local timestamp is wall 1000 counter 6

  Scenario: Far-future remote clocks are rejected
    Given a local wall time of T
    When a remote HLC with wall time T + 6 minutes is observed
    Then observe returns a clock-skew error and the local clock is unchanged

  Scenario: Mood ids round-trip
    Given every Mood variant
    When each is converted to its string id and parsed back
    Then the same variant is returned, and "sarcastic" fails to parse with a ParseError

  Scenario: Redacted values never print their content
    Given Redacted wrapping the text "call Sam at 555-0100"
    When it is formatted with Debug and with Display
    Then both outputs equal "[redacted]"
```

## Design notes

- `arch.toml`:
  ```toml
  [crates.kernel]
  allow = []
  ban = ["tokio", "reqwest", "rusqlite", "serde_json", "tauri", "windows", "uuid", "tracing"]
  ```
- Dependencies: `thiserror` only, plus optional `serde` (derive) behind a `serde` feature so IPC and op encoding can serialize kernel types without making `serde_json` reachable.
- Property tests (`proptest`, dev-dependency) for HLC ordering, merge, and id parsing.
- Every later `*-domain` crate gets `allow = ["kernel"]`.
- `Mood` follows the architecture's nine moods. The vision lists "sarcastic" instead of Mad and Deadpan; the architecture wins until a human decides otherwise.

## QA procedure

1. Run `cargo test -p kernel`. All unit and property tests pass.
2. Run `make arch`. `kernel` has no workspace dependencies and none of the banned crates.
3. Run `cargo tree -p kernel -e normal`. Only `thiserror` (and `serde` when the feature is on) appear.

## Definition of done

- [ ] Acceptance scenarios (here: kernel unit and property tests) pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
- [ ] `arch.toml` has the `kernel` entry (protected file, human-approved)
