# 039. Reminders UI with recurrence editor

| Milestone    | Size | Depends on | Labels                                |
| ------------ | ---- | ---------- | ------------------------------------- |
| M3 Reminders | M    | #032, #008 | area/reminders, area/ui, type/feature |

## Story

As a knowledge worker, I want a reminders screen where I can type a reminder in plain words, see exactly when it will fire, and fine-tune the recurrence with a form, so that I trust what I set.

## Context

The main window hosts reminders ([ui: windows](../architecture/ui.md#windows-and-entry-points)). The UI follows the feature layout `src/reminders/{domain,application,infrastructure,ui}` with typed IPC from #008, zod at the boundary, and stores read with `useSyncExternalStore` ([ui](../architecture/ui.md#ipc)). The parsing and occurrence logic stay in Rust (#032). The UI never re-implements RRULE expansion.

## Scope

- A lazy `reminders` route in `main` with:
  - **Quick add:** a text field that calls `reminders_parse` as you type (debounced 150 ms) and shows a preview chip, such as "Every Thursday at 7:00 PM: take out the trash". If the outcome is `NeedsTime`, an inline "What time?" time field appears, focused. If it is `Ambiguous`, AM and PM buttons appear. If it is `NotUnderstood`, an "Open the editor" button appears.
  - **List:** groups for Overdue, Today, Upcoming, Recurring, All-day, and Completed (collapsed). Each row shows the title, the next due time, a recurrence summary, a provider badge (Local, To Do, Google, CalDAV), and actions: Done, Snooze (10 min, 1 h, tomorrow), and Delete (with an undo toast).
  - **Editor dialog:**
    - title and notes
    - date and time
    - repeat: Never, Daily, Weekly with weekday toggles, Monthly by day or by "second Tuesday", Yearly
    - interval, and an end: never, on a date, or after N times
    - time zone: "Follow my current time zone" (default) or a fixed zone
    - alert channels: toast, and spoken (from #035)
    - provider and list
  - **Occurrence preview:** the next 5 dates from `reminders_preview_occurrences`, refreshed when the form changes.
- `reminders/application`: `RemindersStore` (snapshot plus subscribe), `QuickAddStore`, and the `ReminderGateway` port. `reminders/infrastructure`: the Tauri adapter and a fake adapter for acceptance (`?ipc=fake`).
- Live updates: the store subscribes to `ReminderChanged` and `ReminderDue` events through the subscription port from #011.
- Empty state: an example ("Try: remind me every Thursday to take out the trash").

Out of scope: the parser (#032), provider connectors (#036–#038), and the Today view section (#066).

## Acceptance criteria

```gherkin
Feature: Reminders screen

  Scenario: Quick add asks for the missing time
    Given I am on the reminders screen with the fake IPC adapter
    When I type "remind me every Thursday to take out the trash"
    Then I see the preview "Every Thursday: take out the trash"
    And a focused field labelled "What time?"
    When I enter "7:00 PM" and press Enter
    Then the list shows "take out the trash" under Recurring with "Every Thursday at 7:00 PM"

  Scenario: Editor previews the next occurrences
    Given I open the editor for a new reminder
    When I choose Weekly, toggle Thursday, set 19:00, and end after 3 times
    Then the preview lists exactly three Thursdays at 7:00 PM

  Scenario: Monthly by weekday ordinal
    When I choose Monthly and "on the last Friday"
    Then the preview shows the last Friday of each of the next 5 months

  Scenario: Delete with undo
    Given a reminder "water the plants"
    When I delete it and click Undo within 5 seconds
    Then "water the plants" is back in the list

  Scenario: Invalid data from the core is rejected at the boundary
    Given the fake adapter returns a reminder with a missing next_due
    When the list loads
    Then that row shows "Could not load this reminder" and the others render

  Scenario: Keyboard-only use
    When I tab to quick add, type a reminder, press Enter, then Tab to its Done button and press Space
    Then the reminder is marked done without using a mouse

  Scenario: Live update from another source
    Given the reminders screen is open
    When a ReminderChanged event arrives for a reminder completed from a toast
    Then the row moves to Completed without a reload
```

## Design notes

- **UI feature:** `src/reminders/`.
  - `domain/`: view models, recurrence summary formatting, and time formatting per locale with `Intl.DateTimeFormat`.
  - `application/`: stores and ports.
  - `infrastructure/`: the Tauri and fake adapters with zod/mini schemas.
  - `ui/`: `RemindersScreen`, `QuickAdd`, `ReminderRow`, `RecurrenceEditor`, and `OccurrencePreview`.
- **Rules:** no `useEffect`. Debounce lives in the store, not in a component. The recurrence editor's state is a pure reducer in `domain/` that produces the `Schedule` DTO the core validates.
- **dependency-cruiser:** `reminders` is imported by other features only through `src/reminders/index.ts`. #066 uses it for Today's reminders.
- **IPC:** uses the commands from #032. Rust is the single source of validation, and the UI shows the core's error text.
- **Bundle:** the route is lazy-loaded, so `main` stays within 120 KB gzip for the initial entry.
- **Accessibility:** every control is labelled, the preview chip is an `aria-live="polite"` region, and the weekday toggles are a group with `aria-pressed`. Layouts are checked at 200% text scale.

## QA procedure

1. Open the app → Reminders. Type "remind me every Thursday to take out the trash". Observe the "What time?" field focused. Type 7 pm and press Enter.
2. Observe the row under Recurring with the next Thursday's date.
3. Open the row, switch to Monthly → "last Friday", and observe 5 correct dates in the preview (compare with a calendar).
4. Delete a reminder and click Undo. Observe it restored.
5. Use only the keyboard (Tab, Shift+Tab, Enter, Space, Esc) to add, edit, and complete a reminder.
6. Turn on Narrator (Ctrl+Win+Enter) and add a reminder. Observe that the preview is announced.
7. Set Windows text size to 200% (Settings → Accessibility → Text size). Observe no clipped or overlapping controls.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
