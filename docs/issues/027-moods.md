# 027. Moods: style vectors, prosody fallback, voice picker with previews

| Milestone    | Size | Depends on | Labels                             |
| ------------ | ---- | ---------- | ---------------------------------- |
| M2 Voice out | M    | #026       | area/speech, area/ui, type/feature |

## Story

As a user, I want to hear text calm, cheerful, alert, urgent, angry, or whispered, and to preview each mood in my chosen voice, so that read-aloud and alerts carry the right tone.

## Context

[Voice: moods](../architecture/voice-and-speech.md#moods): `kernel::Mood` = `Neutral | Calm | Cheerful | Alert | Urgent | Angry | Mad | Whisper | Deadpan`. `speech-domain` maps each mood to an engine-neutral style vector (`emotion`, `intensity`, `rate`, `pitch_semitones`, `energy_db`, `pause_scale`). Each engine adapter translates the vector into what it supports, and the UI marks each mood "native" or "approximated" for the current voice. Kokoro and Piper use the prosody fallback until the expressive model passes evaluation (#081). MVP outcome 3: hear selected text "in that mood".

## Scope

- `speech-domain`: `StyleVector` and `style_for(mood, intensity) -> StyleVector`, with a table tuned by ear and kept in code with tests. Example targets: Calm (rate 0.9, pitch −1, energy −3 dB, pauses 1.3), Urgent (rate 1.15, pitch +1, energy +3 dB, pauses 0.7), Mad (rate 1.2, pitch +2, energy +6 dB, pauses 0.6), Whisper (rate 0.9, energy −10 dB). `MoodSupport { Native | Approximated }` per engine.
- Prosody fallback in `infer-host`: rate through the engine's speed parameter; pitch and energy through a post-synthesis DSP stage (pitch shift without tempo change, gain with a limiter); pause scaling by stretching inter-sentence and comma silences.
- `speech-application`: `Speak` resolves mood → style vector → engine request; a per-source default mood setting (`speech.default_mood.read_aloud`, `.alert`, `.reply`).
- `src/speech` UI in the main window: a voice picker (installed voices, with language and license) and a mood grid; each mood has a Preview button playing a fixed sample sentence in that mood, and a "native" or "approximated" badge.
- Speed setting (0.5×–2.0×, in 0.1 steps) multiplies the mood's rate.

Out of scope: the expressive model (#060, #080, #081), the mood picker at the cursor (#028), the browser menu (#029).

## Acceptance criteria

`features/core/moods.feature` (style mapping and DSP with fixtures) and `features/ui/voice-picker.feature` (fake IPC):

```gherkin
Feature: Moods

  Scenario: Every mood maps to a valid style vector
    Then for each of the nine moods the style vector has rate in 0.5..2.0, pitch in -6..6, and energy in -12..6

  Scenario: Urgent is faster than Calm
    Given the same sentence synthesized with Kokoro in Calm and in Urgent
    Then the Urgent audio is at least 15% shorter

  Scenario: Whisper is quieter
    Given the same sentence in Neutral and Whisper
    Then the Whisper audio's RMS is at least 8 dB lower and it does not clip

  Scenario: Approximated moods are labeled
    Given the current voice is Kokoro "af_heart"
    When I open the voice picker
    Then every mood except Neutral shows "approximated"

  Scenario: Preview plays the mood
    When I press Preview on "Alert"
    Then speak is called with mood "alert", source ReadAloud, and the sample sentence

  Scenario: Speed multiplies the mood's rate
    Given speed 1.5 and mood Calm
    Then the requested rate is 1.35

  Scenario: Preview with no voice installed
    Given no TTS voice is installed
    When I open the voice picker
    Then I see "No voices installed" and a button to open Models
```

## Design notes

- `arch.toml`: no new crates. The style table and mapping are pure (`speech-domain`, depends on `kernel` for `Mood`). The DSP code lives in `infer-host` (or a pure `audio-dsp` library crate with allow `[]`, so it is covered and mutation-tested); prefer the library crate.
- Pitch shifting: implement a small, tested algorithm (for example WSOLA or PSOLA) or use a permissively licensed crate (verify the license). Quality bar: no audible clicks at ±2 semitones, judged in QA.
- Previews are synthesized on demand and cached in memory per voice and mood for the session.
- The table of nine moods matches the architecture docs; the vision lists "sarcastic", which is not a mood here (flagged for a product decision).
- Accessibility: the mood grid is a radio group with arrow-key navigation; previews never auto-play.

## QA procedure

1. Open Settings → Voice. Installed voices are listed with license info.
2. Press Preview on each of the nine moods. Each sounds different; Urgent is faster, Calm slower, Whisper quieter, and no preview clips or clicks.
3. Each mood shows "approximated" for Kokoro.
4. Set speed to 1.5× and preview Calm. It is faster than before but still slower than Urgent at 1.5×.
5. Set the default read-aloud mood to Cheerful and use `POST /v1/speech/speak` without a mood. It reads cheerfully.
6. Navigate the picker with the keyboard only. Arrow keys move between moods; Space previews.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
