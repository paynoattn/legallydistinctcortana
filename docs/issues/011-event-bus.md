# 011. Domain event bus and UI subscription stores

| Milestone      | Size | Depends on | Labels                                  |
| -------------- | ---- | ---------- | --------------------------------------- |
| M0 Foundations | S    | #002, #008 | area/foundation, area/ipc, type/feature |

## Story

As a developer, I want contexts to announce what happened on one typed bus, and the UI to read a filtered subset through stores, so that contexts stay decoupled and windows update without polling.

## Context

[Overview: cross-context communication](../architecture/overview.md#cross-context-communication): domain events (`ReminderDue`, `MailDigestReady`, `DictationFinished`) go on an in-process `tokio::sync::broadcast` bus with a typed envelope defined in `kernel`. The desktop forwards a filtered subset to webviews as Tauri events. UI stores expose `subscribe`/`getSnapshot` for `useSyncExternalStore`; `useEffect` is banned ([UI: IPC](../architecture/ui.md#ipc)).

## Scope

- `kernel`: `EventEnvelope<E> { id, occurred_at: Timestamp, source: ContextName, data_class: DataClass, event: E }` and an `EventPublisher<E>` port.
- `event-bus` infrastructure crate: a typed broadcast bus per event family, publish never blocks the publisher, a bounded capacity per channel (default 1024), and a lag policy: a subscriber that falls behind gets a `Lagged(n)` signal and logs it; it does not crash.
- Desktop forwarding: a per-window allow-list of event kinds. Forwarded payloads are UI DTOs, not domain types. The overlay receives only voice and speech state and levels (no text). The main window receives what its features subscribe to.
- Level events are throttled at the source to 30 Hz.
- UI: `src/shared/events/` with an `EventSource` port, a Tauri adapter on top of the generated event bindings, a fake adapter for acceptance (`window.__ipcFake.emit(kind, payload)`), and a small `createEventStore(reducer, initial)` helper that returns `{ subscribe, getSnapshot }`.
- One end-to-end event to prove the path: `CoreHeartbeat { uptime_s }` every 10 s, shown as "Core running" in About.

Out of scope: cross-device events (#069, #071), persistence of events (events are not stored), specific feature events (each feature issue).

## Acceptance criteria

`features/core/event-bus.feature` (in process) and `features/ui/events.feature` (fake IPC):

```gherkin
Feature: Events

  Scenario: Every subscriber receives a published event
    Given two subscribers to the heartbeat family
    When one heartbeat is published
    Then both receive it with the same id and timestamp

  Scenario: A slow subscriber does not block publishers
    Given a subscriber that never reads
    When 5000 events are published
    Then publishing completes without waiting
    And the slow subscriber reports Lagged with a count greater than 0

  Scenario: The overlay never receives text-bearing events
    Given a DictationFinished event that carries the transcript
    When the desktop forwards events
    Then the main window receives a DTO without the transcript text
    And the overlay receives nothing for that event

  Scenario: The UI reflects events without effects
    Given the main window is open with "?ipc=fake"
    When the fake emits CoreHeartbeat with uptime 42
    Then About shows "Core running · 42 s"

  Scenario: Malformed events are dropped at the boundary
    When the fake emits CoreHeartbeat with uptime "soon"
    Then About still shows the previous value
    And a parse error is counted in the diagnostics store
```

## Design notes

- `arch.toml`: `event-bus` allow `["kernel"]`, ban `tauri`, `windows`, `rusqlite`. Application crates depend on the `EventPublisher` port in `kernel`, never on `event-bus`, so they stay free of `tokio`.
- `DictationFinished` and similar events may carry user content for in-process subscribers (for example dictation history). The forwarding filter, not the publisher, decides what reaches a webview. Test the filter table directly.
- dependency-cruiser: `src/shared/events` may be imported only by `infrastructure` layers and composition roots.
- Ordering: events within one family are delivered in publish order; no ordering across families is promised.
- Performance: publish under 10 µs; forwarding at most one Tauri emit per event per window.

## QA procedure

1. Run `make dev` and open About. "Core running" appears and the seconds increase every 10 s.
2. Open the overlay's DevTools (debug build) and listen to all events. Only voice and speech kinds arrive.
3. Close and reopen the main window from the tray. The heartbeat resumes without a restart.
4. Check `desktop.log` after an hour idle. There are no Lagged warnings.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
