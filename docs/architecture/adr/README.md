# Architecture decision records

One decision per file. Status is `proposed` until a human accepts it, then `accepted`, and later possibly `superseded by NNNN`. Change a decision by writing a new ADR, not by editing an accepted one.

| ADR                                                    | Decision                                                                                         | Status   |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------------ | -------- |
| [0001](0001-tauri-and-rust-core.md)                    | Tauri 2 + Rust core + React UI, not Electron                                                     | proposed |
| [0002](0002-process-model.md)                          | Core in the desktop process; inference in a supervised child process                             | proposed |
| [0003](0003-supply-chain-exceptions.md)                | cargo-deny exceptions required by Tauri                                                          | accepted |
| [0004](0004-storage-and-replication.md)                | SQLCipher store + signed, encrypted op log with HLC last-writer-wins                             | proposed |
| [0005](0005-identity-and-keys.md)                      | 24-word BIP-39 phrase as the group root; per-device keys and certificates                        | proposed |
| [0006](0006-transport.md)                              | QUIC + rustls raw public keys; discovery by mDNS and Tailscale; no relays in MVP                 | proposed |
| [0007](0007-model-runtimes.md)                         | whisper.cpp + sherpa-onnx STT, Kokoro + fine-tuned expressive TTS, OpenAI-compatible LLM servers | proposed |
| [0008](0008-browser-integration.md)                    | Browser extension + native messaging for right-click read-aloud                                  | proposed |
| [0009](0009-untrusted-content-and-tool-permissions.md) | Tool effects, confirmations, and taint tracking against prompt injection                         | proposed |
| [0010](0010-license.md)                                | MIT OR Apache-2.0                                                                                | proposed |
| [0011](0011-brand-neutral-code.md)                     | Brand-neutral code; product name in one place                                                    | accepted |
| [0012](0012-unsafe-and-ffi.md)                         | `unsafe` only in FFI modules of allow-listed crates, with `// SAFETY:` comments                  | proposed |
| [0013](0013-hardware-adapters-and-mutation.md)         | Hardware-bound adapter modules named `os` are excluded from mutation testing                     | accepted |
