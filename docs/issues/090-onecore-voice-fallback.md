# 090. Windows OneCore voice fallback (zero download)

| Milestone    | Size | Depends on | Labels                                   |
| ------------ | ---- | ---------- | ---------------------------------------- |
| M2 Voice out | S    | #026       | area/speech, area/platform, type/feature |

## Story

As a new user who hasn't downloaded a voice yet, or whose machine is too weak for Kokoro, I want read-aloud and alerts to work with the voices built into Windows, so that speech works from the first minute with nothing to download.

## Context

[Voice: TTS engines](../architecture/voice-and-speech.md#engines-in-infer-host) and [ADR 0007](../architecture/adr/0007-model-runtimes.md): Windows OneCore voices are the zero-download fallback, with rate and pitch only. [Moods](../architecture/voice-and-speech.md#moods): each mood is marked "native" or "approximated" for the current voice.

## Scope

- `platform-ports`: an `OsVoices` port: `list() -> [OsVoice { id, name, language }]` and `synthesize(text, rate, pitch) -> PCM`.
- `platform-windows`: an adapter over the WinRT `Windows.Media.SpeechSynthesis.SpeechSynthesizer` (OneCore voices), returning PCM decoded from the synthesized WAV stream. Rate and pitch go through the synthesizer options (verify the `SpeakingRate` and `AudioPitch` ranges and the minimum Windows build). FFI lives in `src/ffi/` under ADR 0012.
- `speech-infrastructure`: a `OneCoreSynthesizer` that implements the `SpeechSynthesizer` port from `OsVoices`, mapping the style vector's `rate` and `pitch_semitones` and ignoring `emotion`, `energy_db`, and `pause_scale`.
- Voice selection: OneCore voices appear in the voice picker as "Built into Windows". When no downloaded voice is installed, the default voice is the OneCore voice matching the Windows display language.
- Mood support: every mood except Neutral is marked "approximated" for OneCore voices.

Out of scope: SAPI 5 desktop voices (OneCore only), installing extra Windows language voices (link to Windows Settings instead), synthesis in `infer-host` (OneCore runs in the desktop process; it is an OS service, not a model).

## Acceptance criteria

`features/core/onecore-voices.feature` with a fake `OsVoices`; the real adapter in the QA procedure.

```gherkin
Feature: OneCore voice fallback

  Scenario: Speech works before any download
    Given no Kokoro or Piper voice is installed
    And Windows has the voice "Microsoft Aria" for en-US
    When text is read aloud
    Then it is spoken with the OneCore voice matching the display language

  Scenario: Moods are approximated with rate and pitch
    Given the voice is a OneCore voice
    When "Urgent" is requested
    Then the synthesizer receives a higher rate and pitch than for Neutral
    And the voice picker marks Urgent as "approximated"

  Scenario: A downloaded voice takes over
    Given Kokoro "af_heart" is installed and no voice was chosen
    Then the default voice becomes Kokoro "af_heart"

  Scenario: No OneCore voice for the language
    Given the display language is Icelandic and no OneCore voice exists for it
    When text is read aloud with no downloaded voice
    Then an English OneCore voice is used
    And the voice picker suggests downloading a voice or adding one in Windows Settings

  Scenario: OneCore is unavailable
    Given the OS reports no OneCore voices at all
    When text is read aloud with no downloaded voice
    Then SpeechFinished reports Failed with "no_voice" and the fix "open_voices"
```

## Design notes

- `arch.toml`: `speech-infrastructure` allow `["speech-application", "speech-domain", "platform-ports", "kernel"]`, ban `tauri`. The WinRT calls stay in `platform-windows`; `speech-infrastructure` sees only the port.
- Mapping from the style vector (rate 0.5–2.0, pitch −6..+6 semitones) to the OneCore option ranges is a pure function with tests; clamp at the engine limits.
- Data class follows the source of the text (`Screen`, `Mail`, `Public`). OneCore synthesis runs locally (verify that no OneCore voice sends text off the device; Windows "natural" voices may download models but synthesize locally).
- Budget: first audio within the 300 ms read-aloud budget; OneCore synthesizes a whole sentence at a time, which fits sentence streaming (#025, #026).

## QA procedure

1. On a clean Windows 11 install with no voices downloaded in the app, select text and press Ctrl+Alt+R (or use `POST /v1/speech/speak`). It is read in a Windows voice.
2. Open Settings → Voice. Windows voices are listed as "Built into Windows", and every mood except Neutral says "approximated".
3. Preview Calm and Urgent. Urgent is faster and higher.
4. Download a Kokoro voice. The default switches to it unless a voice was chosen.
5. Unplug the network and repeat step 1 with a OneCore voice. It still works.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
