# 020. Text injection with clipboard restore and target-aware formatting

| Milestone    | Size | Depends on | Labels                                  |
| ------------ | ---- | ---------- | --------------------------------------- |
| M1 Dictation | M    | #019       | area/platform, area/voice, type/feature |

## Story

As a user dictating into Notepad, VS Code, Teams, a browser, or a terminal, I want the text to land at my cursor formatted for that app, with my clipboard left as it was, so that dictation feels like typing.

## Context

[Voice: dictation pipeline](../architecture/voice-and-speech.md#dictation-pipeline) and [OS integration: `TextInjector`, `ForegroundApp`](../architecture/os-integration.md). Default insertion: save the clipboard, set the text, send Ctrl+V with `SendInput`, restore after 150 ms. `ValuePattern.SetValue` for simple fields, typed `SendInput` Unicode for short text in terminals. Elevated targets (UIPI) reject input from a non-elevated app: detect that and fall back to "Copied, press Ctrl+V". M1 exit demo: dictate into Notepad, VS Code, Teams, and a browser text area, offline.

## Scope

- `platform-ports`: `TextInjector` (`inject(text, method) -> InjectionOutcome { Inserted | CopiedOnly(reason) }`) and `ForegroundApp` (`current() -> { process_name, window_class, elevated }`).
- `platform-windows`:
  - Clipboard paste: snapshot every clipboard format that can be read, set Unicode text, `SendInput` Ctrl+V, restore after 150 ms only if `GetClipboardSequenceNumber` shows nobody else changed the clipboard meanwhile.
  - Mark our temporary clipboard content so it stays out of Windows clipboard history and cloud clipboard (verify the `ExcludeClipboardContentFromMonitorProcessing` and `CanIncludeInClipboardHistory` formats).
  - Unicode `SendInput` typing for terminals and text up to 200 characters where paste is unreliable.
  - UIPI detection: compare the foreground process's integrity level with ours.
- `voice-domain`: `TargetProfile` classification (`CodeEditor`, `Chat`, `Email`, `Terminal`, `Document`, `Unknown`) from process name and window class via an editable table, and formatting rules per profile: chat drops the trailing period; terminal has no auto-capitalization and no trailing newline; code editors keep identifier casing; email and documents keep full punctuation.
- `voice-application`: the `TextSink` implementation from #019 that picks the method and formatting, and publishes `TextInserted` or `TextCopiedOnly`.
- `src/dictation` UI: a Target apps table (process → profile) with overrides.

Out of scope: overlay check and error visuals (#021), reading selections (#028), UIA `ValuePattern` beyond simple single-line fields (later if needed).

## Acceptance criteria

`features/core/text-injection.feature` with fake `TextInjector` and `ForegroundApp`; the real adapter in the QA procedure and a Windows-only integration test.

```gherkin
Feature: Text injection

  Scenario: Paste and restore the clipboard
    Given the clipboard holds an image
    When "Hello, world." is injected into Notepad
    Then Notepad receives "Hello, world."
    And after 150 ms the clipboard holds the original image again

  Scenario: The user copies during the restore window
    Given an injection just pasted
    When another app sets the clipboard within 150 ms
    Then the clipboard is not overwritten by the restore

  Scenario: Chat apps get no trailing period
    Given the foreground app is "ms-teams.exe"
    When "See you at noon." is dictated
    Then "See you at noon" is inserted

  Scenario: Terminals are typed, not capitalized
    Given the foreground app is "WindowsTerminal.exe"
    When "git status" is dictated
    Then "git status" is typed with no capital letter and no newline

  Scenario: Elevated target window
    Given the foreground window runs as administrator and the app does not
    When text is injected
    Then the text is left on the clipboard
    And the outcome is CopiedOnly with "This window runs as administrator. Press Ctrl+V to paste."

  Scenario: No focused text field
    Given the foreground is the desktop with no window focused
    When text is injected
    Then the outcome is CopiedOnly with "No text field focused. Press Ctrl+V to paste."

  Scenario: Dictated text stays out of clipboard history
    Given Windows clipboard history is on
    When "secret plan" is injected and the clipboard restored
    Then Win+V history does not contain "secret plan"
```

## Design notes

- `arch.toml`: no new crates; `platform-windows` gains the injector and foreground adapters. `voice-application` uses `platform-ports` only.
- Clipboard snapshot limits: skip delayed-render formats and anything larger than 64 MB; if a format cannot be restored, keep what can and log the format id (never the content).
- `SendInput` with Ctrl+V must not fight a held Right Ctrl from push-to-talk: injection waits until key-up has been processed, and releases stray modifiers first (verify with Right Ctrl as the hotkey).
- Privacy: inserted text is `DataClass::Voice`, never logged. The clipboard snapshot lives in memory only and is dropped after restore.
- Security: "CopiedOnly" leaves dictated text on the clipboard by design; the overlay says so, so the user knows.
- Budget: injection under 50 ms from `TextSink` call to `SendInput` (excluding the 150 ms restore delay, which runs after insertion).

## QA procedure

On Windows 11 with the network unplugged:

1. Copy an image in Paint. Dictate "hello comma world period" into Notepad. "Hello, world." appears; paste in Paint again and the image is still there.
2. Dictate into VS Code, a Teams chat, an Edge text area, and Windows Terminal. Each gets the right formatting (no period in Teams, lowercase in Terminal).
3. Open Notepad as administrator and dictate. Nothing is typed, the overlay or main window says to press Ctrl+V, and Ctrl+V pastes the text.
4. Turn on clipboard history (Win+V), dictate a sentence, and open Win+V. The sentence is not in the history.
5. Click the desktop so no window has focus and dictate. The "No text field focused" message appears.
6. Add "notepad.exe → Chat" in Target apps and dictate into Notepad. The trailing period is dropped.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
