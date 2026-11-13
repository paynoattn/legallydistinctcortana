# 080. Mood TTS dataset: consent, license, recording plan

| Milestone | Size | Depends on | Labels                             |
| --------- | ---- | ---------- | ---------------------------------- |
| ML track  | L    | #060       | area/ml, area/speech, type/feature |

## Story

As the maintainer of the assistant's voice, I want a speech dataset of one consenting voice actor saying the same kinds of sentences in every mood, under a license that lets us publish the fine-tuned weights, so that the mood voice can be trained, shipped, and redistributed without legal doubt.

## Context

[voice-and-speech: the fine-tuned expressive model](../architecture/voice-and-speech.md#the-fine-tuned-expressive-model-ml-track): only data whose license permits redistributing derived weights; several well-known emotional speech corpora are non-commercial or research-only and are excluded; preferred route is a consenting voice actor recording 10–20 hours across the moods under CC-BY, plus synthetic augmentation, with a dataset card. [ADR 0010](../architecture/adr/0010-license.md): our weights can be no more permissive than their training data. The base model chosen in spike [#060](060-spike-expressive-tts-base.md) sets the format details (sample rate, transcript conventions, parallel sentences, hours per mood). The [MVP plan](../product/mvp-plan.md#risks) lists "commission data" as the mitigation for scarce clean expressive data.

## Scope

- **Consent and contract:** a written agreement with the voice actor covering: recording for a synthetic voice; publication of the recordings under CC BY 4.0 (or CC0 if the actor agrees); publication of models trained on them; attribution wording; fair payment; what the voice may not be used for in our project (impersonating the actor, political or adult content in our releases); and a withdrawal clause that is honest about what cannot be recalled once weights are published. Reviewed by someone qualified in the actor's jurisdiction.
- **Moods:** `kernel::Mood` = Neutral, Calm, Cheerful, Alert, Urgent, Angry, Mad, Whisper, Deadpan, each with a direction sheet and reference examples for the actor (for example Alert: clear, raised, steady; Mad: Angry turned up to theatrical, louder, faster, clipped; Deadpan: flat pitch, even rate).
- **Script:** phonetically balanced sentences from public-domain or CC0 text, plus assistant-specific lines (alerts, reminders, meeting heads-ups, read-aloud paragraphs, numbers, times, names). A shared core set recorded in every mood (parallel data), plus mood-specific lines.
- **Recording plan:** target 10–20 hours total (final split per mood from #060); 48 kHz, 24-bit, mono, treated room, fixed mic and distance, noise floor and peak targets, session length limits to protect the actor's voice (Mad and Whisper sessions short).
- **Processing pipeline** in `training/tts/data/`: loudness normalization, silence trimming, segmentation, forced alignment check, transcript validation with whisper (flag clips with WER above a threshold), and per-clip mood labels from the session sheet.
- **Synthetic augmentation:** only from models whose license and terms allow using their output to train and redistribute a model (verify each); recorded in the card.
- **Audit of public corpora:** a table of known emotional corpora with their license as stated by the source, each marked usable or excluded with a link. Verify every row; do not assume.
- **Dataset card** (`training/tts/data/DATASET_CARD.md` and on the hosting platform): provenance, consent summary, license, hours per mood, recording conditions, known limitations, and contact.
- **Held-out set:** 5% of the core sentences in every mood never used for training; used by [#081](081-mood-tts-finetune.md) for evaluation and to train or validate the mood classifier.

Out of scope: training (#081); multiple speakers or languages (later); the wake-word dataset ([#061](061-spike-wake-word.md)).

## Acceptance criteria

```gherkin
Feature: Mood TTS dataset

  Scenario: Every clip has provenance
    Given the processed dataset manifest
    When the validation tool checks it
    Then every clip has a speaker id, session id, mood, transcript, license, and consent reference

  Scenario: Excluded sources are rejected
    Given a manifest that includes a clip whose source license is marked "non-commercial"
    When the validation tool checks it
    Then it exits non-zero and names the clip and its license

  Scenario: Transcripts match the audio
    Given a clip whose whisper transcript differs from its script text by more than 10% WER
    When the pipeline processes it
    Then the clip is flagged for review and excluded from the training split

  Scenario: Mood coverage
    When the coverage report runs
    Then it lists hours per mood and fails if any mood is below the minimum set from #060

  Scenario: The held-out set does not leak
    When the split tool builds train and test sets
    Then no sentence id in the test set appears in the training set in any mood
```

## Design notes

- **Code location:** `training/` is Python and adopts the python-fastapi starter's gates through its own `Makefile` implementing the [gate contract](../engineering/gate-contract.md): ruff with ALL rules, mypy `--strict`, coverage ≥ 90%, CRAP ≤ 6, import-linter for layering, and mutation testing. Acceptance features for the tools live in `training/features/`.
- **Layout:** `training/tts/data/{domain,application,infrastructure}` for manifest models and validation rules (pure), use cases (validate, split, report), and adapters (audio I/O with `soundfile`, whisper transcription).
- **Storage:** raw recordings are not committed to git; they live on the dataset host (for example a Hugging Face dataset repository under CC BY 4.0) with SHA-256 manifests in the repository.
- **Privacy:** the actor's personal details stay in the signed contract, not in the dataset; the card uses a stage name if the actor wishes.
- **Classifier note:** standard speech-emotion classifiers have no labels for Alert, Urgent, Mad, Whisper, or Deadpan, so #081's "emotion classifier agrees ≥ 80%" gate needs a classifier trained or validated on this held-out set plus a human listening panel; budget recording time for it.

## QA procedure

1. Read the signed consent agreement and confirm each item in Scope is covered; a second maintainer signs off.
2. Listen to 10 random clips per mood from the processed set and confirm the mood is recognizable and the audio is clean.
3. Run `make check` in `training/` and the dataset validation, coverage, and split commands; confirm they pass.
4. Open the dataset card on the hosting platform and confirm license, hours per mood, and consent summary are present and match the manifest.

## Definition of done

- [ ] Acceptance scenarios in `training/features/` pass in `training/`'s `make check`
- [ ] `make mutate SCOPE=changed` at 100% in `training/`
- [ ] Consent agreement signed and stored privately; dataset card published
- [ ] voice-and-speech.md updated with the dataset facts
