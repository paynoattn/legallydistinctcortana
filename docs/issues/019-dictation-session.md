# 019. Dictation session, cleanup rules, dictionary, snippets

| Milestone    | Size | Depends on       | Labels                   |
| ------------ | ---- | ---------------- | ------------------------ |
| M1 Dictation | M    | #012, #014, #016 | area/voice, type/feature |

## Story

As a writer, I want to hold a key, speak, and get clean text with my own words spelled right and my snippets expanded, so that I can dictate without fixing the output afterwards.

## Context

[Voice: dictation pipeline](../architecture/voice-and-speech.md#dictation-pipeline): `voice-application::DictationSession` runs the state machine from `voice-domain`: Armed → Listening → Finalizing → Inserting → Done | Cancelled | Failed. Deterministic cleanup handles spoken punctuation, fillers, numbers and dates, the dictionary, and snippets. MVP outcome 2: clean text within 500 ms of release on a GPU, offline.

## Scope

- `voice-domain`:
  - `DictationSession` state machine with transitions for hotkey down/up, VAD end, Esc, errors, and timeouts. A hold shorter than 250 ms cancels. A session longer than 5 minutes finalizes automatically.
  - `CleanupPipeline` of pure rules, applied in order: spoken punctuation ("comma", "period", "new line", "new paragraph", "question mark"), filler removal ("um", "uh", "erm"; not "like"), number and date normalization ("twenty five" → "25" in plain contexts), dictionary replacements and casing, snippet expansion ("insert my address").
  - `Dictionary { entries: [{ heard, written, case_sensitive }] }` and `Snippet { trigger, text }` with validation (trigger 2–40 characters, text up to 2,000).
- `voice-application`: `DictationService` wiring hotkey events (#012), capture (#013), VAD (#014), and `SpeechToText` (#016), and ending in a `TextSink` port that #020 implements. It publishes `DictationStateChanged` (no text) and `DictationFinished` (with text, in-process only, see #011).
- Store tables `voice_dictionary`, `voice_snippets` (shared data, synced once #069 lands) with migrations.
- `src/dictation` UI in the main window: dictionary and snippets editors, and a "try it" box that runs the cleanup rules on typed text.

Out of scope: insertion into apps (#020), LLM cleanup (#022), translation (#023), history (#024), overlay visuals (#021).

## Acceptance criteria

`features/core/dictation.feature` against `headless` with fake hotkey, audio, and STT ports; `features/ui/dictionary.feature` with fake IPC.

```gherkin
Feature: Dictation session

  Scenario: Hold, speak, release
    Given STT will return "hello comma world period"
    When the dictate key is held for 2 s and released
    Then the TextSink receives "Hello, world."
    And the states were Armed, Listening, Finalizing, Inserting, Done

  Scenario: A tap is not a dictation
    When the dictate key is held for 120 ms
    Then the session is Cancelled and STT is not called

  Scenario: Esc cancels while finalizing
    Given the session is Finalizing
    When Esc is pressed
    Then the session is Cancelled and nothing reaches the TextSink

  Scenario: Fillers and dictionary
    Given the dictionary maps "tower e" to "Tauri"
    And STT will return "um so we use tower e uh for the shell"
    Then the TextSink receives "So we use Tauri for the shell"

  Scenario: Snippets expand
    Given the snippet "my address" is "1 Main St, Springfield"
    And STT will return "ship it to insert my address"
    Then the TextSink receives "Ship it to 1 Main St, Springfield"

  Scenario: STT fails
    Given STT fails with "model_missing"
    When a dictation finishes
    Then the session is Failed with that code and the state event carries the fix action

  Scenario: A new hotkey press during Inserting
    Given the previous session is Inserting
    When the dictate key is pressed again
    Then a new session is Armed only after the previous one is Done

  Scenario: Invalid snippet is refused in the UI
    When I save a snippet with an empty trigger
    Then I see "Trigger must be 2 to 40 characters" and nothing is saved
```

## Design notes

- `arch.toml`: `voice-domain` allow `["kernel"]`; `voice-application` allow `["voice-domain", "platform-ports", "settings-domain", "kernel"]` (its `SpeechToText` port is implemented by `infer-client`); `voice-infrastructure` allow `["voice-application", "voice-domain", "kernel", "store"]`. Usual bans.
- Every cleanup rule is a pure function with its own unit and property tests (idempotence: running the pipeline twice gives the same text).
- The dictionary also feeds whisper's `initial_prompt` (#016).
- Privacy: transcripts are `DataClass::Voice`, held in memory, wrapped in `kernel::Redacted`, and never logged. `DictationFinished` text is not forwarded to any webview (#011 filter).
- Budget share: cleanup rules under 5 ms for 15 s of speech, so the 500 ms release-to-insert budget goes to STT and insertion.
- English rules first; the rule set is keyed by language so others can be added.

## QA procedure

Uses a temporary "Dictate to the try-it box" debug mode until #020 lands.

1. Hold Right Ctrl, say "hello comma world period", and release. The try-it box shows "Hello, world."
2. Tap Right Ctrl quickly. Nothing happens.
3. Add the dictionary entry "tower e" → "Tauri". Dictate "we use tower e". The output says "Tauri".
4. Add a snippet "my address". Dictate "insert my address". The address appears.
5. Delete the model and dictate. The error with the "Open Models" fix appears in the main window.
6. Dictate for 20 seconds and press Esc while it is finishing. Nothing is produced.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
