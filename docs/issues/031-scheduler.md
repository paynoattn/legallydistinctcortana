# 031. Scheduler: durable jobs, RRULE, catch-up after sleep, leases

| Milestone    | Size | Depends on | Labels                       |
| ------------ | ---- | ---------- | ---------------------------- |
| M3 Reminders | L    | #006, #011 | area/scheduler, type/feature |

## Story

As a knowledge worker, I want everything time-based (reminders, heads-ups, digests, feed polls, routines) to fire at the right moment even after my laptop slept, rebooted, or crossed a DST change, so that I can trust the assistant with my schedule.

## Context

Every proactive feature runs on the scheduler ([assistant: proactive engine](../architecture/assistant.md#proactive-engine)). The overview assigns the `scheduler` context "durable jobs, RRULE recurrence, catch-up after sleep, lease so a job fires on one device only" ([overview](../architecture/overview.md#bounded-contexts-and-crates)). `PowerEvents` is the platform port that tells it about sleep and resume ([os-integration](../architecture/os-integration.md)).

This issue builds the engine. Other contexts own their jobs: reminders (#032), calendar heads-ups (#041), digests (#045), connector polls (#046), routines (#054), feeds (#059).

## Scope

- `scheduler-domain`: `Job { id, owner: JobOwner, schedule, misfire: MisfirePolicy, enabled }`, `Schedule::Once(instant) | Schedule::Recurring { rule, dtstart, zone: ZonePolicy }`, occurrence expansion, and misfire evaluation. All of it is pure and property-tested.
- `Recurrence`: an RFC 5545 RRULE subset (`FREQ` DAILY/WEEKLY/MONTHLY/YEARLY, `INTERVAL`, `BYDAY` with ordinals, `BYMONTHDAY`, `BYMONTH`, `BYSETPOS`, `COUNT`, `UNTIL`, `EXDATE`) with parse, display, and `next_after(instant)`. It lives in the new shared-kernel crate `kernel-time`, which this issue creates, because reminders, calendar, and routines need it too.
- `ZonePolicy::Floating` (follows the device's current time zone, the default for reminders) and `ZonePolicy::Fixed(iana_zone)`.
- DST rules: a nonexistent local time (spring forward) fires at the first valid instant after the gap. An ambiguous local time (fall back) fires once, at the earlier instant.
- `MisfirePolicy`: `FireOnceLate` (collapse every missed occurrence into one late fire carrying `missed_count`), `SkipToNext`, `FireIfWithin(duration)`.
- `scheduler-application`: a `Scheduler` service with `upsert`, `cancel`, `next_due`, and `tick(now)`; ports `JobStore`, `LeaseStore`, and the kernel `Clock`. Publishes `JobDue { job_id, owner, occurrence, late_by, missed_count }` on the event bus (#011).
- `scheduler-infrastructure`: a SQLCipher `JobStore` and `LeaseStore`, plus a runner that sleeps until the next due job (never longer than 60 s) and re-evaluates on resume, unlock, and system time or zone change.
- A run ledger keyed on `(job_id, occurrence)`, so a crash between "fire" and "record" can never fire the same occurrence twice on this device.
- A generic lease primitive: `Lease { key, holder_device, renewed_at (HLC), ttl }` with `acquire`, `renew`, and `release`, plus takeover after `ttl` of silence. Jobs that set `singleton: true` run only on the lease holder.

Out of scope: the reminders model and parsing (#032), deciding which device notifies (#033, #071), replicating leases between devices (#069), and per-account connector leases (#046). Waking the PC from sleep to run a job is not built; catch-up covers it.

## Acceptance criteria

```gherkin
Feature: Durable scheduler

  Scenario: Weekly recurring job fires at local wall time
    Given the device time zone is "Europe/Berlin"
    And a job recurs "FREQ=WEEKLY;BYDAY=TH" at 19:00 floating
    When the clock reaches Thursday 2026-11-19 19:00 local
    Then exactly one JobDue is published for that occurrence
    And the next due time is Thursday 2026-11-26 19:00 local

  Scenario: Fires at 19:00 local across a DST change
    Given a job recurs every Thursday at 19:00 in "America/New_York"
    When the occurrences around 2026-11-01 are expanded
    Then the occurrence on 2026-10-29 is at 23:00 UTC
    And the occurrence on 2026-11-05 is at 00:00 UTC the next day

  Scenario: Nonexistent local time during spring forward
    Given a daily job at 02:30 in "Europe/Berlin"
    When the occurrence for 2027-03-28 is expanded
    Then it fires at 03:00 local on 2027-03-28

  Scenario: Catch-up after sleep collapses missed occurrences
    Given an hourly job with misfire policy FireOnceLate
    And the device slept from 13:55 to 17:10
    When the device resumes
    Then one JobDue is published with missed_count 3 and late_by 70 minutes
    And the next due time is 18:00

  Scenario: Skip policy does not fire stale work after resume
    Given a feed poll job every 30 minutes with misfire policy SkipToNext
    And the device slept for 5 hours
    When the device resumes
    Then no catch-up JobDue is published
    And the next due time is the next 30-minute boundary

  Scenario: Crash after firing does not fire twice
    Given a once job due at 09:00 has fired
    And the app is killed before the in-memory state is cleared
    When the app restarts at 09:01
    Then no JobDue is published for that occurrence

  Scenario: Lease takeover after silence
    Given a singleton job whose lease is held by device "laptop" with ttl 10 minutes
    And "laptop" has not renewed for 21 minutes
    When device "desktop" ticks
    Then "desktop" acquires the lease and runs the job
```

## Design notes

- **Crates:** create `scheduler-domain`, `scheduler-application`, and `scheduler-infrastructure`. The desktop and headless roots start the runner.
- **`kernel-time`** ([overview](../architecture/overview.md#bounded-contexts-and-crates)): create it here. It holds civil dates, `Recurrence`, `ZonePolicy`, and DST rules on `jiff`, and stays pure. Time zones are passed in as values: `scheduler-infrastructure` reads the system tz database and the current zone and hands them in. Verify that `jiff`'s license passes `deny.toml`. Do not depend on chrono and jiff together.
- **arch.toml:** add `[crates.kernel-time] allow = ["kernel"]` with the domain bans, and `[crates.scheduler-domain] allow = ["kernel", "kernel-time"]` with the standard domain bans (`tokio`, `reqwest`, `rusqlite`, `serde_json`, `tauri`, `windows`, `uuid`). `scheduler-application` allows `scheduler-domain`, `kernel`, `kernel-time`, and `platform-ports`, with the same bans. `scheduler-infrastructure` allows `scheduler-application`, `scheduler-domain`, `kernel`, `kernel-time`, and `store`.
- **Migrations** (owned by `scheduler-infrastructure`): `scheduler_jobs` (device-local, rebuilt by owning contexts on start through `upsert`), `scheduler_runs` (`job_id`, `occurrence_utc`, `fired_at`; unique pair; pruned after 30 days), and `scheduler_leases` (declared as a synced entity; replication is wired by #069).
- **Time sources:** the runner listens to `PowerEvents` (resume, unlock) and to a new `SystemTimeEvents` port (`WM_TIMECHANGE`, time-zone change). Implement both in `platform-windows`; fakes go in tests.
- **Privacy:** jobs hold ids and schedules, not content. Data class: none (references only).
- **Budgets:** p95 fire accuracy is ±1 s while awake. Idle cost is one wakeup per minute at most. `next_due` over 10,000 jobs takes under 5 ms (bench).
- **Testing:** use proptest for `next_after` (monotonic, matches a brute-force minute-by-minute expansion over 2 years in 20 zones) and for DST gap and overlap handling. Acceptance runs in `features/core/scheduler.feature` against `headless` with a fake clock and fake power events.

## QA procedure

1. Build a dev `headless` and set `QA_SCHEDULER_DEMO=1` (registers a demo job every 2 minutes with FireOnceLate). Observe a `JobDue` line in the redacted log every 2 minutes.
2. Put the laptop to sleep (Start → Power → Sleep) for 7 minutes, then wake it. Within 5 s of unlocking, observe one `JobDue` with `missed_count` 3, not three separate fires.
3. Settings → Time & language → Date & time: turn off automatic time zone and switch from your zone to "(UTC-05:00) Eastern Time". Observe that the next due time is recomputed in the log within 5 s.
4. Kill the process in Task Manager right after a fire and restart it. Observe no duplicate fire for that occurrence.
5. Reboot. After sign-in, observe that the job resumes and catch-up applies to the time the PC was off.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
