# 032. Reminders and natural-language parsing

| Milestone    | Size | Depends on | Labels                       |
| ------------ | ---- | ---------- | ---------------------------- |
| M3 Reminders | M    | #031, #003 | area/reminders, type/feature |

## Story

As a knowledge worker, I want to say or type "remind me every Thursday to take out the trash" and have it understood offline, with a question if I left out the time, so that I can set a reminder faster than opening an app.

## Context

The headline reminder example. The overview gives the `reminders` context the reminder model, natural-language parsing, and provider sync ports ([overview](../architecture/overview.md#bounded-contexts-and-crates)). Parsing is "deterministic parser first; local LLM fallback" (`reminder-parse` task, [assistant](../architecture/assistant.md#model-router-models-context)). Issue #003 creates the `reminders-*` skeleton. This issue makes it real.

## Scope

- `reminders-domain`:
  - `Reminder { id, title, notes, schedule, zone_policy, channels, status, provider_link, source_text, created_via }`.
  - `Occurrence` state: `next_due`, `snoozed_until`, `last_fired`, and per-occurrence `done`.
  - Rules: completing a recurring reminder completes the current occurrence only. Snooze moves only the current occurrence. Delete is a tombstone.
- **Deterministic parser** in `reminders-domain`, English for MVP. It returns `ParseOutcome::Complete(draft) | NeedsTime(draft) | Ambiguous(draft, options) | NotUnderstood`. It covers:
  - "remind me every Thursday to take out the trash" → weekly on Thursday, title "take out the trash", `NeedsTime`
  - "remind me every Thursday at 7pm to take out the trash" / "every Thursday at 19:00 …"
  - "in 20 minutes …", "tomorrow at 8 …", "next Friday at noon …", "on December 3rd at 9 …"
  - "every day / weekday / weekend at 8:30 …", "every other Monday …", "on the 1st of every month …", "every last Friday of the month …"
  - "at 7" with no am/pm: a one-off picks the next 7:00 within 12 h. A recurring reminder returns `Ambiguous` with "7 AM or 7 PM?"
- **Missing time:** if the setting `reminders.default_time` is set (for example 09:00), apply it and say so in the confirmation ("every Thursday at 9:00 AM"). Otherwise return `NeedsTime`, and the caller asks "What time?". The answer ("7 pm", "at 19") completes the draft.
- **LLM fallback** (`reminder-parse` task through the router, #047, if present): used only when the parser returns `NotUnderstood`. The output is JSON matching a fixed schema, validated by the domain, and always shown as a preview. When no model is available, offline or not, the result is "I couldn't parse that; open the editor?"
- `reminders-application` use cases: `parse`, `create`, `update`, `complete`, `snooze(minutes)`, `delete`, and `list`. Each reminder registers a scheduler job (`FireOnceLate`, floating zone by default). On `JobDue` it publishes `ReminderDue { reminder_id, occurrence, late_by }` for the notify context.
- IPC commands: `reminders_parse`, `reminders_create`, `reminders_update`, `reminders_complete`, `reminders_snooze`, `reminders_delete`, `reminders_list`, and `reminders_preview_occurrences(schedule, count)`.
- The local provider is the default `TaskProvider`. External providers come in #036–#038.

Out of scope: the UI (#039), toasts (#034), spoken alerts (#035), assistant tool registration (#049), and location and person triggers (#086). Non-English grammars are a follow-up.

## Acceptance criteria

```gherkin
Feature: Reminders from natural language

  Scenario: Headline example with no time asks "What time?"
    Given the network is disconnected
    And no default reminder time is set
    When I parse "remind me every Thursday to take out the trash"
    Then the outcome is NeedsTime with title "take out the trash" and rule "FREQ=WEEKLY;BYDAY=TH"
    And the prompt is "What time?"
    When I answer "7 pm"
    Then the draft is complete: every Thursday at 19:00, floating time zone

  Scenario: Default time is applied when set
    Given the default reminder time is 09:00
    When I parse "remind me every Thursday to take out the trash"
    Then the draft is complete: every Thursday at 09:00
    And the confirmation text says "every Thursday at 9:00 AM"

  Scenario: Full sentence is parsed without any model
    Given no language model is configured
    When I create a reminder from "remind me every Thursday at 7pm to take out the trash"
    Then a reminder exists whose next due time is the next Thursday at 19:00 local
    And no call was made through the model router

  Scenario: Ambiguous hour on a recurring reminder
    When I parse "every Monday at 7 call mom"
    Then the outcome is Ambiguous with options "7:00 AM" and "7:00 PM"

  Scenario: Completing one occurrence keeps the series
    Given a reminder every Thursday at 19:00 is due on 2026-11-19
    When I mark it done
    Then the next due time is 2026-11-26 19:00

  Scenario: Unparseable text offline
    Given the network is disconnected and no local model is running
    When I parse "ping me about that thing when it makes sense"
    Then the outcome is NotUnderstood with the action "Open the editor"

  Scenario: LLM fallback output is validated
    Given the local model returns a schedule with "BYDAY=XX"
    When I parse "remind me on the second workday after payday"
    Then the draft is rejected as invalid and no reminder is created
```

## Design notes

- **Crates:** extend `reminders-domain`, `reminders-application`, and `reminders-infrastructure`. They use `Recurrence` from `kernel-time` (created by #031); `reminders-domain` allows `kernel` and `kernel-time`. The parser is a hand-written tokenizer plus a small grammar, with no regex crate in the domain unless it passes the bans. Keep each function at complexity ≤ 6 by splitting into per-phrase parsers.
- **arch.toml:** `reminders-application` may allow `scheduler-domain` (another context's domain, for `Schedule`) but not `scheduler-application`. It reaches the scheduler through a `ReminderScheduler` port that the root wires to `scheduler-application`, and it reaches the router through a `ReminderParseModel` port.
- **Migrations:** `reminders`, `reminder_occurrences`. Fields are declared synced. Op emission is wired by #069.
- **Privacy:** reminder text is `DataClass::Reminders`. The `reminder-parse` fallback is local by default, and a cloud route needs the user's policy to allow `Reminders` for that provider (#047).
- **Fuzzing:** the parser takes free text from voice and the assistant. Add a cargo-fuzz target (`fuzz_reminder_parse`) now, even though fuzzing in CI is post-MVP ([security](../architecture/security.md)).
- **Budget:** a deterministic parse takes under 1 ms p95.

## QA procedure

1. Unplug the network. In a dev build, open the reminders command palette (or call `reminders_parse` from the dev console) with "remind me every Thursday to take out the trash". Observe the prompt "What time?".
2. Answer "7 pm". Observe the preview "Every Thursday at 7:00 PM: take out the trash" and the next date.
3. Set Settings → Reminders → Default time to 09:00 and repeat step 1. Observe that it completes with 9:00 AM and no question.
4. Create "in 2 minutes stretch". Observe `ReminderDue` in the log at the right minute (a toast arrives once #034 lands).
5. Create "every Monday at 7 call mom". Observe the AM/PM choice.
6. Mark the Thursday reminder done on its due day. Observe that the next due date is the following Thursday.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
