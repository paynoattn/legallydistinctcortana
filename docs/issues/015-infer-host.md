# 015. infer-host: protocol, supervision, hardware probe

| Milestone    | Size | Depends on | Labels                       |
| ------------ | ---- | ---------- | ---------------------------- |
| M1 Dictation | L    | #002       | area/inference, type/feature |

## Story

As a user with a GPU or NPU, I want speech models to run in a separate process that the app restarts when a driver crashes, so that a CUDA fault never takes my reminders down and "unload models" frees the memory.

## Context

[ADR 0002](../architecture/adr/0002-process-model.md) and [ADR 0007](../architecture/adr/0007-model-runtimes.md): all engines run in `infer-host`, a child process the core supervises (restart with backoff, health pings, kill to unload). Protocol: framed msgpack over a named pipe restricted to the current user; message types in `infer-protocol`. Per-accelerator builds (CPU+Vulkan, CUDA, later QNN and OpenVINO). [Voice: hardware selection](../architecture/voice-and-speech.md#speech-to-text-engines-in-infer-host): `infer-host --probe` reports GPUs (DXGI), NPUs, VRAM, and CPU features.

## Scope

- `infer-protocol`: versioned request and response types (`Hello { protocol_version, session_token }`, `Ping`, `LoadModel`, `UnloadModel`, `Transcribe`, `Synthesize`, `Cancel`, streamed `Partial`/`Final`/`AudioChunk`, `EngineError { code, message }`), a length-prefixed frame codec (u32 length, msgpack body, 16 MiB max frame), and request ids for multiplexing and cancellation.
- `infer-host` binary: pipe client, handshake, dispatch to an `Engine` trait, and an echo engine for tests. `--probe` prints a JSON hardware report: GPUs with vendor, name, dedicated VRAM, and driver version; CPU features (AVX2, AVX-512, NEON); and which execution backends this build supports.
- `infer-client` (infrastructure library, wired by `desktop` and `headless`): `InferenceSupervisor`: creates the pipe, spawns the child with fixed arguments, health-pings every 2 s with a 1 s deadline, restarts with backoff (0.25 s doubling to 30 s), stops retrying after 5 crashes in 2 minutes and reports "Speech engine keeps crashing", and kills the child on "Unload models" or after 10 minutes idle (configurable).
- Consumers own their ports: `SpeechToText` in `voice-application` (#016) and `SpeechSynthesizer` in `speech-application` (#025). `infer-client` implements them, the way a connector implements several contexts' ports, so no context sees pipes or processes.
- Events: `InferenceHostStateChanged { Starting | Ready | Restarting | Failed | Stopped }`.

Out of scope: real engines (#016, #017, #025), model downloads (#018), installer flavor selection (#075), llama-server supervision (#048).

## Acceptance criteria

`features/core/infer-host.feature`, run against the real `infer-host` binary with the echo engine:

```gherkin
Feature: infer-host supervision

  Scenario: Handshake with the right token
    When the supervisor starts infer-host
    Then the state becomes Ready within 2 seconds
    And a Ping round-trips

  Scenario: A client with the wrong token is refused
    Given infer-host is ready
    When a second client connects to the pipe with a wrong token
    Then the connection is closed before any request is served

  Scenario: The child crashes mid-request
    Given a Transcribe request is in flight
    When the infer-host process is killed
    Then the request fails with code "engine_restarted"
    And the supervisor restarts the child and the next request succeeds

  Scenario: Repeated crashes stop the restart loop
    Given infer-host crashes on startup every time
    Then after 5 crashes in 2 minutes the state is Failed
    And the error says "Speech engine keeps crashing" with the fix "Switch to CPU engine"

  Scenario: Unload frees the process
    Given infer-host is ready
    When the user chooses "Unload models"
    Then the infer-host process exits and the state is Stopped
    And the next Transcribe starts it again

  Scenario: Protocol version mismatch
    Given an infer-host build with a newer protocol version
    Then the handshake fails with "Speech engine version mismatch: reinstall"

  Scenario: Probe reports hardware
    When "infer-host --probe" runs
    Then it prints valid JSON listing at least the CPU features and zero or more GPUs
```

## Design notes

- `arch.toml`:
  ```toml
  [crates.infer-protocol]
  allow = ["kernel"]
  ban = ["tauri", "windows", "rusqlite", "reqwest"]

  [crates.infer-host]
  allow = ["infer-protocol", "kernel", "platform-ports", "platform-windows"]
  ban = ["tauri", "rusqlite"]

  [crates.infer-client]
  allow = ["infer-protocol", "kernel", "platform-ports", "voice-application", "voice-domain", "speech-application", "speech-domain", "models-application", "models-domain"]
  ban = ["tauri", "windows", "rusqlite"]
  ```
  (Add the context crates to `infer-client`'s allow list as they land.) Add `infer-host` to `no_dependents`. DXGI enumeration needs the `windows` crate, so the GPU probe lives in `platform-windows` behind a `GpuProbe` port and `infer-host` (a composition root) wires it.
- Pipe security: the desktop creates `\\.\pipe\<app>-infer-<random>` with `reject_remote_clients`, `first_pipe_instance`, and a DACL that grants only the current user's SID (built in `platform-windows`). The pipe name and a 256-bit session token reach the child through an inherited environment variable, never the command line (visible to other processes).
- Codec: `rmp-serde`; fuzz the frame decoder later (#078). Reject frames over the maximum before allocating.
- `infer-host` main is excluded from coverage and mutation; its dispatch logic lives in a tested library module.
- Flavor features on `infer-host`: `cpu-vulkan` (default), `cuda`. Engines are compiled in per flavor by #016, #017, and #025.
- Budget: spawn to Ready under 500 ms without models; Ping round-trip under 2 ms.

## QA procedure

The debug endpoints below are added to `local-api` by this issue (`GET /v1/inference/status`, `POST /v1/inference/unload`, `POST /v1/inference/ping`), token-protected like the rest.

1. Start `headless` with `LOCAL_API_TOKEN=dev PORT=3999`. `curl -H "Authorization: Bearer dev" localhost:3999/v1/inference/status` shows `Ready`, and Task Manager shows an `infer-host` child process.
2. End `infer-host` in Task Manager. The status shows `Restarting`, then `Ready` within a few seconds.
3. `POST /v1/inference/unload`. The child process disappears and the status is `Stopped`. A ping starts it again.
4. Run `infer-host --probe` in a terminal. The JSON lists your GPU and its VRAM.
5. From a second Windows account, open the pipe name shown in the log (for example with PowerShell's `System.IO.Pipes.NamedPipeClientStream`). Access is denied.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
