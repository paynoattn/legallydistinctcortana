# 030. Reading pill: pause, stop, mood, speed

| Milestone    | Size | Depends on       | Labels                             |
| ------------ | ---- | ---------------- | ---------------------------------- |
| M2 Voice out | S    | #021, #026, #012 | area/ui, area/speech, type/feature |

## Story

As a listener, I want a small control next to the corner mark while text is read, so that I can pause, stop, change the mood, or speed up without opening the app.

## Context

[OS integration: read aloud](../architecture/os-integration.md#read-aloud-from-the-right-click-menu): during playback a small "reading" pill appears with the overlay: pause, stop, mood switch, speed. Esc stops. [UI: windows](../architecture/ui.md#windows-and-entry-points): the pill lives in the `overlay` entry (25 KB budget). The overlay is non-activating, so the pill must be clickable without taking focus.

## Scope

- `src/overlay`: a `ReadingPill` component shown while `SpeechStarted { source: ReadAloud | Briefing }` is active: Pause/Resume, Stop, a mood button cycling through a short list (Neutral, Calm, Cheerful, Alert, Urgent), a speed button (1.0×, 1.25×, 1.5×, 2.0×, 0.75×), and the source origin when known ("Reading from nytimes.com").
- The overlay window grows to fit the pill (for example 360×64 logical px plus the mark) and turns off `WS_EX_TRANSPARENT` while the pill is visible, keeping `WS_EX_NOACTIVATE` so clicks never steal focus (#021 adapter).
- Commands (overlay capability): `speech_pause`, `speech_resume`, `speech_stop`, `speech_set_mood`, `speech_set_rate`. Mood and speed changes apply from the next sentence.
- Esc: while a read-aloud plays, the hotkey hook watches Esc and stops playback. Esc is passed through to the focused app, not swallowed.
- The pill hides 1 s after `SpeechFinished`. Alerts and replies show the mark only, not the pill.

Out of scope: the mood picker at the cursor (#028), the full voice picker (#027), assistant reply controls (#051).

## Acceptance criteria

`features/ui/reading-pill.feature` (overlay entry, fake IPC) and `features/core/reading-controls.feature`:

```gherkin
Feature: Reading pill

  Scenario: The pill appears for read-aloud
    Given the overlay is open with "?ipc=fake"
    When the fake emits SpeechStarted with source ReadAloud and origin "example.com"
    Then the pill shows Pause, Stop, mood, speed, and "Reading from example.com"

  Scenario: No pill for alerts
    When the fake emits SpeechStarted with source Alert
    Then only the mark is shown

  Scenario: Pause and resume
    Given a read-aloud is playing
    When I click Pause
    Then speech_pause is called and the button becomes Resume

  Scenario: Speed applies from the next sentence
    Given sentence 3 of 10 is playing at 1.0×
    When I click speed once
    Then speech_set_rate(1.25) is called and sentence 4 is synthesized at 1.25×

  Scenario: Esc stops reading without being swallowed
    Given a read-aloud is playing and Notepad has focus
    When Esc is pressed
    Then playback stops and Notepad also receives the Esc key

  Scenario: Clicking the pill does not steal focus
    Given Word has focus and a read-aloud is playing
    When I click Pause on the pill
    Then Word still has focus

  Scenario: The pill hides after reading ends
    When the fake emits SpeechFinished
    Then the pill is gone after 1 second
```

## Design notes

- `arch.toml`: no new crates. Pill state is derived in `src/overlay/application` from events; the component has no logic beyond rendering and click handlers.
- Bundle: the pill must fit in the overlay's 25 KB gzip budget with the mark; use plain buttons and CSS, no icon library (inline SVG only).
- Accessibility: the overlay is not focusable, so every pill action also has a keyboard path: Esc (stop), the stop hotkey (#026), and a "Now reading" card in the main window with the same controls for screen-reader users.
- Capability `overlay.json` gains only the five speech commands; it is a protected file (human approval).
- Esc pass-through must be tested with apps that use Esc themselves (a browser's full-screen video, a dialog) to confirm both effects happen.

## QA procedure

1. In Edge, read a long article aloud through the extension (#029) or Ctrl+Alt+R. The pill appears next to the mark with the site name.
2. Click Pause, then Resume. Reading pauses and continues from the same place. The browser keeps focus.
3. Click speed twice. From the next sentence, reading is faster.
4. Click the mood button. From the next sentence, the mood changes.
5. Press Esc. Reading stops; if a dialog was open in the browser, it also closes.
6. Trigger a spoken alert (`POST /v1/speech/speak` with source Alert). Only the mark appears.
7. Turn on Narrator and open the main window during reading. The "Now reading" card offers the same controls.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed (os-integration.md overlay size while the pill is shown)
