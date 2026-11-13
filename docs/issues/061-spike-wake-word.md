# 061. Spike: Wake word engine and custom phrase

| Time box | Needed by | Labels                 |
| -------- | --------- | ---------------------- |
| 3 days   | #057      | area/voice, type/spike |

## Question

Which on-device engine should detect "Hey <brand name>", how do we produce a model for a phrase no one has trained yet, and can that model meet the budget of fewer than 1 false accept per 8 hours of mixed office audio at about 1–2% of one CPU core, with a license that lets the installer ship it?

## Why it matters

[ADR 0007](../architecture/adr/0007-model-runtimes.md) names openWakeWord on ONNX Runtime CPU, and [voice-and-speech](../architecture/voice-and-speech.md#wake-word) says the model is trained on synthetic positives from our TTS plus negatives, with an optional personal verifier from 5 samples, and that the installer ships the default wake-word model. #057 cannot start until we know the engine, the training pipeline, and whether the shipped files are redistributable. The phrase needs the product name from [#005](005-finalize-brand-and-identifier.md) before a model is trained for release; the spike itself uses a placeholder phrase of similar length and phonetics.

## Options to evaluate

| Option                           | Known (to verify)                                                                                                                                                                                             | Must verify                                                                                                                                                                                                                                                                                               |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **openWakeWord**                 | Apache-2.0 code; shared melspectrogram and embedding front end in ONNX; small per-phrase classifier; a documented pipeline trains a custom phrase from synthetic TTS clips; supports per-user verifier models | License of the shared front-end models and of the pretrained phrase models (the project has published some under non-commercial terms); licenses of the negative datasets the training notebook uses, because they bound the license of our trained classifier; false-accept rate for a 3-syllable phrase |
| **sherpa-onnx keyword spotting** | Open-vocabulary keyword spotting with an Apache-2.0 runtime; no per-phrase training                                                                                                                           | Model license; accuracy and false accepts for a short phrase versus a trained detector; CPU cost; it would share the sherpa-onnx dependency with #017                                                                                                                                                     |
| **microWakeWord**                | Apache-2.0; designed for tiny devices; trains per phrase                                                                                                                                                      | Whether its TFLite models convert cleanly to ONNX; training data licenses                                                                                                                                                                                                                                 |
| **Porcupine** (Picovoice)        | Proprietary; needs an access key                                                                                                                                                                              | Listed only to reject: an account and key conflict with "no account" in the [vision](../product/vision.md).                                                                                                                                                                                               |

## Method

1. **License audit** of every file that would ship (front-end models, classifier, verifier) and of every dataset used to train them. Record links at pinned commits.
2. **Train a placeholder phrase** with the openWakeWord pipeline using positives from Kokoro and Piper voices (Apache-2.0 and MIT) across speakers, speeds, and room impulse responses, and negatives only from datasets whose license allows redistributing the trained model. Note the GPU hours.
3. **Build a test set we own:** 8 hours of office audio recorded with consent (talk, calls, music, keyboard, a TV), plus 200 positive utterances from at least 10 real speakers on laptop mics and a headset.
4. **Measure** for each engine: false accepts per 8 hours, false-reject rate at that threshold, CPU per core on a mid-range laptop (16 kHz, 80 ms hops), and memory. Repeat with a 5-sample personal verifier.
5. **Integration check:** run the chosen model from Rust with the `ort` crate in a test binary to confirm it fits beside Silero VAD in the capture path (or in `infer-host`, if the front end is too heavy for the core process).

Proposed pass bar, to confirm in the ADR: false accepts < 1 per 8 hours and false rejects ≤ 10% at the same threshold. voice-and-speech.md states only the false-accept budget.

## Output

- An update to [ADR 0007](../architecture/adr/0007-model-runtimes.md) (VAD and wake word) naming the engine, the training pipeline, the shipped files and their licenses, and the measured rates.
- A `training/wakeword/` plan (Python, python-fastapi starter gates) if we train our own classifier, added to [#057](057-wake-word.md) or as a new ML issue.
- Changes to [#057](057-wake-word.md): where detection runs (core or `infer-host`), the verifier enrollment flow, and the threshold settings.
- If no option passes the license audit, #057 stays "Could" and the push-to-talk assistant hotkey remains the only voice trigger.
