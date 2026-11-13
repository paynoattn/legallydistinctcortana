# 013. Audio capture, device selection, level metering

| Milestone    | Size | Depends on | Labels                                  |
| ------------ | ---- | ---------- | --------------------------------------- |
| M1 Dictation | M    | #011       | area/voice, area/platform, type/feature |

## Story

As a user with a laptop mic and a headset, I want the assistant to capture from the right microphone, follow my headset when I plug it in, and show that it hears me, so that dictation starts instantly and I can trust it is listening.

## Context

[Voice: dictation pipeline](../architecture/voice-and-speech.md#dictation-pipeline): capture through cpal/WASAPI shared mode, 16 kHz mono f32, 20 ms frames, into a ring buffer. Budget: hotkey down → first audio frame buffered in 30 ms, which needs the stream kept open while armed. [OS integration: `AudioDevices`](../architecture/os-integration.md): WASAPI enumeration and `IMMNotificationClient`; the default device follows Windows unless pinned.

## Scope

- `voice-application`: `AudioInput` (open a device, start, stop; yields `AudioFrame` of 320 samples at 16 kHz mono f32) and `AudioDevices` (list inputs, default input) ports. `platform-ports`: `AudioDeviceEvents` (hot-plug and default-device changes).
- `voice-infrastructure`: the cpal adapter (WASAPI shared mode, device format → 16 kHz mono resampling and downmix) and the device list. `platform-windows`: hot-plug notifications through `IMMNotificationClient` (verify that cpal offers no device-change callback of its own).
- `voice-domain`: `RingBuffer` (pre-roll of 300 ms so the first syllable is not clipped), RMS level computation, and a `CaptureState` machine (Closed → Warm → Capturing).
- `voice-application`: `CaptureService` (warm on arm, capture on listen, close after 30 s of being unarmed), publishing `InputLevel { rms_db }` at 30 Hz and `InputDeviceChanged` on the bus.
- Settings (device-local): `voice.input_device` (follow Windows default, or a pinned device id).
- `src/settings` → Microphone section: device list, live level meter (`Meter` primitive), "Follow Windows default" option.

Out of scope: VAD (#014), noise suppression and echo cancellation (#087), recording to disk (only #024 may, opt-in).

## Acceptance criteria

`features/core/audio-capture.feature` with a fake `AudioInput` and `AudioDevices`; `features/ui/microphone.feature` with fake IPC.

```gherkin
Feature: Audio capture

  Scenario: Frames arrive in the canonical format
    Given a fake 48 kHz stereo device
    When capture runs for 1 second
    Then 50 frames of 320 mono samples at 16 kHz reach the ring buffer

  Scenario: Pre-roll keeps the first syllable
    Given capture is warm
    When listening starts
    Then the first frame handed to the session is from at least 300 ms before the start

  Scenario: No microphone
    Given no input device exists
    When dictation is armed
    Then the session fails with code "no_microphone"
    And the error says "No microphone found" with the fix "Open sound settings"

  Scenario: The default device changes mid-session
    Given capture follows the Windows default
    When a USB headset is plugged in and becomes the default
    Then the current session continues on the old device
    And the next session uses the headset

  Scenario: The pinned device is unplugged
    Given the pinned device is unplugged during capture
    Then the session ends with code "device_lost" and keeps the audio captured so far
    And Settings shows the pinned device as "Not connected"

  Scenario: Microphone access is blocked by Windows privacy settings
    Given Windows denies microphone access to desktop apps
    When dictation is armed
    Then the error says "Windows is blocking the microphone" with the fix "Open privacy settings"

  Scenario: The level meter moves with speech
    Given the Microphone settings page is open with "?ipc=fake"
    When the fake emits InputLevel at -20 dB
    Then the meter shows a level and announces nothing to screen readers more than once per second
```

## Design notes

- cpal uses the `windows` crate only transitively, which the direct-dependency rule allows ([overview](../architecture/overview.md#dependency-rules-added-to-archtoml-as-crates-land), #092), so the adapter lives in `voice-infrastructure`. If cpal's `windows` version differs from ours, cargo-deny's duplicate check will say so.
- `arch.toml`: `voice-domain` allow `["kernel"]`; `voice-application` allow `["voice-domain", "platform-ports", "kernel"]`; `voice-infrastructure` allow `["voice-application", "voice-domain", "platform-ports", "kernel", "store"]`, ban `tauri`. The usual bans apply, including `tokio`; the audio callback hands frames to a lock-free SPSC queue drained by an infrastructure task.
- Resampling: pick a permissive pure-Rust resampler (for example `rubato`; verify its license) or a fixed polyphase filter for 44.1/48 → 16 kHz.
- The audio callback must not allocate or block. Frames are dropped, and a counter is incremented, if the consumer falls behind.
- Privacy: audio is `DataClass::Voice`, lives only in memory, and is never logged. Device names are not sensitive.
- Detecting the Windows privacy block: verify which error WASAPI returns when "Let desktop apps access your microphone" is off.

## QA procedure

1. Open Settings → Microphone. The list shows every input, and the default is marked.
2. Speak. The meter moves; silence keeps it low.
3. Plug in a USB headset and make it the default in Windows. The page updates without a restart.
4. Pin the laptop mic, then unplug the headset during a dictation. The session ends with "Microphone disconnected" and nothing crashes.
5. Turn off "Let desktop apps access your microphone" in Windows Settings and try to dictate. The privacy error and its fix link appear.
6. Disable all inputs in Sound settings and try to dictate. "No microphone found" appears.
7. Arm and release dictation with no speech, 20 times. Memory in Task Manager does not grow.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
