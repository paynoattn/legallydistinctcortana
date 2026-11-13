# 088. NPU builds: QNN and OpenVINO

| Milestone | Size    | Depends on             | Labels                                               |
| --------- | ------- | ---------------------- | ---------------------------------------------------- |
| Post-MVP  | unsized | #015, #017, #018, #075 | area/inference, area/release, type/feature, post-mvp |

## Story

As a user of a Copilot+ laptop with a Snapdragon X or Intel Core Ultra NPU, I want dictation (and later TTS and small models) to run on the NPU, so that it is fast, quiet, and easy on my battery.

## Context

[ADR 0007](../architecture/adr/0007-model-runtimes.md): NPU support comes through ONNX Runtime execution providers with per-vendor `infer-host` builds. [voice-and-speech](../architecture/voice-and-speech.md#speech-to-text-engines-in-infer-host): sherpa-onnx (Parakeet, streaming Zipformer) is the NPU path. The [MVP plan](../product/mvp-plan.md#scope-moscow) makes NPU builds "Could" (v0.2) because providers vary by vendor and driver.

## Scope

- `infer-host` flavors: **QNN** (Snapdragon X, ARM64) and **OpenVINO** (Intel Core Ultra NPU and GPU, x64), each with ONNX Runtime built or shipped with that execution provider.
- Model variants in the catalog: quantized ONNX (for example QDQ int8 for the QNN HTP backend) for Parakeet and Silero VAD; verify which operators each provider supports and fall back per node to CPU where needed.
- Hardware probe ([#015](015-infer-host.md)) detects the NPU and driver version; the model manager recommends the NPU path only after a 5 s benchmark passes.
- Installer offers the matching flavor as a verified download ([#075](075-installer-signing-updater.md)).
- Ryzen AI (Vitis AI execution provider), mentioned in the vision, is evaluated in the same issue but may become its own follow-up.

Out of scope: NPU TTS and LLM until STT is stable on NPUs.

## Acceptance criteria

```gherkin
Feature: NPU builds: QNN and OpenVINO

Scenario: Dictation on a Snapdragon NPU
  Given a Snapdragon X laptop with the QNN flavor installed
  When I dictate a 10-second sentence
  Then the text is inserted within 500 ms of release
  And Task Manager shows NPU activity during transcription

Scenario: Unsupported driver falls back
  Given an NPU whose driver fails the probe benchmark
  Then the model manager recommends the CPU or GPU path and explains why
```

## Design notes

- Only `infer-host` depends on ORT bindings; flavors differ only in build features and bundled provider libraries.
- Provider libraries and model variants carry their own licenses; check redistribution terms for each vendor's runtime (verify).
- Add NPU paths to the bench job ([#077](077-performance-bench.md)) when a reference device is available.

## QA procedure

1. On a Snapdragon X laptop and an Intel Core Ultra laptop, install the matching flavor, run the hardware probe, and dictate into Notepad; check latency and NPU activity in Task Manager.
2. Downgrade or disable the NPU driver and confirm the fallback message.
