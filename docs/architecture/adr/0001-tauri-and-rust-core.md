# 0001. Tauri 2 + Rust core + React UI

Status: proposed

## Context

The assistant runs all day in the tray, captures audio with tight latency budgets, talks to GPUs and NPUs, and must stay small. OpenWhispr uses Electron, which ships a full Chromium and does its native work through Node addons.

## Decision

A Tauri 2 shell (system WebView2 on Windows), a Rust core for everything that is not pixels, and React 19 for the UI. The repository combines the `rust-axum` and `react-vite-ts` starters and their gates.

## Consequences

- Installer around 10–15 MB before models; idle memory well under Electron's.
- Audio, scheduling, crypto, and storage are native Rust under strict clippy gates, and are testable without a UI.
- WebView2 differs slightly from Chromium (no extensions API in the webview; the Gamepad API is supported).
- Rust compile time is the main cost; mitigated by crate boundaries and a build cache in CI.

## Alternatives

- **Electron:** larger, more memory, and native code would live in Node addons outside the Rust gates.
- **Native WinUI 3:** best Windows fit, but no path to macOS and Linux and no reuse of the React starter.
- **Flutter:** good UI, weak story for this much native integration, and no shared gates.
