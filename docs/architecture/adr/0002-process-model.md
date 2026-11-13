# 0002. Process model

Status: proposed

## Context

GPU and NPU drivers crash, leak, and hold gigabytes of memory. The core (scheduler, reminders, connectors) must keep running whatever the models do. A separate always-on daemon would also isolate the core from UI crashes, but it doubles installation, update, and IPC-security work.

## Decision

- The **core runtime runs in the desktop (Tauri) process**, tray-resident and started at login.
- **Inference runs in `infer-host`**, a child process the core supervises: restart with backoff, health pings, and kill to unload models. Protocol: framed msgpack over a named pipe restricted to the current user. Message types live in `infer-protocol`.
- A **`headless`** binary hosts the same core without UI, for always-on sync peers and QA.
- The browser extension reaches the core through **`native-host`** (native messaging) and a named pipe.

## Consequences

- A hung UI thread must not stall the core: the core runs on its own tokio worker threads, never on the webview thread, and a watchdog logs any event-loop stall over 250 ms.
- One installer, one updater, and one process to trust for the core.
- Per-accelerator `infer-host` builds can be swapped without touching the app.

## Alternatives

- **Separate core daemon + UI client:** better isolation, more moving parts. Revisit if UI crashes become a real problem.
- **In-process inference:** simplest, but a CUDA fault would take reminders down with it.
