# 038. CalDAV VTODO connector

| Milestone    | Size | Depends on | Labels                                        |
| ------------ | ---- | ---------- | --------------------------------------------- |
| M3 Reminders | M    | #032       | area/connectors, area/reminders, type/feature |

## Story

As a privacy-conscious user with Nextcloud, Fastmail, or Radicale, I want my reminders to sync with my CalDAV task lists, so that I keep one task list across all my apps without a big-tech account.

## Context

CalDAV VTODO is a Should provider ([mvp-plan scope](../product/mvp-plan.md#scope-moscow)), served by `connector-caldav` through `reminders-application::TaskProvider` ([connectors](../architecture/connectors.md#reminders-and-tasks)). Incremental sync uses `sync-token` (RFC 6578) with a ctag/etag fallback ([connectors: common rules](../architecture/connectors.md#common-rules)). iCloud Reminders created in the modern Reminders app are not exposed over CalDAV. That question belongs to spike #063, and this connector does not promise iCloud Reminders.

## Scope

- In `connector-caldav`:
  - **Discovery:** `/.well-known/caldav` → `current-user-principal` → `calendar-home-set` → collections whose `supported-calendar-component-set` includes `VTODO`.
  - **Sync:** a `sync-collection` REPORT with a stored `sync-token`. When the server lacks it or the token is invalid, compare `getctag` and per-item `getetag`.
  - **Writes:** create with `PUT` + `If-None-Match: *`, update with `If-Match: <etag>`, delete with `If-Match`. On `412 Precondition Failed`, refetch, resolve by `LAST-MODIFIED`, and log the conflict.
- **VTODO mapping:**
  - `SUMMARY`, `DESCRIPTION`
  - `DTSTART` / `DUE` with `TZID`
  - `RRULE` (native, so recurrence round-trips)
  - `VALARM` (`ACTION:DISPLAY`, relative trigger)
  - `STATUS:COMPLETED`, `COMPLETED`, `PERCENT-COMPLETE`
  - Unknown properties and `X-` properties are preserved on update.
- **Recurring completion:** client behavior varies between servers and clients (advancing `DTSTART`/`DUE` versus `RECURRENCE-ID` overrides). Verify what Nextcloud Tasks, Thunderbird, and Apple clients do. Default to advancing `DUE` to the next occurrence, keeping the series in one resource.
- **Account types:** generic CalDAV (URL, user, password or app password), presets for Nextcloud and Fastmail, and iCloud CalDAV for legacy task lists only (with a note pointing at #063).
- Polling runs on the scheduler at the account cadence (default 5 min).

Out of scope: VEVENT calendars (#041 reuses the ICS parser and discovery from here), iCloud Reminders from the modern Reminders app (#063), and scheduling (iTIP).

## Acceptance criteria

```gherkin
Feature: CalDAV VTODO sync

  Scenario: Headline reminder round-trips with native recurrence
    Given a CalDAV account with task list "Personal" as the default
    When I create "every Thursday at 7pm take out the trash"
    Then the server has a VTODO with SUMMARY "take out the trash" and RRULE "FREQ=WEEKLY;BYDAY=TH"
    And its DUE is 19:00 in my time zone with a TZID

  Scenario: Incremental sync with sync-token
    Given a prior sync stored a sync-token
    And one VTODO was added on the server
    When the next poll runs
    Then only the changed resource is fetched
    And a local reminder exists for it

  Scenario: Server without sync-collection falls back to etags
    Given the server does not support sync-collection
    When a poll runs
    Then items whose getetag changed are fetched and the others are not

  Scenario: Concurrent edit returns 412
    Given the local title changed and the server copy changed after our last fetch
    When the update is sent with If-Match
    Then the server answers 412
    And the connector refetches, keeps the newer LAST-MODIFIED, and logs the conflict

  Scenario: Unknown properties survive an update
    Given a VTODO with "X-NC-CUSTOM:1" and "CATEGORIES:home"
    When I rename it locally
    Then the updated resource still contains "X-NC-CUSTOM:1" and "CATEGORIES:home"

  Scenario: Wrong password
    Given the server answers 401
    When a poll runs
    Then Settings → Accounts shows "Password rejected by <host>"
    And local reminders keep firing

  Scenario: Plain HTTP is refused except on loopback
    When I add a CalDAV account at "http://cloud.example.test/remote.php/dav"
    Then the account is rejected with "Use https"
```

## Design notes

- **Crates:** `connector-caldav` implements `TaskProvider`. It holds a WebDAV client (on `reqwest` with rustls), an XML layer (`quick-xml` with DTDs and entity expansion disabled, to prevent XXE), and an iCalendar parser and serializer. Choose the iCalendar crate in coding and verify its license with `cargo deny`. If none fits, write a minimal RFC 5545 parser for the properties above. The parser is a fuzz target (`fuzz_ics_parse`), because server content is untrusted input.
- **arch.toml:** `connector-caldav` allows `reminders-application`, `reminders-domain`, `calendar-application`, `calendar-domain`, `kernel`, `store`, and `accounts-application`.
- **Auth:** Basic auth over TLS only, with the password from the credential vault (#040). This issue needs only password storage from #040, not OAuth.
- **Data:** reuses `reminder_provider_links` (+ `sync_token`, `ctag` per collection, `etag` per item) and the outbox.
- **Privacy:** reminder text goes to the user's server. Write an audit log entry (#074) when the host is not loopback or the user's own LAN (the user labels a host as "my server").
- **Testing:** contract tests against a Radicale container in CI ([overview testing notes](../architecture/overview.md#quality-gates)), plus recorded Nextcloud fixtures.

## QA procedure

Accounts: QA-NEXTCLOUD (a Nextcloud instance with the Tasks app), QA-FASTMAIL (optional), and a local Radicale container (`docker run` per the `qa/procedures` file).

1. Settings → Accounts → Add → CalDAV with the Nextcloud URL and an app password. Observe the task lists discovered. Pick "Personal".
2. Create "every Thursday at 7pm take out the trash". In Nextcloud Tasks, observe the task with a weekly repeat and a 7 PM due time.
3. Add a task in Nextcloud Tasks. Observe it locally after one poll.
4. Edit the same task's title in both places within one poll interval. Observe one winner and an entry in Settings → Accounts → Conflict log.
5. Change the app password in Nextcloud. Observe "Password rejected" after the next poll.
6. Point an account at the Radicale container on `http://127.0.0.1:5232`. Observe that it is accepted (loopback), with sync working.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
