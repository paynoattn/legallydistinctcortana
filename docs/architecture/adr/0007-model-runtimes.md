# 0007. Model runtimes

Status: proposed

## Decision

- **STT:**
  - **Default:** whisper.cpp (CPU, CUDA, Vulkan).
  - **Speed and NPU path:** sherpa-onnx (Parakeet, streaming Zipformer) on ONNX Runtime execution providers: DirectML, CUDA, QNN (Snapdragon NPUs), OpenVINO (Intel NPUs).
  - **Cloud:** opt-in only.
- **VAD and wake word:** Silero VAD and openWakeWord on ONNX Runtime CPU, **in-process** in `voice-infrastructure` (tiny models; the latency budget does not allow an IPC hop per 20 ms frame). Everything GPU- or NPU-bound stays in `infer-host`.
- **TTS:**
  - **Default:** Kokoro-82M.
  - **Low-end hardware:** Piper.
  - **Moods:** a fine-tuned expressive model, once it passes its evaluation gate.
  - **Zero-download fallback:** Windows OneCore voices.
- **LLM:**
  - Any OpenAI-compatible server (Ollama, LM Studio, llama-server, Foundry Local, vLLM), plus the Anthropic and Gemini APIs.
  - A bundled, supervised `llama-server`, so the app works out of the box with a downloaded GGUF.
  - Our fine-tuned persona model ships as GGUF.
- **No GPL code is linked into our binaries.** espeak-ng (GPL-3.0) is the usual phonemizer for Kokoro and Piper. Use permissively licensed G2P instead (Kokoro's misaki for English plus a pronunciation dictionary; per-language alternatives to verify). If a language needs espeak-ng, run it as a separate, user-installed executable, never linked, and only after a license review. Piper voices that cannot be served without espeak-ng are dropped. Spike [#060](../../issues/060-spike-expressive-tts-base.md) covers the G2P question too.
- **Placement:** every STT, TTS, and LLM engine lives in `infer-host` ([ADR 0002](0002-process-model.md)). Models are downloaded on demand, each with a pinned SHA-256.

## Consequences

- NPU support comes through ONNX Runtime execution providers, with per-vendor `infer-host` builds.
- Fine-tuned models are an independent ML track with explicit evaluation gates. The product works without them.
