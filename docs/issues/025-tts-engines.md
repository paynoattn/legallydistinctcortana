# 025. TTS engines: Kokoro and Piper with sentence streaming

| Milestone    | Size | Depends on | Labels                                    |
| ------------ | ---- | ---------- | ----------------------------------------- |
| M2 Voice out | M    | #015, #018 | area/speech, area/inference, type/feature |

## Story

As a user who listens to text, I want a natural local voice that starts speaking almost immediately, even for long text, so that read-aloud and spoken replies feel instant and work offline.

## Context

[Voice: text-to-speech and moods](../architecture/voice-and-speech.md#text-to-speech-and-moods): Kokoro-82M (ONNX, Apache-2.0) is the default voice, fast on CPU and any GPU; Piper covers low-end hardware and many languages. Long text is split into sentences, synthesized one sentence ahead, and streamed to playback. Budget: "Read aloud" → first audio in 300 ms. [ADR 0007](../architecture/adr/0007-model-runtimes.md).

## Scope

- `speech-domain`: `SentenceSplitter` (handles abbreviations, decimals, URLs, lists, and CJK punctuation; caps a sentence at 400 characters by splitting at commas), `TextNormalizer` for TTS (numbers, dates, units, common abbreviations; English first), and `Voice { id, engine, language, gender_hint, license }`.
- `speech-application`: a `SpeechSynthesizer` port (`synthesize(sentence, voice, style) -> stream of PCM chunks`). `infer-client` (#015) implements it.
- `infer-host`: a `KokoroEngine` (ONNX through ORT; voice packs as style tensors) and a `PiperEngine` (ONNX voices with their JSON config), both producing 24 kHz (Kokoro) or the voice's native rate (Piper) mono PCM in `AudioChunk` frames.
- Catalog entries (#018) for Kokoro v1.0 with a curated set of voices, and a handful of Piper voices whose licenses permit redistribution and that can be served without espeak-ng.
- A `speak_preview(text, voice)` debug path on `local-api` for QA before the queue (#026) exists.

Out of scope: the playback queue and audio output (#026), moods and the voice picker (#027), Windows OneCore voices (not yet scheduled), the expressive model (#081).

## Acceptance criteria

`features/core/tts.feature`, with the real engines behind the `slow` feature and a fake for error paths:

```gherkin
Feature: Local TTS

  Scenario: Kokoro speaks a sentence
    Given the Kokoro model and voice "af_heart" are installed
    When "Hello from your own computer." is synthesized
    Then PCM audio of 1 to 4 seconds is returned
    And transcribing it with whisper tiny.en gives word error rate at most 15%

  Scenario: Long text streams sentence by sentence
    Given a 2,000-word article
    When synthesis starts
    Then the first audio chunk is returned after only the first sentence is synthesized
    And sentence n+1 is synthesized while sentence n plays

  Scenario: Abbreviations and numbers
    When "Dr. Smith paid $3.50 on 3/14." is split and normalized
    Then it is one sentence read as "Doctor Smith paid three dollars fifty on March fourteenth"

  Scenario: Voice not installed
    Given voice "bf_emma" is not installed
    When it is requested
    Then synthesis fails with "voice_missing" and the fix "open_voices"

  Scenario: Piper on a low-end machine
    Given the probe reports no GPU and 4 CPU cores
    Then the recommended voice is a Piper voice with the reason "Fast on your CPU"

  Scenario: Text the engine cannot pronounce
    When a sentence of only emoji is synthesized
    Then it is skipped without an error and the next sentence plays
```

## Design notes

- **No GPL linking** ([ADR 0007](../architecture/adr/0007-model-runtimes.md)). espeak-ng (GPL-3.0) and Piper's `piper-phonemize` are not linked. English uses a Rust implementation of Kokoro's misaki approach (lexicon plus pronunciation dictionary; verify the license of the lexicon data). Piper voices that need espeak-ng are dropped from the catalog. Other languages wait for a permissive G2P (spike #060).
- Only `infer-host` depends on `ort`. `speech-domain` allow `["kernel"]`; `speech-application` allow `["speech-domain", "kernel"]`.
- Voice packs and Piper voices are catalog models: SHA-256 pinned, license shown before download. Piper voice licenses vary per voice; include only those that allow redistribution.
- Text passed to TTS may come from mail, web pages, or the assistant. It is data, never instructions; TTS has no tools. Data class follows the source (`Screen`, `Mail`, `Public`).
- Budget: first sentence of 15 words synthesized in under 200 ms on a mid-range GPU and under 400 ms on CPU for Kokoro, leaving room for the 300 ms first-audio budget on GPU (#077 measures it).

## QA procedure

1. Download Kokoro and two voices in Settings → Models.
2. Post a short sentence to the debug `speak_preview` endpoint and play the returned WAV. The voice is clear.
3. Post a 2,000-word article. The response streams; the first chunk arrives in a fraction of a second.
4. Post "Dr. Smith paid $3.50 on 3/14." and listen. Numbers and the abbreviation read naturally.
5. Request an uninstalled voice. The `voice_missing` error is returned.
6. With the network unplugged, repeat step 2. It still works.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
