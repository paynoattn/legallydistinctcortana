# 036. Microsoft To Do connector

| Milestone    | Size | Depends on | Labels                                        |
| ------------ | ---- | ---------- | --------------------------------------------- |
| M3 Reminders | M    | #032, #040 | area/connectors, area/reminders, type/feature |

## Story

As a knowledge worker who already uses Microsoft To Do, I want my reminders to sync both ways with To Do, so that "remind me every Thursday to take out the trash" shows up on my phone too.

## Context

Microsoft To Do is a Must reminder provider ([mvp-plan scope](../product/mvp-plan.md#scope-moscow)). The connector implements `reminders-application::TaskProvider` through Graph `/me/todo/lists/{id}/tasks`, with full read and write. Recurrence maps to Graph `patternedRecurrence` ([connectors: reminders and tasks](../architecture/connectors.md#reminders-and-tasks)). Two-way rules: the provider is the source of truth for items it owns, our copy carries extensions (spoken alerts), conflicts resolve by last-modified, and a conflict log is visible.

## Scope

- In `connector-msgraph`: a To Do `TaskProvider` adapter.
  - List task lists. The user picks which lists to sync and the default list for new reminders.
  - Incremental pull per list (delta query; verify that To Do task delta is available for personal and work accounts). The fallback is a full list compare by `lastModifiedDateTime`.
  - Create, update, complete, and delete tasks.
- **Mapping:**
  - title ↔ `title`; notes ↔ `body` (text)
  - due ↔ `dueDateTime` + `reminderDateTime` with `isReminderOn`
  - done ↔ `status: completed`
  - recurrence ↔ `patternedRecurrence` (daily, weekly with `daysOfWeek`, `absoluteMonthly`, `relativeMonthly`, `absoluteYearly`, `relativeYearly`, with `interval`, plus `range` end date or occurrence count)
- **Unmappable rules** (for example `BYSETPOS` combinations Graph cannot express): keep the schedule locally and mirror only the next occurrence as a one-off task, with a note in the conflict log.
- **Completing a recurring task:** verify how Graph advances a recurring To Do task on completion, and match our occurrence model to it.
- **Double alerts:** To Do apps also alert. A per-list setting "Alert from: this app / To Do / both", defaulting to this app, controls whether `isReminderOn` is set upstream.
- Polling runs on the scheduler at the account cadence (default 5 min) under the account lease (#046, once available).
- Errors:
  - 401 → refresh the token, then "needs sign-in" in Settings → Accounts.
  - 429 or 503 → honor `Retry-After` with jittered backoff.
  - 404 on a deleted list → unlink and log.

Out of scope: OAuth and the vault (#040), the reminders UI (#039), and To Do steps, attachments, and linked resources (ignored, and preserved on update).

## Acceptance criteria

```gherkin
Feature: Microsoft To Do sync

  Scenario: Headline reminder appears in To Do
    Given a Microsoft account with list "Tasks" set as the default reminder list
    When I create "every Thursday at 7pm take out the trash"
    Then a To Do task "take out the trash" is created with weekly recurrence on Thursday
    And its reminder time is 19:00 in my time zone

  Scenario: Task created in To Do appears locally
    Given a task "renew passport" due 2026-12-01 was created in the To Do app
    When the next poll runs
    Then a local reminder "renew passport" exists linked to that task

  Scenario: Completion in To Do completes the local occurrence
    Given the linked recurring task is completed in the To Do app
    When the next poll runs
    Then the local occurrence is done and the next occurrence is scheduled

  Scenario: Both sides edited, the later edit wins and is logged
    Given the title was changed locally at 10:00 and in To Do at 10:05
    When the next poll runs
    Then the title is the To Do value
    And the conflict log has an entry with both values

  Scenario: Expired refresh token
    Given the refresh token was revoked
    When a poll runs
    Then the account shows "Sign in again" in Settings → Accounts
    And local reminders keep firing

  Scenario: Throttled by Graph
    Given Graph answers 429 with Retry-After 30
    When a poll runs
    Then the next request is not sent before 30 seconds

  Scenario: Offline creation is queued
    Given the network is disconnected
    When I create a reminder in the To Do list
    Then it fires locally on time
    And the task is created in To Do after the network returns
```

## Design notes

- **Crates:** `connector-msgraph` (shared with #041 and #043) implements `TaskProvider`. RRULE ↔ `patternedRecurrence` mapping lives in the connector as pure functions with property tests (round trip for every mappable rule).
- **arch.toml:** `connector-msgraph` allows `reminders-application`, `reminders-domain`, `kernel`, and `store`. It may also use `accounts-application` for tokens (see #040).
- **Scopes:** `Tasks.ReadWrite` and `offline_access`, requested incrementally when the user enables To Do on an account.
- **Data:** the migration adds `reminder_provider_links` (`reminder_id`, `account_id`, `list_id`, `remote_id`, `etag`, `remote_modified`), owned by `reminders-infrastructure`, plus an outbox table for offline writes.
- **Privacy:** reminder text goes to Microsoft by the user's choice of provider. Record it in the off-device audit log (#074) as provider "Microsoft Graph", with byte counts only.
- **Testing:** contract tests against a fake `TaskProvider` and recorded Graph fixtures. Live tests are manual (below).

## QA procedure

Accounts: QA-MSA (a personal Microsoft account) and QA-M365 (a work tenant account); the Microsoft To Do app on a phone or the web.

1. Settings → Accounts → add QA-MSA and enable To Do. Choose list "Tasks" as the default.
2. Say or type "remind me every Thursday at 7pm to take out the trash". Within 5 min, observe the task in To Do on the web, recurring weekly on Thursday at 7:00 PM.
3. In To Do on the phone, create "renew passport" due next week. Within one poll, observe it in the app's reminders list.
4. Complete the trash task in To Do. Observe the local occurrence done and next Thursday scheduled.
5. Disconnect Wi-Fi, create a reminder, and reconnect. Observe it in To Do within one poll.
6. At account.microsoft.com, remove the app's access, then wait for a poll. Observe "Sign in again", and local reminders still firing.
7. Repeat steps 1–2 with QA-M365. If the tenant blocks consent, observe the admin-consent message.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
