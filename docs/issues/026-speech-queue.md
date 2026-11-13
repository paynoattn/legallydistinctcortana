# 026. Speech queue, priorities, playback, barge-in

| Milestone    | Size | Depends on | Labels                                   |
| ------------ | ---- | ---------- | ---------------------------------------- |
| M2 Voice out | M    | #025, #012 | area/speech, area/platform, type/feature |

## Story

As a user, I want an urgent reminder to cut in over an article being read, and a key or my own voice to stop the assistant mid-sentence, so that speech never talks over what matters or over me.

## Context

[Voice: requests](../architecture/voice-and-speech.md#requests): `Speak { text, voice, mood, rate, source }` with `source` = `ReadAloud | AssistantReply | Alert | Briefing`. Alerts preempt replies, and replies preempt read-aloud. New user speech or the stop hotkey cancels playback (barge-in). Sentences are synthesized one ahead and streamed to playback.

## Scope

- `speech-domain`: `SpeechQueue`, a pure model of queued and playing items with priorities `Alert > AssistantReply > Briefing = ReadAloud`, preemption, and resume rules: a preempted read-aloud or briefing resumes from the start of the interrupted sentence after the higher-priority item finishes; a preempted reply is dropped. A new read-aloud replaces the current read-aloud instead of queuing behind it.
- `speech-application`: `SpeechService` (`speak`, `stop`, `pause`, `resume`, `skip_sentence`), driving `SpeechSynthesizer` (#025) and an `AudioOutput` port, publishing `SpeechStarted { source, mood }`, `OutputLevel { rms_db }` at 30 Hz, `SpeechPaused`, and `SpeechFinished { reason: Completed | Stopped | Preempted | Failed }`.
- `speech-application`: an `AudioOutput` port (open the default or a pinned output, write PCM, drain, stop immediately). `speech-infrastructure`: the cpal/WASAPI adapter with resampling to the device rate and default-device following.
- Barge-in: the dictation or assistant hotkey going down stops playback before capture starts; a configurable stop hotkey (`hotkeys.stop_speech`, default Win+Alt+X) stops everything.
- `local-api`: `POST /v1/speech/speak` and `/v1/speech/stop` for QA.
- Settings (device-local): `speech.output_device`.

Out of scope: moods and prosody (#027), the reading pill UI (#030), voice barge-in by detecting speech while playing (needs echo cancellation, #087), spoken alerts policy (#035).

## Acceptance criteria

`features/core/speech-queue.feature` with fake synthesizer and fake output:

```gherkin
Feature: Speech queue

  Scenario: An alert preempts read-aloud and read-aloud resumes
    Given an article is being read aloud at sentence 4
    When an Alert "Meeting in 5 minutes" is spoken
    Then the article stops within 100 ms and the alert plays
    And the article resumes from the start of sentence 4

  Scenario: A new read-aloud replaces the current one
    Given article A is being read aloud
    When article B is sent to read aloud
    Then A stops and B starts, and A does not resume

  Scenario: The dictation hotkey barges in
    Given an assistant reply is playing
    When the dictation hotkey goes down
    Then playback stops before capture starts and the reply is dropped

  Scenario: The stop hotkey clears everything
    Given an alert is playing and a read-aloud is waiting to resume
    When Win+Alt+X is pressed
    Then playback stops and nothing resumes

  Scenario: The output device disappears
    Given playback is on a Bluetooth headset
    When the headset disconnects
    Then playback continues on the new Windows default output within 1 second

  Scenario: No output device
    Given no audio output device exists
    When text is spoken
    Then SpeechFinished reports Failed with "no_output_device" and the fix "open_sound_settings"

  Scenario: The engine dies mid-article
    Given infer-host crashes during sentence 6
    Then playback finishes the buffered audio
    And after the restart reading continues from sentence 7, or stops with an error if the restart fails
```

## Design notes

- `arch.toml`: `speech-domain` allow `["kernel"]`; `speech-application` allow `["speech-domain", "platform-ports", "settings-domain", "kernel"]`. `speech-infrastructure` allow `["speech-application", "speech-domain", "platform-ports", "kernel"]`, ban `tauri`. cpal uses `windows` only transitively, which the direct-dependency rule allows (#092).
- The queue model is pure and property-tested (no item is played twice; an Alert never waits behind a non-Alert).
- Stop latency: `AudioOutput::stop` discards buffered audio immediately; target under 50 ms from key to silence.
- Esc as a stop key belongs to the reading pill (#030), not here.
- Privacy: queued text is held in memory only and wrapped in `kernel::Redacted`; logs record source, duration, and outcome.
- Budget: with #025, "Read aloud" → first audio in 300 ms (#077).

## QA procedure

1. Start the app, then `POST /v1/speech/speak` with a long article (source ReadAloud). It starts reading within a moment.
2. While it reads, post an Alert. The article stops, the alert plays, and the article resumes at the interrupted sentence.
3. Press Win+Alt+X. Everything stops.
4. Start a read-aloud and hold Right Ctrl. Playback stops before the overlay shows Listening.
5. Play through a Bluetooth headset and turn the headset off. Audio moves to the laptop speakers.
6. End `infer-host` in Task Manager during a long read. Reading continues after a short gap.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
