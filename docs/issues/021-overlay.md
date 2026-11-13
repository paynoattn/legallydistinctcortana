# 021. Overlay window and animated mark states

| Milestone    | Size | Depends on | Labels                               |
| ------------ | ---- | ---------- | ------------------------------------ |
| M1 Dictation | M    | #004, #011 | area/ui, area/platform, type/feature |

## Story

As a dictating user, I want a small animated mark in the bottom-right corner that shows when the assistant is listening, thinking, done, speaking, or stuck, without ever stealing focus from the app I'm typing in, so that I always know what it is doing.

## Context

[Voice: overlay states](../architecture/voice-and-speech.md#overlay-states) and [OS integration: overlay window](../architecture/os-integration.md#overlay-window). A 160×160 px transparent, always-on-top, non-activating, click-through window, pre-created hidden, shown with `ShowWindow(SW_SHOWNOACTIVATE)` plus an IPC event within 50 ms of hotkey down. It follows the monitor with the cursor and is DPI-aware. [UI: accessibility](../architecture/ui.md#accessibility): state changes are announced through a live region mirrored to the main window.

## Scope

- `platform-ports`: `OverlayWindow` (`show_on_cursor_monitor()`, `hide()`, `set_click_through(bool)`).
- `platform-windows`: after Tauri creates the `overlay` window, set `WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_LAYERED | WS_EX_TOOLWINDOW`; position at the bottom-right of the cursor monitor's work area with a 16 px margin; reposition on DPI or monitor change; toggle `WS_EX_TRANSPARENT` off only in the Error state.
- `voice-domain`: `OverlayState` (`Hidden | Armed | Listening | Thinking | Inserted | Speaking | Error(reason, fix)`) derived from dictation, speech, and error events by a pure reducer, including timing (Inserted fades over 600 ms, Error stays until clicked or 6 s).
- `src/overlay` feature (entry `src/overlay-main.tsx`, 25 KB budget): the animated `Mark` driven by state and input/output level at 30 Hz with CSS transforms, opacity, and `requestAnimationFrame`; the Error state's one-line reason and a "Fix" link that sends the fix action to the main window.
- `prefers-reduced-motion` and the shared `app.reduce_motion` setting switch to opacity-only transitions.
- Optional sounds (Armed tick, Error tone), off by default, setting `voice.overlay_sounds`.
- A live region in the main window mirrors state changes ("Listening", "Inserted", "Error: no microphone") for screen readers.

Out of scope: the reading pill (#030), the final brand mark artwork (#005), partial captions (opt-in, follow-up to #017).

## Acceptance criteria

`features/ui/overlay.feature` (overlay entry with fake events) and `features/core/overlay-state.feature` (reducer in process):

```gherkin
Feature: Overlay

  Scenario: States follow a dictation
    Given the overlay page is open with "?ipc=fake"
    When the fake emits Armed, Listening, Thinking, and Inserted
    Then the mark shows each state in order and is hidden 600 ms after Inserted

  Scenario: Listening reacts to input level
    Given the overlay is Listening
    When InputLevel events of -50 dB and then -10 dB arrive
    Then the mark's scale is larger for -10 dB than for -50 dB

  Scenario: Error offers a fix
    When the fake emits Error "No microphone found" with fix "open_sound_settings"
    Then the overlay shows "No microphone found" and a "Fix" link
    And clicking "Fix" sends the fix action and hides the overlay

  Scenario: Reduced motion
    Given the setting "Reduce motion" is on
    When the overlay goes from Armed to Listening
    Then no transform animation runs, only opacity changes

  Scenario: Screen readers hear state changes
    Given the main window is open
    When the overlay becomes Listening
    Then the main window's live region announces "Listening"

  Scenario: The overlay never takes focus
    Given Notepad has keyboard focus
    When the overlay is shown and hidden 20 times
    Then Notepad still has keyboard focus
```

## Design notes

- `arch.toml`: no new crates. `voice-domain` holds the state reducer; `platform-windows` holds the window adapter.
- The overlay entry receives only state and level DTOs (#011 filter). No text from dictation reaches it except the one-line error reason.
- Capability `overlay.json` (from #004) grants events only, plus one command to post the fix action.
- Multi-monitor: choose the monitor from the cursor position at hotkey time (`MonitorFromPoint`), not the foreground window.
- Bundle: the overlay must stay under 25 KB gzip: no zod (hand-written guards), no router, no design-system components beyond `Mark`.
- Budget: hotkey down → overlay visible in 50 ms p95 (bench job, #077). Level rendering at 30 fps uses under 2% of one core.
- Full-screen games and presentations: hide the overlay while `FocusState` reports full-screen exclusive mode (verify detection with `SHQueryUserNotificationState`), but still run the dictation.

## QA procedure

1. Focus Notepad and hold Right Ctrl. The mark appears bottom-right within a blink, reacts to your voice, pulses after release, shows a check, and fades. Notepad keeps focus throughout.
2. Move the cursor to a second monitor at a different scale and dictate. The overlay appears on that monitor, sized correctly.
3. Unplug the microphone and dictate. The overlay shakes and shows "No microphone found". Click "Fix". Sound settings open.
4. Turn on "Reduce motion" (Windows or in-app). The overlay only fades.
5. Turn on Narrator and dictate. Narrator announces "Listening" and "Inserted".
6. Move the taskbar to the left and dictate. The overlay sits inside the work area, not over the taskbar.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
