# 087. Echo cancellation and voice barge-in

| Milestone | Size    | Depends on       | Labels                                          |
| --------- | ------- | ---------------- | ----------------------------------------------- |
| Post-MVP  | unsized | #013, #026, #057 | area/voice, area/speech, type/feature, post-mvp |

## Story

As a person talking to the assistant on laptop speakers, I want to interrupt it by speaking ("stop", or a new question) while it is still talking, so that conversations feel natural and I do not have to reach for a key.

## Context

[voice-and-speech: wake word](../architecture/voice-and-speech.md#wake-word): while the assistant speaks, the wake word is suppressed unless echo cancellation is active (WebRTC AEC, post-MVP); the barge-in hotkey always works. The speech queue already supports barge-in by key ([#026](026-speech-queue.md)).

## Scope

- Acoustic echo cancellation in the capture path using the WebRTC audio processing module (through a Rust binding; verify the binding's license and maintenance), fed with the playback signal from the speech queue as the far-end reference.
- With AEC active: the wake word ([#057](057-wake-word.md)) and VAD stay on during playback; detected user speech stops or ducks TTS within 200 ms and starts a new turn.
- Noise suppression and automatic gain from the same module, optional.
- Fall back to the current behavior (suppression during playback) when AEC is off or its quality check fails (for example output and input on different clocks such as Bluetooth headsets).

Out of scope: echo cancellation for other apps' audio; meeting mode ([#083](083-meetings.md)).

## Acceptance criteria

```gherkin
Feature: Echo cancellation and voice barge-in

Scenario: Interrupt by voice on speakers
  Given AEC is on and the assistant is reading a long reply aloud through laptop speakers
  When I say "stop"
  Then playback stops within 200 ms of the end of the word

Scenario: The assistant does not trigger itself
  Given AEC is on and the assistant says the wake phrase in a reply
  Then the wake word does not fire
```

## Design notes

- AEC runs in the core's capture path next to VAD (no IPC hop), on its own thread; budget ≤ 2% of one core.
- `speech-application` publishes the rendered output frames to `voice-application` through a port; no direct crate dependency between their application layers.
- Measure on the reference laptop with the false-accept test set from spike #061 played through the speakers.

## QA procedure

1. On laptop speakers, ask a question with a long answer and say "stop" mid-sentence; observe it stop quickly. Repeat 10 times and count misses.
2. Play a recording of the wake phrase through the assistant's own TTS; confirm no trigger.
