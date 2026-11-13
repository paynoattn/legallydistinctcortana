# 041. Calendar sources (Graph, Google, CalDAV/iCloud, ICS) and heads-up alerts

| Milestone            | Size | Depends on | Labels                                       |
| -------------------- | ---- | ---------- | -------------------------------------------- |
| M4 Calendar and mail | L    | #040, #033 | area/calendar, area/connectors, type/feature |

## Story

As a knowledge worker, I want my Microsoft, Google, and iCloud calendars connected so that I get a heads-up before each meeting with the join link, so that I am never late and never hunting for the Teams link.

## Context

MVP done-criterion 6 ([mvp-plan](../product/mvp-plan.md#what-done-means-for-the-mvp)). The `calendar` context owns events, alert lead times, and join links ([overview](../architecture/overview.md#bounded-contexts-and-crates)). `CalendarSource` adapters come from `connector-msgraph`, `connector-google`, and `connector-caldav`, plus local ICS ([connectors: calendar](../architecture/connectors.md#calendar)). The heads-up watcher fires at start minus (travel + prep lead, default 10 min) ([assistant: proactive engine](../architecture/assistant.md#proactive-engine)).

## Scope

- `calendar-domain`:
  - `Event { id, source_calendar, ical_uid, title, start, end, all_day, location, join_link, organizer, response, recurrence }`.
  - Occurrence expansion for the next 14 days, using `Recurrence` from `kernel-time` (#031).
  - Join link extraction:
    1. provider fields first (Graph `onlineMeeting.joinUrl`, Google `conferenceData` / `hangoutLink`)
    2. then the location
    3. then the body, matched against Teams, Meet, Zoom, and Webex URL patterns; https only
- `calendar-application`:
  - Sync per calendar on the scheduler (default every 10 min).
  - Heads-up planning: one scheduler job per upcoming occurrence at `start − lead`. Lead is per calendar (default 10 min) plus an optional per-event prep lead.
  - Publishes `HeadsUpDue { event_id, occurrence, join_link }`, which notify turns into a toast (#034) or speech (#035).
- **Rules:**
  - No heads-up for declined events, all-day events, or events the user marked "free" (setting).
  - The same `ical_uid` in two calendars (an invite in both work and personal) produces one heads-up.
  - A moved event reschedules its job. A cancelled event cancels it.
  - After sleep, a heads-up whose event started more than 5 min ago is not shown (scheduler misfire `FireIfWithin`).
- **Adapters:**
  - Graph: `calendarView` + delta, scope `Calendars.Read`.
  - Google: Calendar API v3 `events.list` with `syncToken`, scope `calendar.readonly`. Handle the 410 that invalidates a sync token by doing a full resync.
  - CalDAV (iCloud `caldav.icloud.com`, Nextcloud, Fastmail): VEVENT through the discovery and ICS parser from #038.
  - ICS: URL with `ETag` / `If-Modified-Since`, or a local file, read-only.
- Read tools for the assistant, registered once #049 exists: `calendar.next_event` and `calendar.list_day` (effect `Read`, data class `Calendar`).
- `src/calendar/` UI: a calendars list in the account pages with a per-calendar lead time, "Speak heads-ups", and "Alert me" toggles, plus a simple agenda for today and tomorrow.

Out of scope: creating or moving events (a Write tool, later), travel-time lead (#094), and the Today schedule section (#066).

## Acceptance criteria

```gherkin
Feature: Calendar heads-up

  Scenario: Heads-up 10 minutes before with a Teams link
    Given a Microsoft calendar with an event "Design review" at 10:00 with an online meeting
    When the clock reaches 09:50
    Then a HeadsUpDue is published with the Teams join link
    And a toast shows "Design review in 10 minutes" with a Join button

  Scenario: Join link from the body of an iCloud event
    Given an iCloud CalDAV event whose description contains "https://us02web.zoom.us/j/123456789"
    When the event is synced
    Then its join_link is that Zoom URL

  Scenario: Moved event reschedules the heads-up
    Given a heads-up job for an event at 15:00
    When the organizer moves it to 16:00 and the next sync runs
    Then the heads-up is scheduled for 15:50 and nothing fires at 14:50

  Scenario: Declined and all-day events do not alert
    Given an all-day event and a declined event today
    Then no heads-up job exists for either

  Scenario: Same invite in two calendars alerts once
    Given the same ical_uid appears in a Google and a Microsoft calendar
    When the heads-up time comes
    Then exactly one notification is requested

  Scenario: Laptop resumes after the meeting started
    Given an event at 10:00 with a heads-up at 09:50
    And the laptop slept from 09:30 to 10:20
    When it resumes
    Then no heads-up is shown and the event appears as "started 20 min ago" on Today

  Scenario: Google sync token expired
    Given the Google API answers 410 for the stored syncToken
    When a sync runs
    Then a full resync happens and heads-ups are rebuilt without duplicates

  Scenario: Offline keeps alerting from the cache
    Given events were synced and the network is now disconnected
    When the heads-up time for a cached event comes
    Then the heads-up is shown
```

## Design notes

- **Crates:** create `calendar-domain` and `calendar-application`. Adapters live in `connector-msgraph`, `connector-google`, and `connector-caldav`. The ICS-over-HTTP adapter goes in `connector-caldav` too (shared parser). The known-meeting-host list is in `calendar-domain` and is reused by #034.
- **arch.toml:** `calendar-domain` allows `kernel` and `kernel-time`. `calendar-application` allows `calendar-domain`, `kernel`, and `scheduler-domain` (for `Schedule` values). Connectors allow `calendar-application` and `calendar-domain`.
- **Migrations:** `calendar_calendars` (synced settings: lead, speak, alert), `calendar_events` (cache), `calendar_alert_state` (synced, per [sync: what syncs](../architecture/sync.md#what-syncs)).
- **Sync:** event summaries (title, times, location, join link) from the polling device sync as `DataClass::Calendar` ops, so every device can plan heads-ups and show Today ([sync: what syncs](../architecture/sync.md#what-syncs)). Descriptions stay device-local. Each device plans its own heads-up jobs from the synced events, and single delivery (#071) decides which one notifies.
- **Security:** event descriptions are untrusted content (invites from strangers). They are never sent to a model with tools, and join links pass the https and host check.
- **Privacy:** `DataClass::Calendar`, local by default. Provider calls are logged to the audit log (#074) with byte counts.
- **Testing:** recorded fixtures for Graph and Google, a Radicale container for CalDAV, and property tests for join-link extraction and dedupe.

## QA procedure

Accounts: QA-MSA or QA-M365 (Outlook calendar with Teams), QA-GOOG (Google Calendar with Meet), QA-ICLOUD (iCloud Calendar).

1. Connect all three calendars (Settings → Accounts). Observe today's events in the agenda within one sync.
2. In Outlook, create a Teams meeting starting in 12 minutes. Wait. At 10 minutes before, observe a toast with Join. Click it and observe Teams opening.
3. In Google Calendar, create a Meet event in 15 minutes and then move it 30 minutes later. Observe no toast at the original time and one at the new time.
4. Invite QA-GOOG and QA-MSA to the same event. Observe one heads-up, not two.
5. In iCloud Calendar on iPhone or icloud.com, add an event with a Zoom link in the notes. Observe the Join button.
6. Disconnect the network after a sync. Observe heads-ups still firing from the cache.
7. Turn on "Speak heads-ups" for one calendar. Observe the spoken heads-up (#035).

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
