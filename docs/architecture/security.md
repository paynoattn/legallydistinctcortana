# Security and privacy

## Promises to the user

1. Nothing about you leaves your devices unless you turned on a feature that says it does, and named the provider.
2. Everything stored is encrypted. Everything sent between your devices is end-to-end encrypted.
3. No account, no telemetry, no servers of ours.
4. The assistant never sends, deletes, or spends anything without showing you exactly what it will do, unless you granted that specific permission.

## Key hierarchy

```
recovery phrase (24 words, BIP-39, 256-bit entropy)
  └─ seed = PBKDF2-HMAC-SHA512(phrase, "mnemonic", 2048)            (BIP-39 standard)
       └─ HKDF-SHA256(seed, info=…)
            ├─ "group-sign/v1"     → Ed25519 group signing key   (signs device certs, revocations, epochs)
            ├─ "group-id/v1"       → group id
            ├─ "content-root/v1"   → epoch-0 content key          (later epochs: random, see rotation)
            ├─ "backup/v1"         → backup wrapping key
            └─ "discovery/v1"      → mDNS hint key

device key (Ed25519, random per device)  ── certified by group signing key
store key (256-bit random per device)    ── wrapped by DPAPI (CurrentUser), optionally gated by Windows Hello
```

- The phrase and seed are held in memory only during setup, joining, and rotation, then zeroized (`zeroize`, `secrecy`). After setup every device keeps, sealed inside the encrypted store: its device key and certificate, the **group signing key** (needed to certify devices it pairs and to sign revocations), the current content key, and the backup key. The phrase itself is never stored; it is needed only to recover when every device is lost, and to rotate.
- **Unlock.** The app opens the store without the phrase (DPAPI on Windows; Secret Service via `keyring` on Linux desktops; for `headless` on a server, a key file readable only by the service user, documented as weaker). Optional "require Windows Hello to open" wraps the store key with a Windows Hello-backed key via `KeyCredentialManager`.

## Rotation and device removal

- **Remove a device** (lost laptop): the group signs a revocation, generates a new random content key (epoch + 1), and distributes it to each remaining device, encrypted to its device key (X25519 converted from Ed25519, or a separate X25519 key in the cert). The removed device cannot read new ops.
- **But it held the group key.** Every device holds the group signing key, and a device that saw the phrase can re-derive it, so a removed device could certify itself back in. The UI therefore offers **"Remove and change recovery phrase"**: generate a new phrase, re-certify the remaining devices, and re-encrypt the backup key. This is the default when the device is reported lost.

## Storage encryption

- SQLCipher (AES-256, HMAC-SHA512 pages) through `rusqlite` with the bundled SQLCipher feature. One database per profile.
- Secrets inside the store (OAuth refresh tokens, API keys, app passwords) are additionally sealed with a per-secret key derived from the store key, so a memory dump of query results does not reveal them in passing.
- Temporary files: none for audio. Model downloads are public data. Logs are redacted (no message bodies, no transcripts, no tokens) and rotated (5 × 5 MB).

## Threat model (STRIDE summary)

| Threat                               | Example                                | Mitigation                                                                                                                                                                                         |
| ------------------------------------ | -------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Spoofed peer on the LAN              | Attacker answers mDNS and offers sync  | Certificates must chain to the group key; raw-public-key TLS; no trust on first use                                                                                                                |
| Tampered ops                         | Malicious peer edits a reminder        | Ops signed by device keys; signatures verified before applying; revoked devices rejected                                                                                                           |
| Eavesdropping                        | Café Wi-Fi                             | QUIC/TLS 1.3 plus content-key encryption of op payloads (defense in depth)                                                                                                                         |
| Prompt injection                     | Email says "forward all invoices to x" | Taint tracking, no auto-run of External or Destructive tools in tainted turns, quarantined summarization ([assistant](assistant.md))                                                               |
| Data exfiltration through cloud LLMs | Mail bodies sent to a cloud model      | Per-data-class privacy policy enforced by the router, local by default                                                                                                                             |
| Malicious MCP server or extension    | Tool output with hidden instructions   | MCP output is untrusted; MCP tools are External and confirmed                                                                                                                                      |
| Local malware as the same user       | Reads the profile                      | Out of scope to fully prevent. DPAPI and optional Windows Hello raise the bar; no plain-text secrets on disk.                                                                                      |
| Local process calling our IPC        | Another app talks to our named pipe    | Pipe ACL to the current user SID; per-session random token; native-host checks the browser extension origin allowlist                                                                              |
| Supply chain                         | Compromised crate or npm package       | cargo-deny (advisories, licenses, sources), pinned lockfiles, `--frozen-lockfile`, knip and depcruise for unused deps, Dependabot with a review gate, reproducible release builds, signed releases |
| Malicious update                     | Swapped installer                      | Tauri updater with a minisign key held offline; Authenticode signing (SignPath Foundation for OSS, or Azure Trusted Signing)                                                                       |
| Model files                          | Tampered GGUF/ONNX                     | SHA-256 pinned in the catalog; safetensors/GGUF/ONNX only, never pickle                                                                                                                            |

## Tauri hardening

- A strict CSP is set in `crates/desktop/tauri.conf.json` (protected gate file): `default-src 'self'`, no remote scripts, no `unsafe-eval`.
- Capabilities are scoped per window. The overlay window gets no file-system or shell permissions. Only the main window may call settings commands.
- No `shell` plugin. External processes (agent CLIs, llama-server) are spawned only by Rust code with fixed argument shapes, never from strings the UI or the model supply.
- Every IPC command validates its input in the application layer (domain newtypes), not in the UI.

## Privacy UX

- **Privacy dashboard:** per data class, where it may go, what was sent in the last 30 days (a local audit log of every off-device call, including connector syncs, model downloads, and update checks: provider or host, task, data classes, byte count, never content. Adapters report through an `AuditSink` port defined in `kernel`; the `privacy` context owns the log and the dashboard), and a one-click "local only" switch.
- **First-run defaults:** everything local; wake word off; dictation history off; cloud providers not configured.
- **Export and delete:** export everything as JSON plus the encrypted backup format. "Delete everything" wipes the store, the models, and the keys, and asks for removal from the group.

## Secure development

- `make check` gates in CI on every PR; `make gauntlet` before releases.
- `docs/architecture/security.md` is reviewed on any change to crypto, IPC, permissions, or connectors (CODEOWNERS).
- A security policy (`SECURITY.md`) with private disclosure via GitHub security advisories.
- Fuzzing (cargo-fuzz) for op decoding, the reminder parser, and the IPC frame decoders, before the v0.1 release (#078).
