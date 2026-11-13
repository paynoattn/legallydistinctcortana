# 033. Notification delivery policy: quiet hours, focus, batching

| Milestone    | Size | Depends on | Labels                    |
| ------------ | ---- | ---------- | ------------------------- |
| M3 Reminders | M    | #011       | area/notify, type/feature |

## Story

As a knowledge worker, I want the assistant to decide if, when, where, and how to tell me something, respecting my quiet hours, Windows Focus, and presentations, so that it is proactive without being noisy.

## Context

"Proactive, not chatty" is principle 3 of the [vision](../product/vision.md#principles). The delivery policy belongs to the `notify` context ([assistant: proactive engine](../architecture/assistant.md#proactive-engine)): quiet hours, Windows Focus state, full-screen apps and presentations (defer), a batching window (default 5 min), channels (toast, voice, overlay, Today only), and cross-device dedupe. Cross-device dedupe is #071. This issue builds the single-device policy and the port #071 plugs into.

## Scope

- `notify-domain`:
  - `NotificationRequest { id, source, priority, title, body, actions, channels_wanted, dedupe_key, not_before, expires_at }`.
  - `source` is one of `Reminder`, `CalendarHeadsUp`, `MailDigest`, `Feed`, `Timer`, `Routine`, `System`. `priority` is one of `Critical`, `High`, `Normal`, `Low`.
- A pure `decide(request, context) -> Decision`, where `context` holds the policy, the focus state, the clock, the pending batch, and the device role. `Decision` is one of:
  - `DeliverNow(channels)`
  - `Defer(until)`
  - `Batch(window_id)`
  - `TodayOnly`
  - `Drop(reason)` (expired, or duplicate `dedupe_key`)
- Rules, all editable in settings:
  - Quiet hours (off by default; when on, default 22:00–07:00) defer `Normal` and `Low` items to the end of the window. `High` items defer too, unless the source is marked "may break through". `Critical` items (timers, alarms the user set) always deliver.
  - Windows Focus / Do Not Disturb, presentation mode, and full-screen D3D apps defer everything below `Critical` until the state clears, then deliver what has not expired.
  - Batching: `Low` and `Normal` items from `MailDigest`, `Feed`, and `Routine` within 5 min merge into one notification. Reminders and heads-ups are never batched.
  - Expiry: a calendar heads-up expires at event start + 5 min. A deferred one that is still valid is re-worded by its source ("started 2 min ago").
- `notify-application`:
  - Subscribes to `ReminderDue`, `HeadsUpDue`, `MailDigestReady`, and the other source events.
  - Persists pending, deferred, and batched items so they survive restart.
  - Calls channel ports `ToastChannel`, `VoiceChannel`, `OverlayChannel`, and `TodayChannel`, and publishes `NotificationDelivered` and `NotificationDeferred`.
  - Uses a `DeliveryTarget` port whose default adapter answers "this device". #071 replaces it.
- Settings UI: the Notifications page in `src/settings/`. It covers quiet hours, per-source channels and break-through, the batching window, and a "Why was this deferred?" log of the last 50 decisions.

Out of scope: toast buttons and activation (#034), spoken alerts (#035), and cross-device single delivery (#071). Until #034 lands, the composition root wires a plain toast without buttons.

## Acceptance criteria

```gherkin
Feature: Delivery policy

  Scenario: Reminder during quiet hours is deferred to the end of the window
    Given quiet hours are 22:00 to 07:00
    When a Normal reminder is due at 23:15
    Then it is deferred until 07:00
    And at 07:00 it is delivered as a toast

  Scenario: Timer breaks through quiet hours
    Given quiet hours are 22:00 to 07:00
    When a Critical timer finishes at 23:15
    Then it is delivered immediately with sound

  Scenario: Presentation mode defers, then delivers on exit
    Given Windows reports presentation mode
    When a High reminder is due at 14:00
    Then it is deferred
    When presentation mode ends at 14:20
    Then it is delivered with the label "Due at 2:00 PM"

  Scenario: Heads-up that expired while deferred is dropped
    Given Focus is on from 09:50 to 10:30
    And a heads-up for a 10:00 meeting is requested at 09:50
    When Focus ends at 10:30
    Then the heads-up is dropped as expired and appears on Today only

  Scenario: Batching merges digest and feed items
    Given the batching window is 5 minutes
    When a mail digest and two breaking feed items are requested within 3 minutes
    Then one notification is delivered at the end of the window listing three items

  Scenario: Pending items survive restart
    Given a reminder is deferred until 07:00
    When the app restarts at 06:30
    Then the reminder is still delivered at 07:00

  Scenario: Same dedupe key is delivered once
    When two requests with the same dedupe_key arrive within 1 minute
    Then only the first is delivered
```

## Design notes

- **Crates:** create `notify-domain` and `notify-application`. Adapters live in `desktop` wiring and `platform-windows`: the `FocusState` port uses `SHQueryUserNotificationState` (presentation, full-screen, quiet time). Detecting Windows 11 Focus sessions needs verification; if there is no public API, use what `SHQueryUserNotificationState` reports (`QUNS_QUIET_TIME`, `QUNS_BUSY`).
- **arch.toml:** `notify-domain` allows `kernel` only. `notify-application` allows `notify-domain` and `kernel`, plus other contexts' domain crates only for event payload types.
- **Migrations:** `notify_pending` (device-local) and `notify_decisions` (a ring of 500 rows for the "why" log; titles redacted to the source type).
- **Privacy:** notification bodies carry other contexts' data classes (Mail, Calendar). The policy never sends them off-device. Toast text lands in the Windows notification store, which is outside our encryption. #034 adds the "private toasts" setting.
- **Testing:** `decide` is property-tested. A Critical item is always `DeliverNow`; every decision for an unexpired item ends in delivery or `TodayOnly` within quiet-hours length + batch window; and no request is delivered twice. Acceptance is `features/core/delivery-policy.feature` with fake focus and clock.
- **Budget:** a decision takes under 1 ms, with no I/O on the decision path.

## QA procedure

1. Settings → Notifications: set quiet hours to start 2 minutes from now and end 5 minutes from now. Create a reminder due in 3 minutes. Observe no toast at the due time and a toast when quiet hours end.
2. Start a timer for 1 minute during quiet hours. Observe that it fires on time with sound.
3. Open PowerPoint, start a slideshow, and let a reminder come due. Observe no toast during the slideshow and the toast within 5 s of pressing Esc.
4. Turn on Windows Do Not Disturb (Notification Center → bell icon). Observe deferral, then delivery when it is turned off.
5. Open the "Why was this deferred?" log. Observe one entry per decision above, with reasons.
6. With a reminder deferred, quit from the tray and restart the app. Observe that it is still delivered at the end of the window.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
