# 071. Presence and single delivery across devices

| Milestone | Size | Depends on | Labels                               |
| --------- | ---- | ---------- | ------------------------------------ |
| M7 Sync   | M    | #070, #033 | area/sync, area/notify, type/feature |

## Story

As a person with a desktop and a laptop that are both on, I want a reminder, meeting heads-up, or mail digest to appear once, on the device I am actually using, and to disappear from the others when I act on it, so that I am not interrupted twice and never miss one.

## Context

MVP done-criterion 4 ("on whichever of their computers they are using") and the M7 exit demo. Design: [sync: presence and single delivery](../architecture/sync.md#presence-and-single-delivery). The delivery policy (quiet hours, focus, batching) is [#033](033-delivery-policy.md); this issue adds the cross-device decision in front of it, as described in [assistant: proactive engine](../architecture/assistant.md#proactive-engine).

## Scope

- **Presence:** each device publishes a presence op `{device_id, last_active, locked, on_battery}` on user input (debounced to once a minute), lock, unlock, sleep, and resume.
- **Delivery target** (pure function in `notify-domain`): among devices whose presence is fresh, the unlocked device with the most recent input, ties broken by device id; if all are locked, the most recently active device.
- **Delivery:** when an item is due, every online device computes the same target. Only the target notifies (subject to its local delivery policy) and writes `delivered(item_id, device_id, hlc)`. Others suppress the item, or remove their toast by tag if they already showed it.
- **Fallback:** if no `delivered` mark is seen within 20 s, the next device in order delivers. Worst case is a duplicate, never a miss.
- **Actions sync:** Snooze or Done on one device updates the reminder (an op) and clears the toast on every device.
- **Device-addressed items:** "Send to my laptop" and "Read this on my desktop" (from the browser extension and the assistant) become ops addressed to one device and are delivered there on its next sync.
- **History:** each delivered item shows "Delivered on <device name>" in the reminder or notification history.

Out of scope: per-device channel preferences beyond the existing delivery policy; mobile devices.

## Acceptance criteria

```gherkin
Feature: Single delivery across devices

  Scenario: The active device delivers
    Given laptop L and desktop D are in sync and both unlocked
    And my last input was on L 10 seconds ago and on D 2 hours ago
    When the reminder "Take out the trash" becomes due
    Then L shows a toast with Snooze and Done
    And D shows no toast
    And both list the reminder as "Delivered on L"

  Scenario: A locked device is skipped
    Given L is locked and D is unlocked with input 5 minutes ago
    When a meeting heads-up becomes due
    Then D delivers it and L does not

  Scenario: The target is asleep
    Given L was the most recently active device and went to sleep without a presence update
    When a reminder becomes due
    Then D delivers it within 25 seconds of the due time

  Scenario: Done on one device clears the other
    Given a partition made both L and D show the same reminder toast
    When I click Done on L and the partition heals
    Then within 5 seconds D's toast is removed and the reminder is done on both

  Scenario: A single device does not wait
    Given L is the only device in the group
    When a reminder becomes due
    Then L delivers it at the due time with no 20-second wait

  Scenario: Presence contains only what it needs
    When L publishes presence
    Then the op contains only device_id, last_active, locked, and on_battery

  Scenario: Send to another device
    Given I select text in Edge on D and choose "Send to my other device" then "L"
    When L next syncs
    Then L reads the text aloud or shows it, according to the action chosen, and D does not
```

## Design notes

- **Crates:** `notify-domain` (`DeliveryTarget` selection, fallback order, freshness window; pure, property-tested so every device computes the same target from the same presence set), `notify-application` (`DeliverAcrossDevices` use case in front of the existing policy; ports `PresenceReader`, `DeliveredMarks`, `Notifier`), `sync-application` (presence publisher), `platform-ports` (`UserActivity`: last-input time and lock state; `platform-windows` uses `GetLastInputInfo` polled every 15 s plus `WTSRegisterSessionNotification`). `arch.toml` updates are protected: need a human's approval.
- **Freshness:** a device counts as online if presence was received in the last 3 minutes or it has a live connection (#070).
- **Toast removal:** removing a delivered toast by tag on the other devices depends on the API chosen in spike [#064](064-spike-actionable-toasts.md); if removal is not possible, the in-app card is updated instead.
- **Leases versus delivery** ([sync: presence and single delivery](../architecture/sync.md#presence-and-single-delivery)): leases ([#031](031-scheduler.md), [#046](046-connector-leases.md)) decide which device does the work; single delivery decides which device notifies. Delivery never consults leases: every device evaluates due items, so a sleeping lease holder cannot cause a miss.
- **Data:** presence and delivered marks are synced entities (#069) with short retention (presence: latest per device; delivered marks: 30 days). Migration in `notify` infrastructure.
- **Privacy:** presence reveals activity times only to the user's own devices; no app or window names.
- **Budget:** the decision adds ≤ 50 ms to delivery on the target in the common case.

## QA procedure

Two Windows 11 PCs, L and D, syncing (#070), both with notifications on.

1. On L, create a reminder for 2 minutes from now. Keep typing in Notepad on L. Observe a toast on L only; on D, the reminder history says "Delivered on L".
2. Lock L (Win+L). Create a reminder due in 1 minute on D and use D. Observe the toast on D only.
3. Put L to sleep from the Start menu right after using it. Create a reminder on D due in 1 minute and do not touch D. Observe D delivering within about 20 s of the due time.
4. Disconnect both from the network. Create a reminder on each due in 1 minute (synced earlier, so both have it). Both show a toast. Click Done on L, reconnect, and observe D's toast clear within 5 s.
5. In Edge on D, select text, right-click → "Send to my other device" → L. Observe it on L.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] sync.md updated (freshness window, all-locked rule)
