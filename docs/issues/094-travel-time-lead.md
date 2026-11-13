# 094. Travel-time lead for calendar heads-ups (Should)

| Milestone            | Size | Depends on | Labels                                    |
| -------------------- | ---- | ---------- | ----------------------------------------- |
| M4 Calendar and mail | S    | #041       | area/calendar, area/privacy, type/feature |

## Story

As a knowledge worker with in-person meetings, I want the heads-up to say "Leave by 9:20 for the dentist (25 min drive)" instead of a fixed 10 minutes, using a routing service I chose, so that I leave on time without handing my location to a service I did not pick.

## Context

The calendar heads-up fires at event start minus (travel time + prep lead, default 10 min) ([assistant: proactive engine](../architecture/assistant.md#proactive-engine)). Travel time is opt-in ([connectors: calendar](../architecture/connectors.md#calendar)) and a Should for v0.1 ([mvp-plan scope](../product/mvp-plan.md#scope-moscow)). Locations are `DataClass::Location`. The privacy policy decides which providers may receive them, and the default is local only ([assistant: model router](../architecture/assistant.md#model-router-models-context)). #041 builds the fixed per-calendar lead. This issue adds travel time on top of it.

## Scope

- **Opt-in:** a per-calendar "Add travel time" toggle in Settings → Calendars (off by default). Turning it on asks for:
  - a **routing provider**: an OpenRouteService-compatible endpoint, either the hosted API with the user's key or a self-hosted instance URL. Verify the ORS API shape and terms for directions, geocoding, caching, and attribution.
  - **places**: Home and Work addresses, plus a per-weekday rule for "where I start" (for example Work on weekdays 9–17, otherwise Home).
  - a travel mode (driving, cycling, walking; transit only if the provider supports it, otherwise hidden) and a buffer (default 5 min).
- **Privacy gate:** before the first call, the router policy must allow `DataClass::Location` for that provider. A hosted provider is `Cloud`, and a LAN instance is `MyNetwork`, as in #047. If it is not allowed, the toggle explains why and links to Settings → Privacy. Nothing is sent until it is allowed.
- **Planning** (`calendar-domain`, pure):
  - `lead = travel + buffer + prep_lead`, capped at 3 h. The origin is the previous event's physical location if it ends within 2 h before, otherwise the place rule.
  - Skip travel for events without a physical location, such as online-only events with a join link, empty locations, or "Microsoft Teams Meeting".
- **Lookups:**
  - Geocode each distinct location string once and cache the result (device-local, 90 days).
  - Ask for a route estimate at most twice per event: when the heads-up is first planned (or 3 h before start, whichever is later), and once 45 min before the planned heads-up. Reschedule the heads-up job if the lead changed.
- **Fallback:** offline, provider error, rate limit, ungeocodable location, or a policy refusal → the fixed per-calendar lead from #041. The heads-up says "travel time unavailable".
- **Heads-up text:** "Leave by 9:20 for Dentist (25 min drive)", passed to notify (#033) and the toast (#034).

Out of scope: live GPS or Windows location services, traffic beyond what the provider returns, other routing adapters (Valhalla, OSRM, and commercial APIs are follow-ups behind the same port), and maps in the UI.

## Acceptance criteria

```gherkin
Feature: Travel-time lead

  Scenario: Heads-up includes travel time
    Given travel time is on for my personal calendar with a 5 minute buffer and 10 minute prep lead
    And Location is allowed for my routing provider
    And an event "Dentist" at 10:00 at "Hauptstraße 5, Berlin" and I start from Home
    When the provider estimates a 25 minute drive
    Then the heads-up is scheduled at 09:20
    And its text is "Leave by 9:20 for Dentist (25 min drive)"

  Scenario: Location not allowed by the privacy policy
    Given Location is local only
    When I turn on travel time with a hosted routing provider
    Then no request is sent to the provider
    And the setting shows "Location is set to local only" with a link to Settings → Privacy

  Scenario: Online meeting skips travel
    Given an event with a Teams join link and location "Microsoft Teams Meeting"
    Then no geocode or route request is made and the fixed 10 minute lead applies

  Scenario: Provider unavailable falls back to the fixed lead
    Given the routing provider answers 429
    When the heads-up is planned for a 10:00 event
    Then the heads-up is at 09:50 and says "travel time unavailable"

  Scenario: Re-estimate moves the heads-up
    Given a heads-up planned at 09:20 from a 25 minute estimate
    When the re-estimate 45 minutes before returns 40 minutes
    Then the heads-up job moves to 09:05

  Scenario: Origin is the previous meeting
    Given an event at "Office B" ending at 13:00 and the next event at 14:00 at "Client site"
    Then the route for the 14:00 event starts at "Office B"

  Scenario: Every lookup is audited without content
    When a route estimate is requested from the hosted provider
    Then an audit record with provider, task "travel-time", class Location, and byte counts is written
    And it contains no address
```

## Design notes

- **Crates:**
  - `calendar-domain`: lead computation, origin choice, the "physical location" check, and place rules.
  - `calendar-application`: the `RouteEstimator` and `Geocoder` ports, and re-planning heads-up jobs through the scheduler port.
  - New `connector-routing`: the ORS-compatible adapter on `reqwest` + rustls, with its API key from the credential vault (#040).
  - Provider location and policy checks reuse the `models-domain` policy types from #047. Travel lookups are not LLM calls, so verify with the lead whether they go through the router or a shared policy check (proposal: a shared policy check, plus an audit through `kernel::AuditSink`).
- **arch.toml:** `connector-routing` allows `calendar-application`, `calendar-domain`, `kernel`, `kernel-time`, `accounts-application`, and `store`.
- **Migrations:** `calendar_places` (synced: Home, Work, rules; `DataClass::Location`) and `calendar_geocode_cache` (device-local, 90 days).
- **Privacy:** `DataClass::Location`. The request carries coordinates and mode only, never the event title. Logs never contain addresses.
- **Budget:** at most 2 route calls and 1 geocode per distinct location, per event. Planning is pure and under 1 ms.

## QA procedure

Setup: QA-GOOG or QA-MSA calendar connected (#041). An OpenRouteService API key (QA-ORS), or a self-hosted ORS instance on the LAN.

1. Settings → Calendars → personal → turn on "Add travel time". Enter Home and Work addresses and the QA-ORS key. Observe the Location privacy notice. Allow Location for this provider in Settings → Privacy.
2. Create an event 90 minutes from now at a real address about 20 minutes away by car. Observe the planned heads-up time in the calendar settings' "Upcoming alerts" list, earlier than the fixed lead.
3. At the heads-up, observe "Leave by <time> for <event> (<n> min drive)".
4. Create an online-only Teams event. Observe the fixed lead and no lookup in Settings → Privacy → Off-device log.
5. Enter an invalid key. Create an event. Observe the fixed lead and "travel time unavailable".
6. Set Location back to local only. Observe that the toggle explains why travel time is off, and no new audit entries appear.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed (connectors.md lists the routing connector)
