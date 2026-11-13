# 018. Model manager: catalog, verified resumable downloads, recommendations

| Milestone    | Size | Depends on | Labels                             |
| ------------ | ---- | ---------- | ---------------------------------- |
| M1 Dictation | M    | #015, #006 | area/models, area/ui, type/feature |

## Story

As a new user, I want the app to recommend the right speech model for my hardware and download it safely, resuming if my Wi-Fi drops, so that I get the best local quality without knowing what "large-v3-turbo on Vulkan" means.

## Context

[Voice: hardware selection and model manager](../architecture/voice-and-speech.md#speech-to-text-engines-in-infer-host): catalog entries pin URL, size, SHA-256, license, and minimum hardware; downloads are resumable and verified, stored in `%LOCALAPPDATA%\<app>\models`. On first run `infer-host --probe` reports the hardware, the manager recommends an engine and model, then benchmarks a 5 s sample to confirm. [Security: threat model](../architecture/security.md#threat-model-stride-summary): SHA-256 pinned; safetensors, GGUF, and ONNX only, never pickle.

## Scope

- `models-domain`: `CatalogEntry { id, kind: Stt | Tts | Vad | Llm | Embedding, engine, files: [{ url, size, sha256 }], license, license_url, min_hardware, languages }`, `InstalledModel`, and a pure `recommend(probe, catalog, prefs)` function.
- The catalog ships inside the app as a versioned file. Catalog updates arrive only with app updates; no remote catalog fetch in v0.1.
- `models-infrastructure`: HTTP downloader (`reqwest`, HTTPS only, Range requests for resume, `.part` files, SHA-256 streamed during download, atomic rename on success), a disk-space check before starting, and a store-backed `installed_models` table (device-local, not synced).
- `models-application`: `ListCatalog`, `Recommend`, `Download`, `Cancel`, `Delete`, `Benchmark` (runs 5 s of a bundled sample through the candidate and reports real-time factor).
- Events: `ModelDownloadProgress { id, bytes, total }` throttled to 4 Hz, `ModelInstalled`, `ModelFailed`.
- `src/models` UI in the main window: recommended pick with a one-line reason, the catalog by kind with size and license, progress bars, cancel, delete, "Use for dictation", and the benchmark result.

Out of scope: onboarding's model step (wired by #007's flow later), LLM GGUF defaults (#048), TTS voices beyond catalog plumbing (#025), installer-bundled VAD (#014, #075).

## Acceptance criteria

`features/core/model-manager.feature` against a local HTTP fixture server, and `features/ui/models.feature` with fake IPC.

```gherkin
Feature: Model manager

  Scenario: Recommend for a mid-range GPU
    Given the probe reports an RTX 4060 with 8 GB VRAM
    When I open Models
    Then the recommendation is whisper large-v3-turbo on CUDA with the reason "Fast on your NVIDIA GPU"

  Scenario: Recommend for a CPU-only laptop
    Given the probe reports no usable GPU and AVX2
    Then the recommendation is a small model with the reason "Runs well on your CPU"

  Scenario: An interrupted download resumes
    Given a 500 MB model download stopped at 300 MB
    When I choose "Resume"
    Then the request asks for bytes from 300 MB onward and the model installs

  Scenario: A tampered file is rejected
    Given the server returns a file whose SHA-256 differs from the catalog
    When the download completes
    Then the model is not installed, the partial file is deleted
    And I see "Download failed verification. Nothing was installed."

  Scenario: Not enough disk space
    Given 200 MB free and a 1.6 GB model
    When I choose "Download"
    Then nothing is downloaded and I see "Needs 1.6 GB free on C:"

  Scenario: Offline
    Given there is no network
    When I choose "Download"
    Then I see "You're offline. Installed models keep working."
    And installed models remain usable

  Scenario: Deleting the model in use
    Given large-v3-turbo is the dictation model
    When I delete it
    Then I am asked to confirm and dictation switches to the next installed model, or reports "model_missing" if none
```

## Design notes

- `arch.toml`: `models-domain` allow `["kernel"]`; `models-application` allow `["models-domain", "kernel"]`; `models-infrastructure` allow `["models-application", "models-domain", "kernel", "store"]`, ban `tauri`, `windows`. `Benchmark` and the hardware probe go through a `ModelRuntime` port in `models-application`, implemented by `infer-client` (#015).
- Hosting: catalog URLs point at the upstream publishers (for example Hugging Face repositories); verify each URL is a pinned revision, not `main`, so the SHA-256 stays valid.
- Files allowed: `.bin`/`.gguf` (GGML/GGUF), `.onnx`, `.safetensors`, plus tokenizer and config text files. Reject anything else.
- Privacy: downloads are `DataClass::Public` but are still off-device calls. Record each in the audit log once #074 exists (provider host, bytes, no content).
- Paths: never build a file path from catalog strings without validating the file name (no separators, no `..`).
- UI copy shows each model's license and links to its card before download.

## QA procedure

1. Start with an empty models folder and open Models. A recommendation appears that matches your hardware.
2. Start the download and turn Wi-Fi off halfway. The item shows "Paused: offline". Turn Wi-Fi on and choose Resume. It finishes without restarting from zero.
3. Corrupt a byte in a `.part` file during a download (dev build). Verification fails and nothing is installed.
4. Run "Benchmark". A real-time factor appears in a few seconds.
5. Delete the active model. The confirmation names the fallback model.
6. Check `%LOCALAPPDATA%\<app>\models`. Only verified files remain; no `.part` files after cancel.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
