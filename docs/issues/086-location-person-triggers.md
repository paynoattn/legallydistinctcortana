# 086. Location and person triggers for reminders

| Milestone | Size    | Depends on             | Labels                                 |
| --------- | ------- | ---------------------- | -------------------------------------- |
| Post-MVP  | unsized | #032, #041, #042, #071 | area/reminders, type/feature, post-mvp |

## Story

As a busy person, I want to say "remind me to drop off the parcel when I get to work" or "next time I talk to Sam, ask about the invoice", so that the reminder appears at the moment it is useful rather than at a guessed time.

## Context

The [vision](../product/vision.md#proactive) lists reminders by event and by person, as Cortana had. The [MVP plan](../product/mvp-plan.md#scope-moscow) lists location and person triggers as "Could" (v0.2). [connectors](../architecture/connectors.md#reminders-and-tasks): our local copy carries schedule extensions a provider cannot express (location, person triggers).

## Scope

- **Location triggers:** named places (Home, Work, custom) defined by Wi-Fi network name or by a geofence from Windows location services (`Windows.Devices.Geolocation`, with the user's permission). "Arrive" and "leave" events. Evaluated on the device where the user is (presence from [#071](071-presence-single-delivery.md)).
- **Person triggers:** a person in the memory notebook with their addresses; fires when a calendar event with them is about to start, when a mail from them arrives, or when the user opens a mail thread with them. Teams and other chat apps are out of scope unless an MCP server provides the signal.
- Natural-language parsing extensions in `reminders-domain` for "when I get to …", "when I leave …", "next time I talk to/see …".
- UI: trigger editor in the reminders page; places in settings.

Out of scope: continuous GPS tracking; phone location.

## Acceptance criteria

```gherkin
Feature: Location and person triggers for reminders

Scenario: Arrive at work by Wi-Fi
  Given the place "Work" is defined by the Wi-Fi network "Contoso-Corp"
  And a reminder "Drop off the parcel" triggers on arriving at Work
  When my laptop connects to "Contoso-Corp"
  Then the reminder is delivered once on my laptop

Scenario: Next time I talk to Sam
  Given "Sam" with address sam@example.com is in my notebook
  And a reminder "Ask about the invoice" triggers on Sam
  When a meeting with sam@example.com starts in 10 minutes
  Then the heads-up includes "Ask about the invoice"
```

## Design notes

- Places and location events carry `DataClass::Location` ([assistant: privacy policy](../architecture/assistant.md#model-router-models-context)), local only by default; never sync raw coordinates, only "arrived at <place>" events. Reminder content keeps `DataClass::Reminders`.
- New `platform-ports` port `PlaceSignals` (Wi-Fi network changes, geolocation) implemented in `platform-windows`.
- Mail-based person triggers read headers only and run in the existing triage path, with no tools ([ADR 0009](../architecture/adr/0009-untrusted-content-and-tool-permissions.md)).

## QA procedure

1. Define Work by your office Wi-Fi, create the reminder, connect to that network, and observe one toast.
2. Add a test contact to the notebook, create a person reminder, schedule a meeting with that address, and observe the heads-up text.
