# Voice in, voice out

Covers dictation (speech to text), read-aloud and spoken replies (text to speech with moods), wake word, and the overlay that shows what the voice system is doing. Decision record: [ADR 0007](adr/0007-model-runtimes.md).

## Budgets

| Path                                                   | Budget (p95, mid-range laptop: RTX 4060 or Snapdragon X Elite) |
| ------------------------------------------------------ | -------------------------------------------------------------- |
| Hotkey down → overlay visible                          | 50 ms (overlay window pre-created, hidden)                     |
| Hotkey down → first audio frame buffered               | 30 ms (capture stream kept open while armed)                   |
| Release → text inserted (≤ 15 s utterance, no cleanup) | 500 ms GPU, 1.5 s CPU                                          |
| Release → text inserted (with local LLM cleanup)       | 1.2 s GPU                                                      |
| "Read aloud" → first audio                             | 300 ms (first sentence streams while the rest synthesizes)     |
| Wake word false accepts                                | < 1 per 8 hours of mixed office audio                          |
| Idle cost, models unloaded                             | < 1% CPU, < 300 MB RAM for the whole app                       |

Latency budgets are asserted in a `bench` CI job, not in `make check`.

## Dictation pipeline

```
hotkey (low-level keyboard hook: key down and key up)
  └─► voice-application::DictationSession (state machine in voice-domain)
        Armed → Listening → Finalizing → Inserting → Done | Cancelled | Failed
  capture (cpal / WASAPI shared mode, 16 kHz mono f32, 20 ms frames)
  └─► ring buffer ─► Silero VAD (in-process, ONNX Runtime CPU, ~1 ms per frame)
                      └─► segments ─► infer-host STT (streaming partials when the engine supports it)
  final text ─► cleanup (deterministic voice-domain rules, then optional LLM pass via the models router)
             ─► target-aware formatting (foreground app class: code editor, chat, email, terminal)
             ─► platform TextInjector (paste with clipboard restore, or UIA ValuePattern)
```

- **Modes:** push-to-talk (hold), toggle (tap to start and stop, optional VAD auto-stop), assistant (speech goes to the assistant instead of the cursor), translate (speak in language A, paste in language B).
- **Deterministic cleanup (domain):** spoken punctuation ("new line", "comma"), filler removal ("um", "uh"), number and date normalization, the user's dictionary (replacements and casing), and snippets ("insert my address").
- **LLM cleanup (optional):** a short prompt to the routed `dictation-cleanup` task, local by default. It fixes grammar and formatting only and must not add content. The edit is bounded: if the edit distance exceeds 35% of the input, the raw text wins. Skipped for utterances under 4 words.
- **Target awareness:** `platform::foreground_app()` returns the process name and window class. For example: no trailing period in chat apps, no auto-capitalization in terminals, and identifier casing preserved in code editors.
- **Insertion:** default is clipboard paste. Save the clipboard, set the text, send Ctrl+V via `SendInput`, and restore after 150 ms. Elevated windows (UIPI) reject input from a non-elevated app. Detect that and fall back to "Copied, press Ctrl+V".
- **History:** off by default. When on, transcripts are stored encrypted. Audio is optional and kept 7 days by default.

## Speech-to-text engines (in infer-host)

| Engine                     | Models                                                                | Acceleration                                                                      | Notes                                                                             |
| -------------------------- | --------------------------------------------------------------------- | --------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| whisper.cpp (`whisper-rs`) | tiny → large-v3-turbo, distil                                         | CPU (AVX2), CUDA, Vulkan (AMD, Intel, NVIDIA)                                     | Default. Turbo on Vulkan is the best general choice.                              |
| sherpa-onnx (`sherpa-rs`)  | NVIDIA Parakeet TDT 0.6B v3, streaming Zipformer                      | ONNX Runtime: CPU, DirectML, CUDA, QNN (Snapdragon NPU), OpenVINO (Intel NPU/GPU) | Fastest English, true streaming partials, and the NPU path.                       |
| Cloud (opt-in)             | OpenAI, Groq, Deepgram, any OpenAI-compatible `/audio/transcriptions` | n/a                                                                               | User's key. The privacy router must allow `DataClass::Voice` to leave the device. |

**Hardware selection:** on first run `infer-host --probe` reports GPUs (DXGI), NPUs (QNN or OpenVINO enumeration), VRAM, and CPU features. The model manager recommends an engine and model, then benchmarks a 5 s sample to confirm. The user can override.

**Model manager** (models context): catalog entries pin URL, size, SHA-256, license, and minimum hardware. Downloads are resumable and verified, and are stored in `%LOCALAPPDATA%\<app>\models`. The installer ships only the VAD and the default wake-word model (a few MB each).

## Text-to-speech and moods

### Requests

`speech-application::Speak { text, voice, mood, rate, source }`, where `source` is `ReadAloud`, `AssistantReply`, `Alert`, or `Briefing`. The queue has priorities: alerts preempt replies, and replies preempt read-aloud and briefings (same priority, first come first served). New user speech or the stop hotkey cancels playback (barge-in). Long text is split into sentences, synthesized one sentence ahead, and streamed to playback.

### Moods

`kernel::Mood` = `Neutral | Calm | Cheerful | Alert | Urgent | Angry | Mad | Whisper | Deadpan`. ("Mad" is angry turned up to theatrical: louder, faster, clipped.) `speech-domain` maps each mood to an engine-neutral **style vector**:

| Field                                  | Meaning                                                               |
| -------------------------------------- | --------------------------------------------------------------------- |
| `emotion`                              | Label for expressive engines (tag, instruction, or reference clip id) |
| `intensity`                            | 0.0–1.0 (exaggeration or guidance weight for expressive engines)      |
| `rate`, `pitch_semitones`, `energy_db` | Prosody fallback for engines without emotion control                  |
| `pause_scale`                          | Multiplier for sentence and comma pauses                              |

**Shipped so far:** `speech-domain::Prosody` implements the prosody fallback as three values per mood: speed (applied by resampling, so pitch rises with speed), gain in dB, and the pause after each sentence. Mad, for example, is 1.25× speed, 0 dB, and an 80 ms pause. Gains never go above 0 dB: synthesizers already peak near full scale, so Neutral sits at −6 dB and the loud moods use that headroom instead of clipping. The table is in `crates/speech-domain/src/prosody.rs`. Because speed and pitch are coupled, the moods sound approximate until the expressive model lands.

Each engine adapter translates the style vector into what it supports. The UI marks each mood as "native" or "approximated" for the current voice.

### Engines (in infer-host)

| Engine                                   | Role                                                                                                                                                                                                                                                                                                                                 | License                                             | Mood support                 |
| ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------- | ---------------------------- |
| Wyoming server (`wyoming-piper`, remote) | **First engine shipped** (read-aloud MVP): Piper running on a machine you own, reached over the LAN or Tailscale (`VOICE_HOST`, `VOICE_TTS_PORT`, `VOICE_TTS_VOICE` for the default voice). Its voices are listed with Wyoming `describe` and picked in the read-aloud window. The same protocol serves Whisper for dictation later. | MIT (Piper; voices vary)                            | Prosody fallback             |
| Kokoro-82M (ONNX)                        | Default voice, fast on CPU and any GPU                                                                                                                                                                                                                                                                                               | Apache-2.0                                          | Prosody fallback             |
| Piper                                    | Low-end hardware, many languages                                                                                                                                                                                                                                                                                                     | MIT (voices vary)                                   | Prosody fallback             |
| **Expressive model** (fine-tuned, below) | Mood voice for read-aloud and the assistant persona                                                                                                                                                                                                                                                                                  | Weights and training data must allow redistribution | Native emotion and intensity |
| Windows OneCore voices (SAPI)            | Zero-download fallback                                                                                                                                                                                                                                                                                                               | OS                                                  | Rate and pitch only          |

### The fine-tuned expressive model (ML track)

Goal: one assistant voice that can say the same sentence calm, alert, urgent, angry, or mad, in real time on a mid-range GPU, and later on an NPU.

- **Bases to evaluate** (spike [#060](../issues/060-spike-expressive-tts-base.md)): Chatterbox (emotion-exaggeration control), Orpheus-style LLM TTS (emotion tags, GGUF through llama.cpp plus a neural codec), StyleTTS2 (reference-style transfer), and CosyVoice-family instruct TTS. Compare license (code and weights), real-time factor on reference hardware, ONNX or GGUF export, and how controllable emotion is.
- **Data:** only data whose license permits redistributing derived weights. Several well-known emotional speech corpora are non-commercial or research-only and are excluded. Preferred route: a consenting voice actor records 10–20 hours across the moods under CC-BY, plus synthetic augmentation. The dataset card records consent and license.
- **Training:** in `training/tts/`, LoRA or a full fine-tune depending on the base. The model ships only after passing evaluation:
  - an emotion classifier agrees with the target mood at least 80% of the time
  - the UTMOS quality estimate stays within 0.2 of the base model
  - word error rate (measured by transcribing the output with whisper) stays within 2 points of the base
  - real-time factor is ≤ 0.3 on an RTX 4060
- **Shipping:** the model manager downloads the weights like any other model, with a model card and checksum. Until a model passes evaluation, moods use the prosody fallback on Kokoro.

### The persona model (assistant voice replies)

A small local LLM (3–4B class) fine-tuned with LoRA in `training/persona/` to answer in short, spoken-style sentences in the assistant's personality, and to emit our tool-call format reliably. It handles quick spoken turns ("what's next on my calendar"). Anything long or hard is routed to the user's chosen bigger model ([assistant](assistant.md)). Shipped as GGUF, served by the bundled llama-server or by Ollama. Evaluation: tool-call validity ≥ 98% on the held-out set, median spoken reply ≤ 2 sentences, and a refusal and safety suite pass.

## Wake word

- Off by default. When on, an always-listening capture runs openWakeWord (ONNX, CPU) at about 1–2% of one core. Audio never leaves the ring buffer unless the wake word fires.
- The phrase is "Hey <brand name>". The model is trained on synthetic positives from our TTS plus negatives (spike [#061](../issues/061-spike-wake-word.md)). Users can add a personal verifier from 5 samples.
- While the assistant speaks, the wake word is suppressed unless echo cancellation is active (WebRTC AEC, post-MVP). The barge-in hotkey always works.

## Overlay states

The overlay is a 160×160 px transparent, always-on-top, non-activating, click-through window in the bottom-right corner. It sits above the taskbar inside the work area, follows the monitor the cursor is on, and is DPI-aware. Design: [`../design/artboards/`](../design/artboards/).

| State     | Trigger                   | Visual (the brand mark animates)                  | Sound (optional) |
| --------- | ------------------------- | ------------------------------------------------- | ---------------- |
| Hidden    | Idle                      | Nothing on screen                                 |                  |
| Armed     | Hotkey down (≤ 50 ms)     | Mark scales in                                    | Soft tick        |
| Listening | First frame captured      | Reacts to input level (RMS, 30 fps)               |                  |
| Thinking  | Release or VAD end        | Slow orbit or pulse                               |                  |
| Inserted  | Text injected             | Brief check, fades out over 600 ms                |                  |
| Speaking  | TTS playing               | Reacts to output level; the mood tints the accent |                  |
| Error     | Mic missing, model failed | Shake plus a one-line reason; click opens the fix | Error tone       |

Animation runs in the overlay webview with CSS and `requestAnimationFrame`, driven by level events throttled to 30 Hz. `prefers-reduced-motion` and the in-app setting switch to opacity-only transitions.
