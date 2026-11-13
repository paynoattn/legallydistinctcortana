# 017. STT engine: Parakeet via sherpa-onnx (CPU, DirectML)

| Milestone    | Size | Depends on | Labels                                   |
| ------------ | ---- | ---------- | ---------------------------------------- |
| M1 Dictation | M    | #015       | area/inference, area/voice, type/feature |

## Story

As an English-speaking user on a laptop without a CUDA GPU, I want a faster engine that shows words while I speak, so that dictation feels instant on Intel, AMD, and Qualcomm hardware.

## Context

[Voice: STT engines](../architecture/voice-and-speech.md#speech-to-text-engines-in-infer-host): sherpa-onnx (`sherpa-rs`) with NVIDIA Parakeet TDT 0.6B v3 and streaming Zipformer models on ONNX Runtime (CPU, DirectML, CUDA, later QNN and OpenVINO). It is the fastest English path, the one with true streaming partials, and the future NPU path. This is a "Should" in the [MVP scope](../product/mvp-plan.md#scope-moscow); NPU builds are #088.

## Scope

- A `SherpaEngine` in `infer-host` implementing the same `Transcribe` request as #016, plus streaming: `TranscribeStream` accepts audio chunks and emits `Partial { text, stable_prefix_len }` and `Final`.
- Two model families: Parakeet TDT 0.6B v3 (offline, highest accuracy) and a streaming Zipformer (true partials).
- Execution providers: CPU in every flavor; DirectML in the `cpu-vulkan` flavor (it runs on any DX12 GPU); CUDA in the `cuda` flavor. On EP failure, fall back to CPU and report why.
- Catalog entries for both models with URL, size, SHA-256, license, and minimum hardware (consumed by #018).
- Partials are published as `DictationPartial` events for the overlay's optional live caption (rendered in #021 if enabled).

Out of scope: QNN and OpenVINO EPs (#088), non-English Parakeet variants beyond what v3 supports, the engine picker UI (#018).

## Acceptance criteria

`features/core/stt-parakeet.feature`, with the real engine behind the `slow` feature and a fake for error paths:

```gherkin
Feature: Parakeet transcription

  Scenario: Offline Parakeet matches the reference
    Given the Parakeet TDT v3 model is installed
    When "the-quick-brown-fox.wav" is transcribed
    Then the word error rate is at most 10%

  Scenario: Streaming produces partials before the end
    Given the streaming Zipformer model is installed
    When a 6 s recording is streamed in 100 ms chunks
    Then at least one Partial arrives before the last chunk is sent
    And the Final text starts with the last Partial's stable prefix

  Scenario: DirectML is unavailable
    Given the DirectML execution provider fails to initialize
    When the model loads
    Then it runs on the CPU provider and the status reports "DirectML unavailable: <reason>"

  Scenario: The user picks Parakeet without the model
    Given the engine setting is Parakeet and the model is not installed
    When a transcription is requested
    Then it fails with code "model_missing" naming the Parakeet model

  Scenario: Unsupported language falls back to whisper
    Given the dictation language is set to Japanese
    And the engine setting is Parakeet
    When a transcription is requested
    Then whisper is used and the status says "Parakeet does not support this language"
```

## Design notes

- Only `infer-host` depends on `sherpa-rs` (or the official sherpa-onnx Rust bindings; verify which is maintained and how each links ONNX Runtime) and on `ort`.
- Licenses to verify before merging: sherpa-onnx (Apache-2.0), the Rust binding crate, ONNX Runtime (MIT), and the model weights (Parakeet's card on Hugging Face; check that redistribution through our catalog and attribution requirements are met). Record any cargo-deny exception in ADR 0003 and show attribution in Settings → About.
- DirectML: verify the ONNX Runtime DirectML package works from a per-user install without admin rights and which `DirectML.dll` version it needs.
- Shared model files: the engine loads ONNX only, with SHA-256 checked by #018.
- The engine-selection rule (language support × installed models × hardware) is a pure function in `models-domain`, property-tested.
- Privacy: audio is `DataClass::Voice`; partial text goes to the overlay window only as a short caption when the user enables live captions, otherwise not at all.
- Performance: partial latency under 300 ms from audio to `Partial`; measured in the `bench` job (#077).

## QA procedure

Uses the `POST /v1/debug/transcribe` endpoint from #016 with `engine=parakeet`.

1. On an Intel or AMD laptop without CUDA, place the Parakeet model in the models folder.
2. Post a recorded 10 s English WAV. The text is accurate, and the status reports the DirectML provider.
3. Stream the same file with `engine=zipformer&stream=1`. The response shows partials arriving before the final text.
4. Rename `DirectML.dll` in the infer-host folder (dev build only) and repeat step 2. It runs on CPU with the reason reported.
5. Set the language to Japanese and request Parakeet. Whisper handles it and the status explains why.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
