# 034. Actionable toasts (snooze, done, join)

| Milestone    | Size | Depends on | Labels                             |
| ------------ | ---- | ---------- | ---------------------------------- |
| M3 Reminders | M    | #033, #064 | area/notify, area/os, type/feature |

## Story

As a knowledge worker, I want reminder and meeting toasts with Snooze, Done, and Join buttons that work even when the app window is closed, so that I can act on them in one click.

## Context

MVP done-criterion 4 is a toast with Snooze and Done at 7 pm every Thursday ([mvp-plan](../product/mvp-plan.md#what-done-means-for-the-mvp)). The `Notifier` port is implemented with Windows App SDK app notifications, an AppUserModelID, a Start-menu shortcut, and COM activation ([os-integration](../architecture/os-integration.md)). Spike #064 decides the Tauri-compatible activation path. If the spike fails, the fallback is an in-app reminder card with the same actions ([mvp-plan risks](../product/mvp-plan.md#risks)).

## Scope

- `platform-windows` `Notifier` adapter, following the path chosen by #064:
  - Reminder toasts use the `reminder` scenario (they stay until acted on) and carry: title; due time; a Snooze selection (5, 10, 15, 60 min; default 10); and Done and Snooze buttons.
  - Heads-up toasts carry: event title; start time; location; and Join (only when #041 found a join link) and Dismiss buttons.
  - Digest and feed toasts carry an Open button that opens the main window on the item.
- **Activation:** button clicks arrive as `ToastActivation { action, item_id, occurrence, snooze_minutes }`. This works whether the app is running (in-process) or not (the OS launches it with activation arguments, and the single-instance handler from #009 forwards them).
- **Validation:** the activation handler parses arguments into a closed enum, checks that `item_id` exists and the action is allowed for its source, and ignores anything else. No free text from arguments is executed.
- **Join:** the core opens the join URL in the default browser only if it is `https` and its host matches a known meeting provider (Teams, Meet, Zoom, Webex). Otherwise the toast shows the domain and the button opens the event in the main window instead.
- **Retraction:** when an item is completed elsewhere (in-app, or on another device through #071), its toast is removed from Notification Center by tag and group.
- **"Private toasts"** setting (off by default): toasts show "Reminder" or "Meeting in 10 min" without the title, because toast content is stored by Windows outside our encrypted store.
- **Fallback, if #064 rules out COM activation:** a small always-on-top reminder card window (reuses the `panel` entry) with the same actions, and a plain toast whose click opens it.

Out of scope: the policy deciding whether to show a toast (#033), spoken alerts (#035), and cross-device retraction logic (#071, which calls the retraction API built here).

## Acceptance criteria

```gherkin
Feature: Actionable toasts

  Scenario: Snooze from a reminder toast
    Given a reminder "take out the trash" is delivered as a toast at 19:00
    When I choose 10 minutes and click Snooze
    Then the reminder occurrence is snoozed until 19:10
    And the toast is removed
    And at 19:10 a new toast is shown

  Scenario: Done from a toast when the app window is closed
    Given the app runs in the tray with no window open
    When I click Done on a reminder toast
    Then the occurrence is marked done
    And no window opens

  Scenario: Activation launches the app when it is not running
    Given the app is not running
    When I click Done on a reminder toast in Notification Center
    Then the app starts in the background and marks the occurrence done

  Scenario: Join opens a known meeting link
    Given a heads-up toast for an event with join link "https://teams.microsoft.com/l/meetup-join/abc"
    When I click Join
    Then the default browser opens that URL

  Scenario: Join refuses an unknown or non-https link
    Given an event whose extracted link is "http://example.test/join"
    Then the heads-up toast has no Join button
    And it has an Open button that shows the event in the main window

  Scenario: Forged activation arguments are ignored
    When the app receives activation arguments "action=done&id=does-not-exist"
    Then nothing changes and a redacted warning is logged

  Scenario: Private toasts hide the title
    Given private toasts are on
    When a reminder "doctor appointment" is delivered
    Then the toast text is "Reminder" and does not contain "doctor"
```

## Design notes

- **Crates:** the `platform-windows` `Notifier` implements the `platform-ports` trait, and only it touches the `windows` crate. The activation argument parser lives in `notify-domain` (pure, fuzzable). The known-meeting-host check lives in `calendar-domain` and is shared with #041.
- **Installer:** the AUMID and Start-menu shortcut are created by the installer (#075). Dev builds register them on first run under a dev AUMID so QA works before #075.
- **Security:** the activation surface is local input from the OS. Treat it like IPC: parse into a closed type and never pass it to a shell. URLs are opened through the shell "open" verb only after the https and host check. Note the `unsafe_code = "forbid"` workspace lint: COM activation needs a reasoned exception scoped to `platform-windows` (open question for the lead).
- **Privacy:** the "private toasts" setting is device-local.
- **Budget:** from button click to state change, p95 is under 300 ms when running and under 3 s on cold launch.

## QA procedure

Use a Windows 11 PC with a dev build installed through the dev installer, so the AUMID and shortcut exist.

1. Create "in 1 minute test toast". When the toast appears, pick 5 minutes and click Snooze. Observe that the toast disappears and returns 5 minutes later.
2. Close the main window (tray only). Click Done on the next toast. Observe that no window opens and the reminder shows as done when you open the app.
3. Quit from the tray. Open Notification Center (Win+N) and click Done on a leftover reminder toast. Observe that the app starts in the tray and the reminder is done.
4. Connect the QA Microsoft account (QA-MSA) calendar (needs #041) with a Teams meeting in 12 minutes. On the heads-up, click Join. Observe that Teams opens in the browser or the Teams app.
5. Turn on Settings → Notifications → Private toasts. Observe a reminder toast showing "Reminder" only.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed (os-integration `Notifier` row reflects the #064 outcome)
