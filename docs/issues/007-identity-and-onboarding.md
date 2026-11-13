# 007. Identity: recovery phrase, key derivation, onboarding

| Milestone      | Size | Depends on       | Labels                                     |
| -------------- | ---- | ---------------- | ------------------------------------------ |
| M0 Foundations | M    | #006, #004, #008 | area/identity, area/security, type/feature |

## Story

As a new user, I want to create a 24-word recovery phrase on my first computer, or type it on another to join, so that my devices share one private identity with no account.

## Context

[ADR 0005](../architecture/adr/0005-identity-and-keys.md), [security: key hierarchy](../architecture/security.md#key-hierarchy), [sync: joining](../architecture/sync.md#joining). MVP outcome 1: install, create a phrase, and finish onboarding in under 5 minutes, offline.

## Scope

- `identity-domain`: BIP-39 (English wordlist, 256-bit entropy, checksum), seed derivation, HKDF-SHA256 derivations with versioned labels (`group-sign/v1`, `group-id/v1`, `content-root/v1`, `backup/v1`, `discovery/v1`), `DeviceCertificate { device_id, name, platform, created_at, device_public_key }` signed by the group key, and certificate verification.
- `identity-application`: `CreateIdentity` and `JoinWithPhrase` use cases, with ports `EntropySource`, `IdentityRepository`, and `Clock`. Both derive keys, issue this device's certificate, persist, and zeroize the phrase and seed.
- `identity-infrastructure`: OS entropy adapter and a store-backed repository (tables `identity_device`, `identity_group_keys`) holding, each sealed with the store's secret sealing (#006): the device key, the **group signing key**, the epoch-0 content key, and the **backup key**; plus the device certificate, the group public key, and the group id in the clear. The phrase and seed are never stored.
- `src/onboarding` UI in the main window: Welcome → Create or Join → Show phrase (with a printable sheet) → Confirm three random words → Done. Join: 24 inputs with wordlist autocomplete, paste of a whole phrase, and checksum validation.
- Tauri commands: `identity_status`, `identity_begin_create` (returns the words once), `identity_confirm_create`, `identity_join`.

Out of scope: pairing code and device list (#068), announcing to peers (#070), rotation (#072), backups (#073), other BIP-39 languages.

## Acceptance criteria

`features/ui/onboarding.feature` (fake IPC) and `features/core/identity.feature` (in process):

```gherkin
Feature: Recovery phrase and onboarding

  Scenario: Create a phrase on the first device
    Given no identity exists
    When I choose "Set up this PC" and confirm words 3, 11, and 20 correctly
    Then onboarding finishes and identity_status reports a group id and one device certificate
    And the store holds the sealed group signing key, content key (epoch 0), and backup key

  Scenario: A wrong confirmation word does not finish setup
    Given I am on the confirm step
    When I enter a wrong word for word 11
    Then I see "Word 11 doesn't match" and setup is not finished

  Scenario: Quitting before confirmation leaves no identity
    Given the phrase was shown but not confirmed
    When the app restarts
    Then onboarding starts again with a new phrase

  Scenario: Joining derives the same group
    Given device A created an identity from a phrase
    When device B joins with the same 24 words
    Then both report the same group id
    And B's certificate verifies against the group public key

  Scenario: A phrase with a bad checksum is rejected
    When I join with 24 valid words whose checksum fails
    Then I see "These words don't form a valid recovery phrase. Check the order."
    And no identity is created

  Scenario: A misspelled word is caught before submit
    When I type "abandn" as word 4
    Then the field is marked invalid and suggests "abandon"

  Scenario: The phrase never reaches logs or the store
    Given an identity was created with debug logging on
    Then no log line and no store row contains any of the 24 words
```

## Design notes

- `arch.toml`: `identity-domain` allow `["kernel"]`; `identity-application` allow `["identity-domain", "kernel"]`; `identity-infrastructure` allow `["identity-application", "identity-domain", "kernel", "store"]`; the usual bans (`tokio`, `serde_json`, `rusqlite`, `tauri`, `windows`, `uuid`) on domain and application.
- Crates (verify licenses with cargo-deny): `bip39` (the rust-bitcoin crate is CC0-1.0, which is not on the allow-list and would need an ADR 0003 entry), `hkdf`, `sha2`, `pbkdf2`, `ed25519-dalek`, `zeroize`, `secrecy`.
- Known-answer tests: BIP-39 official test vectors for mnemonic → seed, and fixed vectors for every HKDF label so a refactor cannot silently change derived keys.
- Data: the phrase and seed live only in memory during the two use cases. The words cross IPC once (`identity_begin_create`) and the UI drops them after confirmation. No copy button; the printable sheet uses the webview's print dialog.
- Consider `SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)` on the main window while the phrase is shown (through `platform-windows`; verify it works with WebView2).
- Security review required (CODEOWNERS on crypto). Every device keeps the group signing key sealed so it can certify devices it pairs (#068) and sign revocations (#072); pairing sends keys, never the phrase ([sync: joining](../architecture/sync.md#joining)).

## QA procedure

1. Start the app with an empty profile. Onboarding opens.
2. Choose "Set up this PC". 24 numbered words appear. Print the sheet to PDF; it shows all words and the device name.
3. Enter one wrong confirmation word. An inline error appears. Enter the right words. Onboarding finishes in under 5 minutes with the network unplugged.
4. Search `%APPDATA%\<app>\logs` for any of the words. None are found.
5. On a second PC (or a second Windows account), choose "Join with recovery phrase" and paste the words. Settings → About shows the same group id as the first PC.
6. Swap two words and try again. The checksum error appears.
7. Run through onboarding with Narrator on. Every step and word is announced.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
