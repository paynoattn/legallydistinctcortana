# 012. Global hotkeys with key up/down (push-to-talk, toggle) and bindings UI

| Milestone    | Size | Depends on | Labels                                  |
| ------------ | ---- | ---------- | --------------------------------------- |
| M1 Dictation | M    | #009, #092 | area/platform, area/voice, type/feature |

## Status

**Bindings UI implemented** on branch `feat/overlay-tray-settings` (2026-11-18), on `tauri-plugin-global-shortcut` (`RegisterHotKey`), not a low-level keyboard hook. It reports key down and key up, so hold-to-dictate works. It differs from the design below:

- `settings_domain::Chord` parses and displays chords ("Ctrl+Alt+R", "F9"), refuses reserved Windows chords (Win+L, Ctrl+Alt+Delete, and others) and bare keys other than F-keys, Pause, Scroll Lock, Insert, and Print Screen. The two bindings may not be the same.
- Bindings are `dictateShortcut` (default F9) and `readAloudShortcut` (default Ctrl+Alt+R). There is no assistant binding or toggle mode yet.
- The settings window records the next chord (shortcuts are suspended while it records), shows conflicts inline, and resets to the default. A chord another app holds is refused with "… is taken by another app." and nothing is saved.
- The tray's "Pause shortcuts" unregisters both until it is chosen again.
- QA: [`qa/procedures/tray-and-settings.md`](../../qa/procedures/tray-and-settings.md).

## Story

As a writer, I want to hold a key anywhere in Windows to dictate and release it to stop, and to choose my own keys, so that dictation works in every app without reaching for the mouse.

## Context

[OS integration: `HotkeySource`](../architecture/os-integration.md): a low-level keyboard hook (`SetWindowsHookExW(WH_KEYBOARD_LL)`) on a dedicated thread with its own message loop, because push-to-talk needs key up as well as key down and `RegisterHotKey` reports presses only. The callback posts to a channel and returns in under 1 ms, or Windows removes the hook. Defaults: hold Right Ctrl (dictate), Win+Alt+Space (assistant), Ctrl+Alt+R (read selection). [MVP risks](../product/mvp-plan.md#risks): some antivirus flags low-level hooks; `RegisterHotKey` is the fallback for toggle mode.

## Scope

- `platform-ports`: `HotkeySource` port. It takes a set of `Binding { action, chord, mode: Hold | Toggle | Press }` and yields `HotkeyEvent { action, phase: Down | Up, at: Timestamp }`.
- `platform-windows`: the hook adapter (dedicated thread, message loop, channel, key-repeat suppression, modifier tracking), plus a `RegisterHotKey` adapter used when the hook cannot be installed.
- `voice-domain`: `Chord` parsing and display ("Right Ctrl", "Win+Alt+Space"), conflict detection between our bindings, and a list of reserved Windows chords we refuse (for example Win+L, Ctrl+Alt+Del, Win+D).
- Bindings are device-local settings (#010): `hotkeys.dictate`, `hotkeys.assistant`, `hotkeys.read_selection`, `hotkeys.dictate_mode` (hold or toggle).
- `src/settings` → Hotkeys section: shows each binding, "Change" records the next chord, shows conflicts inline, "Reset to default".
- Tray menu "Pause hotkeys" (from #009) unregisters everything until resumed.

Out of scope: what the actions do (#019 dictation, #028 read selection, #051 assistant), the Esc stop key during playback (#030).

## Acceptance criteria

`features/core/hotkeys.feature` against `headless` with a fake `HotkeySource`, `features/ui/hotkeys.feature` with fake IPC, and the hook itself in the QA procedure.

```gherkin
Feature: Global hotkeys

  Scenario: Hold mode reports down and up once each
    Given "dictate" is bound to Right Ctrl in hold mode
    When Right Ctrl is held for 2 seconds with auto-repeat
    Then exactly one Down and one Up event reach the dictation action

  Scenario: Toggle mode starts and stops on presses
    Given "dictate" is in toggle mode
    When Right Ctrl is pressed and released twice
    Then the action receives Start then Stop

  Scenario: A conflicting binding is refused in the UI
    Given "assistant" is bound to Win+Alt+Space
    When I record Win+Alt+Space for "read selection"
    Then I see "Already used by Assistant" and the binding is not saved

  Scenario: Reserved Windows chords are refused
    When I record Win+L for "dictate"
    Then I see "Windows reserves this shortcut"

  Scenario: The hook cannot be installed
    Given SetWindowsHookExW fails
    When hotkeys start
    Then toggle bindings work through RegisterHotKey
    And Settings shows "Hold-to-talk is unavailable: another program blocked the keyboard hook"

  Scenario: Key-up is lost after the session locks
    Given "dictate" is held down
    When the workstation locks before key-up arrives
    Then the action receives Up with reason "session locked"

  Scenario: Hotkeys are paused from the tray
    When I choose "Pause hotkeys"
    Then pressing Right Ctrl starts nothing until I choose "Resume hotkeys"
```

## Design notes

- `arch.toml`: `platform-ports` allow `["kernel"]`; `platform-windows` allow `["platform-ports", "kernel"]` and is the only crate that may depend on `windows`; `voice-domain` allow `["kernel"]`.
- Whichever of #006 and #012 lands first creates `platform-ports` and `platform-windows`. The hook's Win32 calls are `unsafe` FFI in `platform-windows/src/ffi/` under [ADR 0012](../architecture/adr/0012-unsafe-and-ffi.md) (#092); the hook thread, chord matching, and repeat suppression are safe, tested code.
- The hook callback must not allocate, lock, or log. It writes a fixed-size event into a lock-free channel and calls `CallNextHookEx`. Measure the callback in a debug build; the Windows `LowLevelHooksTimeout` limit removes slow hooks without telling us (verify the current limit on Windows 11). Re-install the hook if it is silently removed: a health check compares expected and observed events after resume.
- Chords with Win are swallowed so Windows does not open the Start menu; a bare Right Ctrl hold is passed through so other apps still see Ctrl.
- Elevated windows: a non-elevated process's hook may not see keys typed into an elevated window (UIPI; verify). The dictation path handles elevated targets in #020.
- `cfg(windows)` appears only in `platform-windows` and the composition roots.
- Budget: hook callback under 1 ms; hotkey down to action under 5 ms (the overlay's 50 ms budget starts here).

## QA procedure

1. Run the dev build. In Notepad, hold Right Ctrl for 3 seconds. The debug log shows one Down and one Up for "dictate".
2. Switch the mode to toggle in Settings → Hotkeys. Tap Right Ctrl twice. The log shows Start then Stop.
3. Rebind "read selection" to Win+Alt+Space. The conflict message appears and nothing is saved.
4. Press Win+Alt+Space in Edge. The Start menu does not open, and the log shows the assistant action.
5. Hold Right Ctrl and press Win+L. Unlock. The log shows Up with reason "session locked".
6. Choose "Pause hotkeys" in the tray. Right Ctrl does nothing. Resume. It works again.
7. With Windows Defender on, run the app for an hour of normal typing. Typing never lags and the hook is still active.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
