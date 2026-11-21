# 095. Focus reader: read aloud with the spoken word lit

| Milestone    | Size | Depends on | Labels                             |
| ------------ | ---- | ---------- | ---------------------------------- |
| M2 Voice out | M    | #028       | area/ui, area/speech, type/feature |

## Story

As someone who reads along while listening, I want the read-aloud shortcut to open a reader that lights each word as the voice says it, so that I can follow the text without losing my place.

## Context

Design: [artboard 04, focus reader](../design/artboards/04-focus-reader.html) and its section in the [artboards README](../design/artboards/README.md#focus-reader). Today the shortcut opens a small mood window next to the cursor (#028 MVP slice), and reading starts only after a mood is picked. This issue replaces that window with the reader. The reading pill (#030) stays the plan for reading without a window and is not built here.

Wyoming Piper returns audio only, with no word or phoneme times. Speech is already synthesized one sentence at a time (`ReadingText::sentences`) and queued on the speakers (`AudioOutput`), so the core can say exactly when each sentence will start and how long its speech lasts. The UI spreads that time over the sentence's words by weight and snaps back to the true time at every sentence.

## Scope

- **Core (`speech-domain`, `speech-application`):**
  - The captured selection is offered to the window as its full list of sentences (the same split the voice uses), not only a 160-character preview.
  - A reading can start from a given sentence index (default 0). Sentences before it are not synthesized.
  - As each sentence is queued, the reading reports `{ reading, index, startsInMs, speechMs }`: how long until the sentence starts playing (the audio already queued ahead of it), and the length of its speech without the trailing pause the mood adds. Reports from a stale reading are dropped, as status reports are today (`Readings`).
  - `AudioOutput::enqueue` (or a sibling method) tells the caller how much audio is queued ahead of the new clip. `SpeakerOutput` computes it from its sample queue and device rate.
- **Desktop (`crates/desktop`, wiring only):** the selection payload carries the sentences; `read_aloud` takes an optional `fromSentence`; a `read-aloud:sentence` event carries the timing report. The `read-aloud` window is sized to 860 × 660 logical px and centred on the monitor under the cursor when shown, at runtime (do not edit `tauri.conf.json`), and no longer hides when it loses focus. No new commands, so `capabilities/` is unchanged.
- **UI (`src/read-aloud`):**
  - The window is the reader from the artboard: title row (small mark, "Read aloud", Close), the text as paragraphs of sentences of words, and a control bar (previous sentence, play/pause, next sentence, mood, voice name, status line, a one-line keyboard hint).
  - Reading starts as soon as the selection arrives, in the last mood and voice used (both remembered in local storage; the mood defaults to Neutral).
  - Word timing: `domain` holds a pure function that spreads `speechMs` over a sentence's words by weight (estimated syllables from vowel groups minus a silent final e, digits at 1.4 each, plus 1.3 after `, ; : — –`, plus 0.25 per word) and returns each word's start offset. `application` turns sentence reports and a clock port into "sentence i, word j is lit"; the UI subscribes with `useSyncExternalStore`, driven by `requestAnimationFrame` through an infrastructure adapter.
  - Colors on Dusk: the lit word is Dusk on a Glow light; the sentence being read is Paper; sentences ahead are Mist-dim; sentences already read are `#9095C2`. The light glides between words on a line and jumps to a new line. Reduced motion: no glide.
  - Follow: the lit line is kept about a third of the way down. A wheel or scrollbar scroll stops following and shows "Follow the voice", which resumes it.
  - Clicking a word, Previous, and Next start a new reading from that sentence (`read_aloud` with `fromSentence`). Pause stops the reading and remembers the sentence; Resume reads from the start of that sentence. Choosing a mood (button menu, or keys 1–9) or a voice (the voice button opens the picker) restarts from the current sentence. The voice button reads "Default voice" when the server default is in use.
  - Status line: "Sentence 3 of 12." while reading, "Paused. Sentence 3 of 12." when paused, "Finished reading." at the end, and the failure message when the voice fails. A thin Glow progress line shows sentences done; it turns Mist-dim when finished.
  - Keys: Space pauses and resumes, ← and → move by sentence, 1–9 pick a mood, Ctrl with + or − sizes the text between 20 and 40 px, Esc stops and closes (focus goes back to the previous app, as `dismiss_read_aloud` does today).
  - Nothing selected, or a failed capture: the reader shows the message and a Close button, as today.

Out of scope: speed control (needs rate support in the synthesizer, #025/#027), the reading pill (#030), real word timestamps from an engine that reports alignments, lazy rendering for selections near the 20,000-character cap.

### Decisions (from the specifier's questions)

- **Voice:** the voice name in the control bar is a button that opens the existing voice picker (`VoicePicker`) as a popover; its scenarios stay. A picked voice restarts the current sentence, like a mood.
- **Title row:** "Read aloud" plus Close. "Reading from …" waits until the source app or site is captured.
- **Paragraphs:** the core sends each sentence with whether it starts a paragraph (it follows a line break in the selection). The first sentence always does.
- **Corner overlay:** unchanged in this issue.
- **Reduced motion:** the system preference (`prefers-reduced-motion`), as the overlay uses. CSS only.
- **Report timing:** a sentence is reported when it is queued, with `startsInMs`; not when it starts playing.
- **Edges:** while paused, ← goes back one sentence. Otherwise ← restarts the current sentence when more than 1.5 s of it has played, otherwise goes back one (at sentence 1 it restarts sentence 1); → at the last sentence does nothing. While paused, a word click, an arrow, a mood or a voice starts reading from the chosen sentence. After "Finished reading.", Space or Play reads again from sentence 1; a word click reads from its sentence. The Close button does what Esc does. Pressing the shortcut while the reader is open captures the new selection and starts a new reading in it.
- **Text size:** starts at 25 px, steps of 3 px, clamped to 20–40 px, remembered in local storage. Matching on `event.key` (`+`, `=`, `-`) covers the numpad.
- **Cap:** 20,000 characters (`MAX_READING_CHARS`), unchanged.

## Acceptance criteria

`features/ui/focus-reader.feature` (replaces the mood-first scenarios of `features/ui/read-aloud.feature`) with fake IPC and a fake clock; Rust unit tests in `speech-application` and `speech-infrastructure` for sentence reports and start offsets.

```gherkin
Feature: Focus reader

  Scenario: Reading starts when the selection arrives
    Given the last mood was "Calm"
    When the text "Good morning. Two emails need you." is selected
    Then reading starts from sentence 1 in the "Calm" mood
    And I see 2 sentences
    And the status says "Sentence 1 of 2."

  Scenario: The spoken word is lit from the sentence timing
    Given the text "Good morning. Two emails need you." is selected
    When sentence 2 starts now with 1200 ms of speech
    And 700 ms pass
    Then the word "need" is lit
    And sentence 1 is shown as read and sentence 2 as being read

  Scenario: Clicking a word reads from its sentence
    Given a reading is under way at sentence 1 of 3
    When I click a word in sentence 3
    Then reading starts again from sentence 3 in the same mood

  Scenario: Pause and resume
    Given a reading is under way at sentence 2 of 3
    When I press "Space"
    Then the reading is stopped and the status says "Paused. Sentence 2 of 3."
    When I press "Space"
    Then reading starts again from sentence 2

  Scenario: Changing the mood restarts the current sentence
    Given a reading is under way at sentence 2 of 3 in the "Neutral" mood
    When I press "5"
    Then reading starts again from sentence 2 in the "Urgent" mood
    And "Urgent" is remembered for the next reading

  Scenario: Scrolling stops following
    Given a long reading is under way
    When I scroll the text with the wheel
    Then I see "Follow the voice"
    When I click "Follow the voice"
    Then the lit line is back in view

  Scenario: The end of the reading
    When the reading finishes
    Then the status says "Finished reading." and no word is lit

  Scenario: Esc stops and closes
    Given a reading is under way
    When I press "Escape"
    Then the reading is stopped and the window is dismissed

  Scenario: Nothing selected
    Given the text "   " is selected
    Then I see the alert "Select some text first, then press the read-aloud shortcut."
    And nothing is read
```

Core (unit tests):

- A three-sentence reading started from sentence 1 synthesizes only sentences 2 and 3, and reports indexes 1 and 2.
- With 2 s of audio already queued, the next report has `startsInMs` 2000; each report's `speechMs` is the clip length without the mood pause.
- A report from a reading that was replaced is not delivered.

## Design notes

- `arch.toml`: no new crates. `speech-domain` gains nothing UI-shaped; the weight function is TypeScript, in `src/read-aloud/domain`.
- The UI time base is the arrival time of each report plus `startsInMs`. IPC latency (a few ms) is below what the eye notices on a word.
- Bundle: the `read-aloud` window budget (80 KB gzip) holds; no new dependencies, inline SVG icons only.
- Accessibility: the text is a normal document, so screen readers can read it; the lit word is not announced (`aria-live="off"` on the text), the status line is the live region. Focus starts on Play/Pause. Contrast: lit word 8.7:1, current sentence 13.4:1, ahead 6.5:1, read 5.2:1.
- Privacy: unchanged. The text already goes to `VOICE_HOST`; nothing new leaves the device, and the sentences are held only in the window's memory.
- One light: the corner overlay stays as it is today during read-aloud; the reader's own mark is the speaking light.

## QA procedure

1. In Edge, select three paragraphs of an article and press Ctrl+Alt+R. The reader opens centred on that monitor and starts reading within half a second, in the last mood.
2. Watch the lit word for a minute. It stays on the spoken word or one either side, and is back in step at each new sentence.
3. Press Space, then Space again. Reading pauses, then continues from the start of the same sentence.
4. Press →, then ←. Reading moves forward one sentence, then back.
5. Click a word two paragraphs down. Reading continues from that sentence.
6. Press 2. From the current sentence, the voice is calm, and the mood button says Calm.
7. Scroll up with the wheel. "Follow the voice" appears; click it and the lit line is back in view.
8. Click the browser. The reader stays open and keeps reading.
9. Press Esc in the reader. Reading stops, the reader closes, and focus is back in Edge.
10. Press Ctrl+Alt+R with nothing selected. The reader shows "Select some text first".

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed (issue 028 status, os-integration.md read-aloud entry point)
