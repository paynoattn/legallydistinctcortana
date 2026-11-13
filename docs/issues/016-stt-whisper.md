# 016. STT engine: whisper.cpp (CPU, Vulkan, CUDA)

| Milestone    | Size | Depends on | Labels                                   |
| ------------ | ---- | ---------- | ---------------------------------------- |
| M1 Dictation | M    | #015       | area/inference, area/voice, type/feature |

## Story

As a writer, I want my speech transcribed on my own GPU or CPU with whisper, so that dictation is accurate, fast, and works with the network unplugged.

## Context

[Voice: STT engines](../architecture/voice-and-speech.md#speech-to-text-engines-in-infer-host): whisper.cpp through `whisper-rs` is the default engine, with CPU (AVX2), CUDA, and Vulkan backends; large-v3-turbo on Vulkan is the best general choice. Budget: release → text inserted in 500 ms on GPU and 1.5 s on CPU for utterances up to 15 s without cleanup. [ADR 0007](../architecture/adr/0007-model-runtimes.md).

## Scope

- A `WhisperEngine` in `infer-host` implementing `Transcribe { request_id, model, audio: f32 16 kHz mono, language: Auto | Code, initial_prompt }` → `Final { text, language, segments, timings }`.
- Keep the loaded model warm between requests; load on first use or on `LoadModel`; unload on `UnloadModel` or idle timeout.
- Backend choice per flavor: the `cpu-vulkan` build tries Vulkan and falls back to CPU; the `cuda` build tries CUDA and falls back to CPU. The chosen backend is reported in `Final.timings`.
- Initial prompt from the user's dictionary terms (#019 supplies them), capped at whisper's prompt token limit.
- `voice-application`: the `SpeechToText` port; `infer-client` (#015) implements it over the pipe.
- Tests with `ggml-tiny.en` behind the `slow` feature, using golden WAV fixtures.

Out of scope: Parakeet (#017), model download UI (#018), cloud STT (later issue), streaming partials (whisper is chunked here; Parakeet brings true streaming).

## Acceptance criteria

`features/core/stt-whisper.feature`, with the real engine and `tiny.en` behind the `slow` feature, and a fake engine for the error paths:

```gherkin
Feature: Whisper transcription

  Scenario: A short English utterance is transcribed
    Given the tiny.en model is installed
    When "the-quick-brown-fox.wav" is transcribed
    Then the text matches "The quick brown fox jumps over the lazy dog" with word error rate at most 10%

  Scenario: The model is not downloaded
    Given no whisper model is installed
    When a transcription is requested
    Then it fails with code "model_missing" and the fix action "open_models"
    And infer-host keeps running

  Scenario: GPU backend fails to initialize
    Given the Vulkan backend reports an initialization error
    When the model loads
    Then it loads on the CPU backend
    And the engine status reports backend "cpu" with the reason "GPU unavailable: <reason>"

  Scenario: GPU runs out of memory
    Given large-v3-turbo does not fit in the free VRAM
    When it is loaded
    Then the engine returns "insufficient_vram" and suggests a smaller model

  Scenario: Cancelled transcription returns nothing
    Given a 15 s transcription is running
    When the request is cancelled
    Then the engine stops within 200 ms and returns "cancelled" with no text

  Scenario: Dictionary terms bias recognition
    Given the dictionary contains "Tauri"
    When "we-use-tauri.wav" is transcribed
    Then the text contains "Tauri"
```

## Design notes

- Only `infer-host` depends on `whisper-rs`. Verify the `whisper-rs` license (it may be Unlicense, which is not on the cargo-deny allow-list; an exception goes into ADR 0003) and that whisper.cpp itself is MIT.
- Build: Vulkan needs the Vulkan SDK at build time, CUDA needs the CUDA toolkit; CI's `make check` builds the CPU flavor only, and the GPU flavors build in the release job (#075). Verify whisper.cpp's CMake options as exposed by `whisper-rs` features.
- Model files are GGML/GGUF only, with SHA-256 checked by the model manager (#018) before load.
- Privacy: audio is `DataClass::Voice`. Transcripts are never logged; timings and model names are.
- Performance: the `bench` job (#077) asserts the 500 ms (GPU) and 1.5 s (CPU) budgets on reference hardware. Locally, `Final.timings` gives load, encode, and decode times for diagnostics.
- Language: `Auto` by default; a fixed language from settings skips detection and saves time.

## QA procedure

This issue adds `POST /v1/debug/transcribe` (WAV body, token-protected) to `local-api` so the engine can be tried before the dictation session (#019) exists.

1. Place `ggml-large-v3-turbo.bin` in `%LOCALAPPDATA%\<app>\models` (the model manager, #018, automates this later).
2. Start `headless` with `LOCAL_API_TOKEN=dev PORT=3999`, unplug the network, and post a recorded 10 s WAV of your own voice with `curl --data-binary @me.wav`. The text is accurate and the reported timings show a GPU backend and well under a second on a GPU laptop.
3. Run the CPU flavor (or disable the GPU in Device Manager) and repeat. It still works, slower, and the status reports `cpu` with a reason.
4. Delete the model file and repeat. The response is `model_missing` with the fix `open_models`, and `infer-host` is still running.
5. Repeat step 2 with `initial_prompt=Tauri` and a recording of "we built it with Tauri". The word is spelled correctly.
6. Start a long transcription and cancel it from a second request. The first returns `cancelled` quickly.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
