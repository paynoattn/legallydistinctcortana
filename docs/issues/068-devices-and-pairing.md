# 068. Device certificates, pairing code, devices UI

| Milestone | Size | Depends on | Labels                                 |
| --------- | ---- | ---------- | -------------------------------------- |
| M7 Sync   | M    | #007       | area/sync, area/security, type/feature |

## Story

As a person with a desktop and a laptop, I want each installation to be a certified member of my private group, and to add a new PC either by typing my 24 words or by typing a short code shown on a PC I already have, so that only my devices can ever sync with each other.

## Context

[ADR 0005](../architecture/adr/0005-identity-and-keys.md) and [sync: concepts and joining](../architecture/sync.md#joining). Key hierarchy: [security](../architecture/security.md#key-hierarchy). [#007](007-identity-and-onboarding.md) creates the phrase and derives the group keys during onboarding; this issue adds device keys, certificates, the pairing-code join, and the Devices page. Replication ([#069](069-op-log-replication.md)) and the sync transport ([#070](070-transport-discovery.md)) build on it.

## Scope

- **Group secrets on every device:** the group signing key, the current content key and epoch, and the backup key, sealed in the encrypted store ([security: key hierarchy](../architecture/security.md#key-hierarchy)). The phrase is never stored.
- **Device key:** a random Ed25519 signing key and a separate random X25519 key (for receiving rotated content keys in [#072](072-device-removal-rotation.md)), created at onboarding, sealed in the store.
- **Device certificate:** `{ version, group_id, device_id, name, platform, created_at, ed25519_pub, x25519_pub }` signed by the group signing key. Issued when a device creates a group or joins with the phrase (the phrase is in memory at that moment), or by the existing device during pairing.
- **Certificate verification** (pure): chains to the group key, `group_id` matches, not revoked. Used by the transport verifier and op verification.
- **Join with the phrase:** onboarding's "I already have a recovery phrase" path issues this device's certificate.
- **Join with a pairing code:** on an existing device, Devices → "Add a device" shows a 6-digit code and a QR, valid 2 minutes, 3 attempts. On the new device, onboarding → "Join with a code". SPAKE2 runs over a short-lived LAN channel; on success the new device sends its public keys, the existing device certifies them and sends the certificate, the group signing key, the current content key and epoch, and the backup key inside the PAKE-keyed channel. The phrase never crosses the network.
- **Devices page:** this device (renamable), other known devices with platform and last seen, the group fingerprint (a short word string from `group_id`) to compare between devices, "Add a device", and an entry point for removal (implemented in #072).

Out of scope: discovery and sync transport (#070), revocation and rotation (#072).

## Acceptance criteria

```gherkin
Feature: Devices and pairing

  Scenario: The first device certifies itself
    Given I finish onboarding and create a new recovery phrase
    When I open Settings → Devices
    Then I see "This PC" with a certificate that verifies against my group key
    And a group fingerprint of 4 words

  Scenario: Join with the 24 words
    Given PC A's group fingerprint is "amber delta river stone"
    When I enter the same 24 words during onboarding on PC B
    Then PC B shows the group fingerprint "amber delta river stone"
    And PC B holds a certificate for its own device id signed by the group key

  Scenario: Join with a pairing code
    Given PC A and PC B are on the same LAN and PC A shows a pairing code
    When I enter the code on PC B within 2 minutes
    Then PC B joins the group without the phrase being shown on PC B or sent over the network
    And PC A lists PC B under Devices

  Scenario: Three wrong codes stop pairing
    Given PC A shows a pairing code
    When a wrong code is entered on PC B three times
    Then PC A shows "Pairing stopped after 3 wrong codes" and the code no longer works
    And no key material was sent

  Scenario: An expired code is rejected
    Given PC A showed a pairing code 2 minutes and 1 second ago
    When I enter it on PC B
    Then PC B shows "This code has expired. Ask for a new one."

  Scenario: A relay in the middle cannot pair
    Given a machine on the LAN relays the pairing traffic and substitutes its own SPAKE2 messages
    When I enter the correct code on PC B
    Then pairing fails with "The code did not match"
    And neither side sends key material

  Scenario: A certificate from another group is rejected
    Given a certificate signed by a different group key
    When it is verified against my group
    Then verification fails with "not a member of this group"
```

## Design notes

- **Crates:** `identity-domain` (DeviceCertificate, PairingCode with expiry and attempt counter, verification; uses `ed25519-dalek`, `x25519-dalek`, `sha2`; pure), `identity-application` (use cases `IssueCertificate`, `StartPairing`, `JoinWithCode`; ports `DeviceKeyVault`, `CertificateRepository`, `PairingChannel`), `identity-infrastructure` (store tables, `spake2` crate from RustCrypto PAKEs). The `PairingChannel` adapter (a short-lived `quinn` endpoint with an ephemeral raw public key) lives in `sync-infrastructure`, created here with only that adapter. `arch.toml` entries for all four are protected: needs a human's approval.
- **Channel binding:** the TLS layer of the pairing channel is unauthenticated; SPAKE2 authenticates. Both sides include the TLS exporter value in the SPAKE2 identity strings and confirm the shared key with a MAC before anything else is sent, so a relay cannot splice two sessions.
- **Finding the existing device:** while a code is shown, PC A advertises `_<app>-pair._udp.local` via `mdns-sd` with no TXT content beyond the port; PC B connects to each responder and tries the PAKE (a wrong responder simply fails). The QR encodes `<app>://pair?code=…&hosts=…` for a future companion; on PCs, typing the code is the supported path. This design is recorded in [sync: joining](../architecture/sync.md#joining).
- **Secrets:** private keys sealed in the store with the per-secret key ([security: storage encryption](../architecture/security.md#storage-encryption)); phrase and seed in `secrecy::SecretBox`, zeroized after use. The pairing code is generated with the OS CSPRNG, never logged.
- **Data:** migration in `identity-infrastructure`: `device_keys` and `group_secrets` (device-local, sealed), `device_certificates` (synced, so every device knows the group), `pairing_sessions` (memory only, never persisted).
- **UI:** `src/devices/{domain,application,infrastructure,ui}` in the `main` window; onboarding's join screens in `src/onboarding/`. Only the `main` window may call device commands.
- **Data classes:** none leave the device; only group keys travel, inside the PAKE channel, to the user's own device.

## QA procedure

Two Windows 11 PCs (A and B) on the same Wi-Fi or wired LAN, fresh installs.

1. On A, complete onboarding with a new phrase. Open Settings → Devices. Observe "This PC" and a 4-word group fingerprint; write it down.
2. On B, start onboarding, choose "I already have a recovery phrase", type the 24 words. Open Devices; observe the same fingerprint.
3. Reset B (uninstall with "delete profile"). On A, click "Add a device"; a 6-digit code and a countdown appear.
4. On B, choose "Join with a code" and type the code. Observe B joining without showing words, and A listing B.
5. On A, start a new code. On B, type three wrong codes; observe A's "Pairing stopped" message and that the right code now fails.
6. Start a new code, wait for the countdown to end, type it on B; observe "expired".
7. Rename B on B's Devices page; observe the new name.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] `docs/architecture/sync.md` and `security.md` updated (X25519 key in the certificate)
