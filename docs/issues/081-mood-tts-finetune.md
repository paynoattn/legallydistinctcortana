# 081. Mood TTS fine-tune and evaluation harness

| Milestone | Size                          | Depends on | Labels                             |
| --------- | ----------------------------- | ---------- | ---------------------------------- |
| ML track  | XL (split before work starts) | #080       | area/ml, area/speech, type/feature |

## Story

As a user who selects text and chooses "Read aloud → Alert" or hears a reminder in the Urgent mood, I want one natural assistant voice that really sounds alert, urgent, angry, mad, calm, cheerful, whispered, or deadpan, so that moods are more than pitch and speed changes.

## Context

[voice-and-speech: the fine-tuned expressive model](../architecture/voice-and-speech.md#the-fine-tuned-expressive-model-ml-track) and [ADR 0007](../architecture/adr/0007-model-runtimes.md). The base model and export path come from spike [#060](060-spike-expressive-tts-base.md), the data from [#080](080-mood-tts-dataset.md). It is a "Should" for v0.1 and ships only if it passes its evaluation; until then moods use the prosody fallback on Kokoro ([#027](027-moods.md)).

## Scope

- **Training** in `training/tts/`: LoRA or full fine-tune of the #060 base on the #080 dataset, with mood as a control input (tag, instruction, or style embedding, as the base supports) and an intensity control mapped from the style vector's `intensity`.
- **Evaluation harness** (`training/tts/eval/`) that renders the held-out sentences in every mood and computes the gates from voice-and-speech.md:
  - mood classifier agrees with the target mood ≥ 80%
  - UTMOS within 0.2 of the base model
  - WER (whisper transcription) within 2 points of the base model
  - real-time factor ≤ 0.3 on an RTX 4060
- **Mood classifier:** trained or calibrated on #080's held-out recordings for all nine moods (off-the-shelf emotion classifiers lack Alert, Urgent, Mad, Whisper, and Deadpan), validated against a blind human listening panel of at least 5 people; the panel result is reported next to the automatic score.
- **Export** to the runtime chosen in #060 (ONNX, or GGUF plus an ONNX codec), with a parity test between the Python and exported outputs.
- **Release artifacts:** weights with a model card (base, data, license, evaluation results, intended use, limitations), SHA-256, and a catalog entry for the model manager ([#018](018-model-manager.md)) marking every mood as "native".
- **infer-host adapter** for the exported model behind the existing TTS engine port, with sentence streaming so the 300 ms first-audio budget holds.

Out of scope: more voices or languages; NPU export ([#088](088-npu-builds.md)); voice cloning features of any kind.

### Proposed split (XL must be split before work starts)

1. **081a Evaluation harness and mood classifier** (gates computed on the base model and on recordings; no training yet).
2. **081b Fine-tune recipe and first checkpoint** (training code, configs, reproducible run).
3. **081c Export and parity** (ONNX/GGUF export, parity tests, RTF on reference hardware).
4. **081d infer-host adapter and catalog entry** (Rust engine adapter, streaming, model card, release).

## Acceptance criteria

```gherkin
Feature: Mood TTS evaluation gate

  Scenario: A passing checkpoint
    Given a checkpoint with mood agreement 84%, UTMOS 0.1 below base, WER 1.2 points above base, and RTF 0.22
    When "make eval-tts" runs on it
    Then it exits 0 and writes a report with every metric per mood

  Scenario: A failing metric blocks release
    Given a checkpoint with mood agreement 76%
    When "make eval-tts" runs on it
    Then it exits non-zero and the report names "mood agreement" with 76% against the 80% gate

  Scenario: Export matches training output
    Given a checkpoint and its exported model
    When the parity test renders 20 held-out sentences with both
    Then the mel-spectrogram distance stays under the threshold set in the eval config

  Scenario: The app uses native moods when the model is installed
    Given the mood model is installed through the model manager
    When I choose "Read aloud → Urgent" on a paragraph
    Then the voice picker marks Urgent as "native" and the first audio plays within 300 ms on an RTX 4060

  Scenario: Fallback when the model is missing
    Given the mood model is not installed
    When I choose "Read aloud → Urgent"
    Then Kokoro reads it with the prosody fallback and the picker marks Urgent as "approximated"

  Scenario: The license travels with the weights
    When the release artifacts are built
    Then the model card states the base model license, the dataset license, and attribution, and the catalog entry carries the same license id
```

## Design notes

- **Python gates:** `training/` adopts the python-fastapi starter's gates (ruff with ALL rules, mypy `--strict`, coverage ≥ 90%, CRAP ≤ 6, import-linter, mutation testing) through its own `Makefile` implementing the [gate contract](../engineering/gate-contract.md). GPU training scripts are thin entry points (excluded from coverage like composition roots); the logic they call is tested.
- **Reproducibility:** configs in `training/tts/configs/`, seeds fixed, dataset manifest hash and base-model revision recorded in every report; training runs off CI on a rented or donated GPU; evaluation can run on the reference RTX 4060.
- **Rust side:** a new engine adapter in `infer-host` (only it may depend on ORT or llama.cpp bindings), message types in `infer-protocol`; `speech-domain`'s style vector maps `emotion` and `intensity` to the model's control input. `arch.toml` unchanged unless a new crate is added.
- **Safety:** no voice cloning input is exposed in the product; the model card states that the voice is synthetic and names the actor only as they agreed.
- **Weights license:** no more permissive than the base and data allow ([ADR 0010](../architecture/adr/0010-license.md)).

## QA procedure

1. On the reference RTX 4060 laptop, run `make eval-tts` on the release candidate; confirm exit 0 and read the per-mood report.
2. Install the candidate through Settings → Models (from a local catalog override). Open Settings → Voice and play the preview for each of the nine moods; confirm each sounds distinct and matches its name.
3. In Edge, select a paragraph and use Read aloud → Alert, Angry, Mad, Whisper, and Deadpan; confirm audio starts quickly and stays smooth to the end.
4. Uninstall the model and confirm moods fall back to "approximated" without errors.
5. Run a 5-person blind listening test with 20 clips (the harness generates the sheet) and record agreement in the report.

## Definition of done

- [ ] Split into 081a–081d before work starts
- [ ] Acceptance scenarios pass in `training/`'s `make check` and the app's `make check`
- [ ] `make mutate SCOPE=changed` at 100% in both
- [ ] QA procedure passes; evaluation report and model card published with the weights
- [ ] voice-and-speech.md and ADR 0007 updated with the shipped model
