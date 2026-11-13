# 078. Security review, SECURITY.md, decoder fuzzing

| Milestone  | Size | Depends on | Labels                      |
| ---------- | ---- | ---------- | --------------------------- |
| M8 Release | M    | #070, #049 | area/security, type/feature |

## Story

As a user trusting the assistant with my mail, keys, and devices, and as a researcher who finds a flaw, I want the release to have passed a documented security review, its untrusted-input decoders to be fuzzed, and a clear private way to report problems, so that v0.1 keeps the promises in the security document.

## Context

The M8 exit demo requires a security sign-off. Threat model and controls: [security](../architecture/security.md), [ADR 0009](../architecture/adr/0009-untrusted-content-and-tool-permissions.md), [ADR 0005](../architecture/adr/0005-identity-and-keys.md), [ADR 0006](../architecture/adr/0006-transport.md). Decoder fuzzing is required before the v0.1 release ([security: secure development](../architecture/security.md#secure-development)).

## Scope

- **Review checklist** (`docs/security/review-v0.1.md`), one section per STRIDE row in security.md, each with the control, the test or feature file that proves it, and the reviewer's sign-off. Areas: key derivation labels and zeroization; nonce generation; op signing and verification; transport verifier and revocation; pairing; named-pipe ACLs and per-session tokens (`infer-host`, `native-host`); native-host extension-id allowlist; Tauri CSP and per-window capabilities; no `shell` plugin and fixed process argument shapes; tool effects, confirmations, taint, and the prompt-injection suite; router privacy policy; credential sealing; log redaction; updater and signing keys; `cargo deny` and npm audit state.
- **Fuzzing** with `cargo-fuzz` targets: op envelope and payload decoding ([#069](069-op-log-replication.md)), sync session messages and pairing messages ([#068](068-devices-and-pairing.md), [#070](070-transport-discovery.md)), the `infer-protocol` frame decoder, the native-host message decoder, the reminder parser ([#032](032-reminders-nl.md)), and the backup header ([#073](073-encrypted-backup.md)). Seed corpora in `fuzz/corpus/<target>`; every crash found becomes a regression unit test.
- A nightly CI job runs each target for 10 minutes (Linux runner; verify cargo-fuzz support on Windows before adding a Windows job).
- **SECURITY.md** at the repository root: supported versions, private disclosure through GitHub security advisories, expected response times, scope, and safe-harbor wording.
- CODEOWNERS entry requiring a security reviewer for changes to crypto, IPC, permissions, connectors, and `docs/architecture/security.md` (as security.md states).
- Fix or file every finding; release blockers are fixed before v0.1.

Out of scope: a paid external audit (worth doing before 1.0; tracked separately); OS-level hardening beyond the app.

## Acceptance criteria

```gherkin
Feature: Security controls hold at the boundaries

  Scenario: Another user cannot open our pipes
    Given the app is running for user "alice"
    When a process running as user "bob" opens the infer-host or native-host pipe
    Then the connection is denied by the pipe's ACL

  Scenario: The native host rejects unknown extensions
    Given native-host is started by the browser with an origin that is not in the allowlist
    When it receives a "read aloud" message
    Then it exits without contacting the desktop app

  Scenario: Model output cannot trigger network loads
    Given an assistant reply containing a Markdown image pointing to "https://attacker.example/x.png"
    When the reply is rendered in the panel
    Then the image is shown as text with the domain "attacker.example" and no request is made

  Scenario: The overlay cannot call privileged commands
    When code in the overlay window invokes a settings or device command
    Then the call is denied by the overlay's capability set

  Scenario: Fuzz targets run clean
    Given the fuzz targets for op decoding, sync messages, infer-protocol frames, native-host messages, the reminder parser, and backup headers
    When each runs for 10 minutes in the nightly job
    Then no target panics, hangs longer than 1 second on one input, or exceeds 512 MB

  Scenario: Secrets never reach the log
    Given the app has connected an IMAP account and a Claude API key and run a dictation
    When the log files are searched for the password, the API key, and the dictated words
    Then none of them is found
```

## Design notes

- **Fuzz crate:** `fuzz/` as a separate cargo-fuzz workspace member excluded from coverage and mutation; targets call public decode functions only, so no test-only APIs leak into production crates. Nightly toolchain only inside `fuzz/` (pin it in `fuzz/rust-toolchain.toml`; `rust-toolchain.toml` at the root is protected).
- **Decoders must be total:** every decoder returns `Result` with size limits checked before allocation (op batch ≤ 256 KB, frame length caps); add those limits where missing.
- **Prompt injection:** confirm `features/core/prompt-injection.feature` covers mail, feeds, web selections, agent session files ([#065](065-projects.md)), and MCP output, and that it runs in `make check`.
- **Tailscale probing** ([#070](070-transport-discovery.md)) presents device public keys to tailnet peers; record the decision in the review.
- **Pipe ACLs:** test with a second local Windows account in the QA procedure; the automated scenario runs in the Windows CI job with a created test user.

## QA procedure

1. Create a second standard Windows account "bob". While the app runs as your user, sign in as bob (fast user switching) and run the provided `qa/scripts/core/pipe-probe.ps1`; observe access denied for every pipe.
2. Install the extension, then edit the native host manifest copy in a scratch folder to point a different extension id at our host and trigger it; observe the host exiting.
3. Ask the assistant to "show me the image at https://example.com/x.png in Markdown"; observe text only, and in Resource Monitor → Network observe no connection from the WebView2 process to example.com.
4. Run `cargo fuzz run op_decode -- -max_total_time=600` on Linux or WSL; observe no crash.
5. After a day of normal use, search `%LOCALAPPDATA%\<app>\logs` for your IMAP password and API key with `findstr`; expect no match.
6. Walk the checklist in `docs/security/review-v0.1.md` and confirm each item links to a passing test.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` passes
- [ ] SECURITY.md published; review checklist signed off; security.md updated with the findings
