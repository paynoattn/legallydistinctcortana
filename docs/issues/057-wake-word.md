# 057. Wake word (off by default)

| Milestone    | Size | Depends on | Labels                                 |
| ------------ | ---- | ---------- | -------------------------------------- |
| M5 Assistant | M    | #061, #013 | area/voice, area/privacy, type/feature |

## Story

As an accessibility user, I want to say "Hey <name>" to start the assistant without touching the keyboard, knowing that nothing I say before the wake word leaves the microphone buffer, so that I can use it hands-free without being recorded.

## Context

The wake word is off by default. When on, an always-listening capture runs openWakeWord (ONNX, CPU) at about 1–2% of one core, and audio never leaves the ring buffer unless the wake word fires. The phrase is "Hey <brand name>", the model is trained per spike #061, users can add a personal verifier from 5 samples, and the wake word is suppressed while the assistant speaks unless echo cancellation is active ([voice-and-speech: wake word](../architecture/voice-and-speech.md#wake-word)). Budget: fewer than 1 false accept per 8 hours of mixed office audio ([voice-and-speech budgets](../architecture/voice-and-speech.md#budgets)). It is a Could: it sits in M5 but may slip to v0.2 ([mvp-plan index](../product/mvp-plan.md#issue-index)).

## Scope

- **Settings → Voice → Wake word:** off by default. Turning it on shows what it does, the CPU cost, and that audio stays on the device, then downloads and verifies the model (#018) if needed.
- **Listening loop:**
  - Capture 16 kHz mono through #013's capture, into a 2 s in-memory ring buffer (pre-roll).
  - Detection uses the model and threshold chosen by #061.
  - On fire: show the overlay in Armed, play the soft tick, and start an `assistant`-mode dictation session (#051), with the pre-roll trimmed to after the wake phrase.
- **Personal verifier:** optional enrollment of 5 samples of the user saying the phrase. Stored encrypted and device-local (`DataClass::Voice`), never synced, and deletable.
- **Suppression:**
  - while the assistant or any `Speak` source is playing (no AEC in MVP)
  - while dictation is active
  - while the session is locked (default; setting)
  - on battery saver (setting)
  - while another app holds the microphone exclusively (capture error → pause and show a tray state)
- **Visibility:** a tray icon state and an optional overlay dot while listening. The Windows microphone privacy indicator will also show. A global "pause listening" hotkey (configurable, unbound by default) and a tray menu toggle.
- **Phrase:** read from the brand configuration (`src/shared/brand.ts` and `tauri.conf.json`, per [ADR 0011](../architecture/adr/0011-brand-neutral-code.md)). No product name is hard-coded.

Out of scope: echo cancellation and voice barge-in (#087), custom phrases beyond what #061 enables, and NPU execution (#088).

## Acceptance criteria

```gherkin
Feature: Wake word

  Scenario: Off by default
    Given a fresh install
    Then no audio capture stream is open while idle
    And Settings shows the wake word as off

  Scenario: Wake word starts an assistant turn
    Given the wake word is on
    When the test clip "hey-name-what-time-is-it.wav" is played into the capture
    Then the overlay shows Armed within 300 ms of the phrase ending
    And the assistant receives "what time is it" without the wake phrase

  Scenario: Audio before the wake word never leaves the buffer
    Given the wake word is on
    When 10 minutes of office audio without the phrase are played
    Then no STT request, file write, or IPC event containing audio occurs

  Scenario: Suppressed while the assistant speaks
    Given the assistant is speaking a reply that contains the wake phrase
    Then the wake word does not fire

  Scenario: Locked session pauses listening
    Given "listen when locked" is off
    When the session locks
    Then the capture stream is closed
    And it reopens on unlock

  Scenario: Microphone taken by another app
    Given another app opens the microphone in exclusive mode
    Then listening pauses and the tray shows "Wake word paused: microphone in use"

  Scenario: Verifier rejects other speakers
    Given a personal verifier is enrolled
    When the phrase is played in a different speaker's voice from the test set
    Then the wake word does not fire

  Scenario: False-accept budget on the regression set
    When the 8-hour mixed office audio set is replayed
    Then at most 1 false accept occurs
```

## Design notes

- **Where it runs:** in-process in `voice-infrastructure`, next to Silero VAD (#014), on ONNX Runtime (`ort`) CPU. The overview allows `ort` there for exactly these two models ([overview: dependency rules](../architecture/overview.md#dependency-rules-added-to-archtoml-as-crates-land)). Audio never crosses a process boundary before the trigger.
- **Crates:** `voice-domain` (the `WakeGate` state machine: Listening, Suppressed(reason), Fired, Paused) and `voice-application` (gating rules and enrollment use case). `voice-infrastructure` hosts the detector and the ring buffer, with capture through `cpal`.
- **arch.toml:** no new crates. `ort` stays allowed only in `infer-host` and `voice-infrastructure`.
- **Privacy:** `DataClass::Voice`. No audio before the trigger is persisted or transmitted; this is tested by asserting on the ports in acceptance. Enrollment samples are encrypted in the store and deleted with "Delete wake word data".
- **Budgets:** CPU ≤ 2% of one core on the reference laptop (bench). Detection latency ≤ 300 ms after the phrase ends. False accepts < 1 per 8 h on the regression set from #061.
- **Testing:** golden-audio tests behind the `slow` feature. The 8 h regression replay runs in the `bench` job, not `make check`.

## QA procedure

1. On a fresh install, open Task Manager → Performance → open Resource Monitor → CPU, and check the app is not using the microphone (no mic indicator in the taskbar).
2. Settings → Voice → Wake word → On. Read the explanation and confirm. Observe the model download and the mic indicator appearing.
3. Say "Hey <name>, what time is it?". Observe the overlay and the spoken answer.
4. Hold a 30-minute meeting or play a podcast near the PC. Observe no false triggers (count them in the tray menu's "wake events" counter).
5. Ask a question whose answer contains the wake phrase. Observe no self-trigger.
6. Lock with Win+L. Observe that the mic indicator disappears. Unlock and observe that it returns.
7. Enroll the verifier. Have another person say the phrase. Observe that it does not trigger (or record the rate).
8. Turn the wake word off. Observe that the mic indicator disappears and CPU returns to idle.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
