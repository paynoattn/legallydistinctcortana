# 083. Meeting transcription with on-device diarization

| Milestone | Size    | Depends on             | Labels                             |
| --------- | ------- | ---------------------- | ---------------------------------- |
| Post-MVP  | unsized | #013, #016, #017, #047 | area/voice, type/feature, post-mvp |

## Story

As a knowledge worker in back-to-back calls, I want the assistant to transcribe a meeting on my PC, label who spoke when, and give me a summary with action items, so that I can stay present in the meeting without anything leaving my machine.

## Context

A non-goal for v0.1 in the [vision](../product/vision.md#non-goals-for-now) and "Won't" in the [MVP plan](../product/mvp-plan.md#scope-moscow). OpenWhispr offers it; we add it later on the same capture ([#013](013-audio-capture.md)) and STT engines ([#016](016-stt-whisper.md), [#017](017-stt-parakeet.md)).

## Scope

- Start and stop from the tray, a hotkey, or a calendar heads-up ("Transcribe this meeting?"); a visible recording indicator for the whole session.
- Capture the microphone and system audio (WASAPI loopback) as two channels.
- Streaming transcription in `infer-host`; speaker diarization on device (sherpa-onnx speaker segmentation and embedding models; verify model licenses allow redistribution).
- After the meeting: summary, decisions, and action items through the router's local route; action items can become reminders (Write tool, with undo).
- Transcripts stored encrypted (`DataClass::Voice`), audio not kept unless the user opts in; device-local by default.

Out of scope: joining meetings as a bot; cloud transcription by default.

## Acceptance criteria

```gherkin
Feature: Meeting transcription with on-device diarization

Scenario: Transcribe a two-person call
  Given a Teams call with one remote speaker
  When I start meeting transcription and talk for 5 minutes
  Then the transcript labels two speakers with timestamps
  And nothing is sent off the device

Scenario: Consent reminder
  When I start transcription
  Then I see "Let everyone know you are transcribing" before capture begins
```

## Design notes

- Recording laws differ by place; the product reminds, the user decides. Never start automatically.
- Crates: `voice-domain` (meeting session state), `voice-application`, `infer-host` (diarization engine), `platform-windows` (loopback capture behind the `AudioDevices` port).
- Budget: transcription keeps up in real time on CPU for the default model; diarization runs after the meeting if it cannot keep up.

## QA procedure

1. Join a test call, start transcription, speak alternately with a second person for 5 minutes, stop. Check speaker labels, timestamps, and the summary.
2. Confirm in the privacy dashboard that nothing was sent.
