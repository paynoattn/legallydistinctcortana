# 060. Spike: Expressive TTS base model selection

| Time box | Needed by  | Labels                           |
| -------- | ---------- | -------------------------------- |
| 1 week   | #080, #081 | area/ml, area/speech, type/spike |

## Question

Which open text-to-speech model should the assistant's mood voice be fine-tuned from, so that one voice can say the same sentence in every `kernel::Mood` (Neutral, Calm, Cheerful, Alert, Urgent, Angry, Mad, Whisper, Deadpan) with native emotion control, runs in real time in `infer-host`, and has code and weights whose licenses let us redistribute a fine-tuned derivative?

A base qualifies only if all of these hold:

1. Code and weights allow commercial use, modification, and redistribution of derived weights (ADR 0010: our weights can be no more permissive than their inputs).
2. Real-time factor ≤ 0.3 on an RTX 4060 for the fine-tuned size, and a credible path to first audio within the 300 ms read-aloud budget (sentence streaming).
3. Exportable to a runtime `infer-host` can host without Python: ONNX (ONNX Runtime) or GGUF (llama.cpp) plus any codec or vocoder in ONNX.
4. Emotion or style is controllable by an input (tag, instruction, reference clip, or exaggeration weight) that the `speech-domain` style vector can drive (`emotion`, `intensity`).

## Why it matters

The "Should" goal for v0.1 is an expressive mood voice that passes its evaluation gate ([voice-and-speech](../architecture/voice-and-speech.md#the-fine-tuned-expressive-model-ml-track)). The base decides how much data #080 must record, in what format (single speaker or multi-speaker, parallel sentences or not, transcripts with tags or with style descriptions), what training code #081 writes, and which export path `infer-host` needs. Picking a base with an unclear weight license would make every later step unshippable.

## Options to evaluate

Everything in the "Known" column is from public model cards and papers and must be re-checked against the license files at the pinned commit.

| Candidate                                                      | Known (to verify)                                                                              | Must verify                                                                                                                                                    |
| -------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Chatterbox** (Resemble AI)                                   | Emotion-exaggeration control; reported MIT license; ships an audio watermarker                 | Weight license text; what the watermarker does and whether it can or should stay; ONNX export; RTF on a 4060; fine-tuning recipe availability                  |
| **Orpheus-style LLM TTS**                                      | LLM backbone emitting neural-codec tokens (SNAC); emotion tags in text; GGUF through llama.cpp | Backbone weight license (Llama-derived bases carry the Llama community license terms); codec license and ONNX export; streaming latency with llama.cpp + codec |
| **StyleTTS 2**                                                 | Reference-style transfer; small and fast; MIT code                                             | Pretrained weight conditions (the repository has asked for synthetic-speech disclosure; check whether it binds us); how well intensity can be steered          |
| **CosyVoice family** (instruct TTS)                            | Natural-language instruction for emotion; reported Apache-2.0                                  | Weight license per version; size and RTF; ONNX export of the flow-matching and vocoder parts                                                                   |
| **Parler-TTS**                                                 | Style controlled by a text description; reported Apache-2.0                                    | Whether description control reaches "mad" and "deadpan"; RTF; ONNX export                                                                                      |
| **Kokoro-82M** (baseline, not a candidate unless fine-tunable) | Apache-2.0 weights; our default voice                                                          | Whether a style-conditioned fine-tune is practical, which would keep one voice family                                                                          |

Excluded unless the license check says otherwise: any base whose weights are non-commercial or research-only, or that was trained on such data with that restriction carried into the weights (several recent zero-shot TTS models trained on non-commercial corpora fall here; verify each one rather than assume).

## Method

1. **License audit first.** For each candidate, record the code license, the weight license, the training-data statement, and any use restrictions, with links to the files at a pinned commit. Drop failures before benchmarking.
2. **Harness** in `training/tts/spike/` (Python, the python-fastapi starter's gates: ruff with ALL rules, mypy `--strict`). One script per candidate behind a common `synthesize(text, mood) -> wav` interface.
3. **Fixed prompt set:** 40 sentences × 9 moods, including short alerts ("Your meeting starts in two minutes"), long read-aloud paragraphs, numbers, and names.
4. **Measure on reference hardware** (RTX 4060 laptop, and CPU-only on the same machine):
   - RTF and time to first audio (streaming where supported), VRAM, and model size
   - zero-shot mood control: a 5-person blind listening test (pick the mood from a list) and, where available, an automatic emotion classifier; note that standard classifiers have no labels for Alert, Urgent, Mad, Whisper, or Deadpan
   - quality: UTMOS and WER (whisper `large-v3-turbo` transcription) per mood
5. **Export check:** run the best two through ONNX Runtime or llama.cpp from a small Rust test binary to prove `infer-host` can host them.
6. **Fine-tune estimate:** for the best two, the published fine-tuning recipe, minimum data per mood, and GPU hours.

## Output

- A new ADR (next free number), "Expressive TTS base model", with the recommendation, the license evidence, the benchmark table, and the rejected options. It updates [ADR 0007](../architecture/adr/0007-model-runtimes.md) where the runtime changes (for example llama.cpp + codec instead of ONNX).
- Changes to [#080](080-mood-tts-dataset.md): hours per mood, sample format, transcript conventions (tags or descriptions), and whether parallel sentences are needed.
- Changes to [#081](081-mood-tts-finetune.md): training method (LoRA or full), export path, and the emotion classifier to use for the gate.
- A follow-up issue if `infer-host` needs a new engine adapter beyond those in #025.
