# 028. Read selection by hotkey with mood picker

| Milestone    | Size | Depends on | Labels                                   |
| ------------ | ---- | ---------- | ---------------------------------------- |
| M2 Voice out | M    | #012, #027 | area/speech, area/platform, type/feature |

## Story

As an accessibility user, I want to select text in any app, press one shortcut, and hear it read, or hold Shift to choose the mood first, so that I can listen to anything on my screen without copying it somewhere else.

## Context

[OS integration: read aloud](../architecture/os-integration.md#read-aloud-from-the-right-click-menu), entry point 2: Win+Alt+R reads the current selection in any app through `SelectionReader`; holding Shift opens the mood picker at the cursor. `SelectionReader` uses UI Automation `TextPattern.GetSelection()` on the focused element, and falls back to saving the clipboard, sending Ctrl+C, reading, and restoring. It never leaves the clipboard changed. M2 exit demo: hotkey read in Word.

## Scope

- `platform-ports`: `SelectionReader` (`read() -> Selection { text, source_app, method: Uia | Clipboard } | NoSelection | Blocked(reason)`).
- `platform-windows`: UIA adapter (focused element → `TextPattern` → selection ranges → text), and the clipboard fallback with the same snapshot and restore rules as #020 (sequence-number check, history exclusion).
- Password fields are never read: if the focused element reports `IsPassword`, return `Blocked(PasswordField)` without trying the fallback.
- `speech-application`: `ReadSelection { mood: Option<Mood> }` use case: read the selection, cap it at 50,000 characters, and enqueue `Speak { source: ReadAloud }` (#026).
- Hotkeys: `hotkeys.read_selection` (default Win+Alt+R) reads in the default read-aloud mood; Shift+Win+Alt+R opens the mood picker.
- Mood picker: shown in the `panel` window near the cursor (40 KB entry budget), keyboard-first: digits 1–9 pick a mood, arrows and Enter work, Esc cancels. The picker takes focus while open, so the selection is read before it is shown.

Out of scope: the browser context menu (#029), Explorer files (#058), "Summarize and read" (assistant, after #049), the reading pill (#030).

## Acceptance criteria

`features/core/read-selection.feature` with a fake `SelectionReader`; `features/ui/mood-picker.feature` with fake IPC.

```gherkin
Feature: Read selection by hotkey

  Scenario: Read the selection in the default mood
    Given "The quarterly numbers are in." is selected in Word
    When Win+Alt+R is pressed
    Then speak is called with that text, mood Neutral, and source ReadAloud

  Scenario: Choose a mood first
    Given text is selected in Edge
    When Shift+Win+Alt+R is pressed and I press 6 in the picker
    Then the text is read in mood Angry and the picker closes

  Scenario: Nothing is selected
    Given no text is selected
    When Win+Alt+R is pressed
    Then nothing is spoken and the overlay says "Select some text first"

  Scenario: Password fields are never read
    Given the focus is in a password box with text selected
    When Win+Alt+R is pressed
    Then nothing is read, no Ctrl+C is sent, and the overlay says "Can't read password fields"

  Scenario: The clipboard fallback leaves the clipboard unchanged
    Given an app without UIA text support has "hello there" selected
    And the clipboard holds "original"
    When Win+Alt+R is pressed
    Then "hello there" is read and the clipboard holds "original" afterwards

  Scenario: Elevated app
    Given the selection is in an app running as administrator
    When Win+Alt+R is pressed
    Then the overlay says "Can't read from apps running as administrator"

  Scenario: Very long selection
    Given 80,000 characters are selected
    When Win+Alt+R is pressed
    Then the first 50,000 characters are read and the overlay says "Reading the first part"

  Scenario: Esc closes the picker without reading
    Given the mood picker is open
    When I press Esc
    Then nothing is spoken and focus returns to the previous app
```

## Design notes

- `arch.toml`: no new crates. `speech-application` gains the use case and depends on `platform-ports` for `SelectionReader`.
- UIA calls run on a dedicated COM (MTA) thread in `platform-windows` with a 300 ms timeout per call; some apps hang UIA (verify with Electron apps and old Win32 apps). On timeout, use the clipboard fallback.
- UIPI: a non-elevated process cannot send Ctrl+C to an elevated window and may not get UIA text from it (verify), so detect the integrity level first, as in #020.
- The panel window must return focus to the previous foreground window on close (store its HWND before showing).
- Data class: selected text is `DataClass::Screen`. It stays on the device, is never logged, and is held only until spoken.
- Budget: hotkey → first audio under 400 ms (UIA read under 100 ms plus the 300 ms TTS budget).

## QA procedure

1. In Word, select a paragraph and press Win+Alt+R. It is read within half a second, and the clipboard is unchanged.
2. In Edge, select a sentence, press Shift+Win+Alt+R, and press 5 (Urgent). The picker closes and the sentence is read urgently. Focus is back in Edge.
3. Press Win+Alt+R with nothing selected. "Select some text first" appears.
4. On a sign-in page, select the password field's text and press Win+Alt+R. Nothing is read.
5. In Notepad++ (or another app where UIA lacks a text pattern), copy "original" first, then select other text and press Win+Alt+R. The selection is read; paste afterwards gives "original".
6. Open Notepad as administrator, select text, and press Win+Alt+R. The administrator message appears.
7. Open the picker and press Esc. Nothing is read.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
