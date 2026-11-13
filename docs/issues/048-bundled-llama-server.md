# 048. Bundled llama-server and default local model

| Milestone    | Size | Depends on | Labels                                    |
| ------------ | ---- | ---------- | ----------------------------------------- |
| M5 Assistant | M    | #015, #018 | area/models, area/inference, type/feature |

## Story

As a privacy-conscious user who has never heard of Ollama, I want the assistant to work out of the box with a local model it downloads and runs itself, so that I get triage, summaries, and voice replies without installing anything else.

## Context

[ADR 0007](../architecture/adr/0007-model-runtimes.md) calls for "a bundled, supervised `llama-server`, so the app works out of the box with a downloaded GGUF". `infer-host` manages llama-server ([overview: runtime topology](../architecture/overview.md#runtime-topology)), and the model manager (#018) downloads weights with pinned SHA-256 hashes. The idle budget (< 300 MB RAM with models unloaded) means the model must unload when idle ([voice-and-speech budgets](../architecture/voice-and-speech.md#budgets)).

## Scope

- **Binaries:** ship llama.cpp's `llama-server` binaries matching each `infer-host` flavor (CPU+Vulkan, CUDA). Pin the version and the SHA-256 of each binary in the catalog. Download on first use if it is not in the installer.
- **Supervision** (`infer-host`):
  - Start llama-server bound to `127.0.0.1` on a free port, with a random `--api-key` per launch (verify the flag name for the pinned version).
  - Choose `--n-gpu-layers` and the context size from the hardware probe (#015): VRAM and the model size.
  - Health-check `/health`. Restart with backoff on crash (3 tries, then mark unhealthy and tell the router).
  - Stop on "Unload models", and after an idle timeout (default 10 min, setting).
- **Warm start:** the assistant hotkey (#051) and scheduled digest jobs (#045) send `EnsureLlm` to infer-host ahead of the call, so the first token waits for loading only once.
- **Default model:** one instruct GGUF in the 3–4B class, picked in this issue against:
  1. a license that allows redistribution and use
  2. tool-call validity on our `voice-turn` eval set (≥ 95%, the bar #082 raises to 98% for the persona model)
  3. speed on the reference laptop

  Candidates to verify: Phi-4-mini-instruct, the Qwen family (license varies by size), and Llama 3.2 3B (community license terms). Record the choice and evidence in the catalog's model card. Q4_K_M by default, Q8 when VRAM allows.

- **Router registration:** a "Built-in" provider of `ProviderLocation::ThisDevice` with tasks `voice-turn`, `dictation-cleanup`, `mail-triage`, `mail-summary`, `feed-summary`, and `reminder-parse`, plus `embedding` with a small embedding GGUF (for example nomic-embed or bge-small; verify the license) served by a second llama-server instance in `--embedding` mode.
- **Settings → Models:** state (Stopped / Loading / Ready / Failed with the reason), the model in use, VRAM used, "Unload now", and the idle timeout.

Out of scope: the persona fine-tune (#082), NPU builds (#088), and the installer packaging of binaries (#075).

## Acceptance criteria

```gherkin
Feature: Bundled llama-server

  Scenario: Works out of the box after the model download
    Given no Ollama or LM Studio is installed
    And the default model has been downloaded and verified
    When mail-summary is requested
    Then infer-host starts llama-server on 127.0.0.1 with an API key
    And the router uses the Built-in provider

  Scenario: Server is not reachable from the network
    Given llama-server is running
    When a request reaches its port from another machine on the LAN
    Then the connection is refused because it is bound to 127.0.0.1

  Scenario: Requests without the per-launch key are rejected
    When a local process calls the llama-server port without the API key
    Then it gets 401

  Scenario: Idle unload meets the memory budget
    Given llama-server has been idle for 10 minutes
    Then infer-host stops it
    And the app's total working set returns under 300 MB

  Scenario: Crash restarts with backoff, then reports
    Given llama-server crashes on every start
    When infer-host has retried 3 times
    Then the Built-in provider is marked unhealthy with "Local model failed to start"
    And the router falls back along the task's chain

  Scenario: Tampered binary or model is refused
    Given the downloaded GGUF's SHA-256 does not match the catalog
    When the server would start
    Then it does not start and the model manager offers to re-download

  Scenario: Low VRAM picks CPU offload
    Given the probe reports 2 GB of VRAM
    When the server starts
    Then n-gpu-layers is reduced so the model fits, or 0 for CPU only
```

## Design notes

- **Crates:** `infer-host` (process supervision; it already owns engines per [ADR 0002](../architecture/adr/0002-process-model.md)) and `infer-protocol` (`EnsureLlm`, `LlmStatus`, `UnloadLlm` messages). The router adapter is the OpenAI-compatible client from `connector-llm` (#047), configured with the port and key that infer-host reports. `models-application` gets the Built-in provider definition.
- **arch.toml:** no new crates. `infer-host` stays the only crate with inference bindings. llama-server is a separate process, not a Rust binding.
- **Security:** spawn llama-server with a fixed argument vector built in Rust, never from strings the UI or the model supply ([security: Tauri hardening](../architecture/security.md#tauri-hardening)). The API key is random per launch and held in memory only. Binaries are verified by hash before every start.
- **Privacy:** `ThisDevice`. Prompts never touch disk (no llama-server slot save or prompt cache files; verify the defaults for the pinned version).
- **Budgets:** time to the first token after a warm start, p95 < 400 ms for a 500-token prompt on an RTX 4060 (bench, #077). Cold load is shown in the UI as "Loading model…".

## QA procedure

1. On a clean Windows 11 PC with no Ollama, finish onboarding and accept the recommended local model download in Settings → Models. Observe the verified download and the state Ready after the first use.
2. Run `netstat -ano | findstr LISTENING` and find the llama-server port. Observe that it is bound to `127.0.0.1` only.
3. Run `curl http://127.0.0.1:<port>/v1/models`. Observe 401.
4. Ask the assistant a question. Observe the answer, and llama-server.exe in Task Manager.
5. Wait 10 minutes idle. Observe llama-server.exe gone and the app's memory under 300 MB (Task Manager → Details, summing the app's processes).
6. Kill llama-server.exe in Task Manager during a request. Observe a restart and the request retried or failed with a clear message.
7. On a PC with ≤ 4 GB VRAM, observe a successful start with partial or CPU offload shown in settings.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed (model card and license recorded)
