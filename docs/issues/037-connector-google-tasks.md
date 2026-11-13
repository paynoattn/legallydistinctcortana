# 037. Google Tasks connector

| Milestone    | Size | Depends on | Labels                                        |
| ------------ | ---- | ---------- | --------------------------------------------- |
| M3 Reminders | S    | #032, #040 | area/connectors, area/reminders, type/feature |

## Story

As a Google user, I want my reminders mirrored to Google Tasks, so that they show up in Gmail's side panel and on my Android phone.

## Context

Google Tasks is a Should provider ([mvp-plan scope](../product/mvp-plan.md#scope-moscow)). Tasks API v1 has no recurrence and no time of day, so we keep the schedule locally and mirror the task ([connectors: reminders and tasks](../architecture/connectors.md#reminders-and-tasks)). The two-way rules (provider owns its items, last-modified wins, conflict log) are shared with #036.

## Scope

- In `connector-google`: a Google Tasks `TaskProvider` adapter.
  - List task lists. The user picks which lists to sync and the default list for new reminders.
  - Pull incrementally per list with `updatedMin`, `showCompleted`, and `showHidden`. Use ETags where the API returns them.
  - Create, update, complete (`status: completed`), and delete tasks.
- **Mapping:** title ↔ `title`, notes ↔ `notes`, due date ↔ `due` (date only; verify that the API discards the time part), done ↔ `status`.
- **Time of day and recurrence stay local.** Our reminder fires at the local time. The mirrored task carries the date of the next occurrence only. When a recurring occurrence is completed (here or in Google), complete that task and create a new task for the next occurrence.
- **Tasks created in Google** with a date and no time become local reminders at `reminders.default_time`, if set. Otherwise they become all-day items that appear on Today and send no toast. They never ask "What time?" unprompted.
- Polling runs on the scheduler at the account cadence (default 5 min) under the account lease (#046, once available).
- Errors:
  - 401 → refresh, then "Sign in again".
  - 403 `rateLimitExceeded` or 429 → backoff with jitter, honoring `Retry-After` when present.

Out of scope: OAuth and the vault (#040), and Gmail (#044). Google OAuth app verification for the Tasks scope is covered by the guide from spike #062.

## Acceptance criteria

```gherkin
Feature: Google Tasks mirror

  Scenario: Recurring reminder mirrors only the next occurrence
    Given a Google account with list "My Tasks" as the default reminder list
    When I create "every Thursday at 7pm take out the trash" on Monday 2026-11-16
    Then a Google task "take out the trash" exists with due date 2026-11-19
    And the local reminder fires at 19:00

  Scenario: Completing the occurrence rolls the mirror forward
    Given the mirrored task for 2026-11-19 exists
    When I mark the occurrence done locally
    Then that Google task is completed
    And a new Google task is created with due date 2026-11-26

  Scenario: Task completed in Google completes the local occurrence
    Given the mirrored task is completed in Google Tasks on the phone
    When the next poll runs
    Then the local occurrence is done

  Scenario: Date-only task from Google with no default time
    Given no default reminder time is set
    And a Google task "call the bank" due 2026-11-20 was created on the phone
    When the next poll runs
    Then a local all-day item "call the bank" appears on Today for 2026-11-20
    And no toast is scheduled for it

  Scenario: Rate limited
    Given the Tasks API answers 429
    When a poll runs
    Then the connector backs off and Settings → Accounts shows "Google is rate limiting, retrying"
    And local reminders keep firing

  Scenario: Token revoked
    Given access was removed at myaccount.google.com
    When a poll runs
    Then the account shows "Sign in again"
```

## Design notes

- **Crates:** `connector-google` (shared with #041 calendar and #044 Gmail) implements `TaskProvider`. The "mirror the next occurrence" logic belongs in `reminders-application` as a provider capability (`supports_recurrence: false`, `supports_time: false`), so the connector stays a thin mapper and #036 and #038 can reuse it.
- **arch.toml:** `connector-google` allows `reminders-application`, `reminders-domain`, `kernel`, and `store`.
- **Scopes:** the Tasks scope (`https://www.googleapis.com/auth/tasks`). Verify whether Google classifies it as sensitive. Unverified OAuth clients show a warning screen and have a user cap, and the bring-your-own-client guide from #062 applies.
- **Data:** reuses `reminder_provider_links` and the outbox from #036.
- **Privacy:** task titles go to Google by the user's choice. Write an audit log entry (#074) per sync batch.
- **Testing:** recorded HTTP fixtures plus a fake provider. The capability-driven mirroring is unit-tested in `reminders-application` with fakes.

## QA procedure

Account: QA-GOOG (a Google account with Google Tasks); Google Tasks open in Gmail's side panel or on an Android phone.

1. Settings → Accounts → add QA-GOOG and enable Tasks. Choose "My Tasks" as the default list.
2. Create "every Thursday at 7pm take out the trash". Observe a Google task dated next Thursday, with no time.
3. At 7 pm on that Thursday (or in a test with a 2-minute recurring reminder), observe the local toast. Click Done, and observe the Google task completed and a new one for the next week.
4. On the phone, add "call the bank" for tomorrow without a time. After one poll, observe it on Today as an all-day item and no toast.
5. Remove the app at myaccount.google.com → Security → Third-party access. Observe "Sign in again" after the next poll.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
