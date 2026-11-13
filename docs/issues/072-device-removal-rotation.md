# 072. Device removal, key rotation, phrase rotation

| Milestone | Size | Depends on | Labels                                 |
| --------- | ---- | ---------- | -------------------------------------- |
| M7 Sync   | M    | #070       | area/sync, area/security, type/feature |

## Story

As a person who sold an old laptop or lost one, I want to remove it from my group so it cannot read anything I write from now on, and, if it was lost, to change my recovery phrase so whoever has it cannot rejoin, so that losing a device does not mean losing control of my data.

## Context

[ADR 0005](../architecture/adr/0005-identity-and-keys.md) and [security: rotation and device removal](../architecture/security.md#rotation-and-device-removal). Device certificates and the X25519 key used to receive new content keys come from [#068](068-devices-and-pairing.md); the verifier that rejects revoked devices is in [#070](070-transport-discovery.md).

## Scope

- **Remove a device** (Devices → device → "Remove"): sign a revocation for its device id with the group signing key that every device keeps sealed in its store (no phrase needed), generate a random content key for epoch + 1, encrypt it to each remaining device's X25519 key, sign the epoch record, and replicate both. New ops use the new epoch; old epoch keys are kept to read history.
- **Report lost** (Devices → device → "Lost or stolen", the default for lost devices): remove as above, then rotate the recovery phrase: generate a new phrase, show it once with the printable sheet, derive the new group keys, and send each remaining device a migration record (a new certificate for its existing device key plus the new group signing key, content key, and backup key, encrypted to its X25519 key), signed by both the old and the new group key.
- **Offline remaining devices** receive the migration on their next connection during a 90-day window, during which online devices accept old-group certificates that are not revoked, only to deliver the migration. After the window, such a device must join with the new phrase.
- **On the removed device**, if it is online when the revocation arrives: sync stops with "This PC was removed from your group" and an offer to delete local data.
- Warnings in the UI: a removed device keeps whatever it already synced; because it held the group signing key, plain removal only protects against a device that is still in your hands; backups made with the old phrase can still be opened with the old phrase.

Out of scope: remote wipe beyond the cooperative offer above; leaving a group from the device itself (handled by "Delete everything" in privacy settings).

## Acceptance criteria

```gherkin
Feature: Removing devices and rotating keys

  Scenario: Remove a device
    Given devices A, B, and C are in sync at content-key epoch 4
    When I remove C on A and confirm
    Then A and B move to epoch 5
    And a reminder created on A afterwards cannot be decrypted with any key C holds
    And C's next connection attempt is refused

  Scenario: The removed device is told when online
    Given C is online when it is removed
    Then C shows "This PC was removed from your group" and offers "Delete data on this PC"

  Scenario: Plain removal points to the lost flow
    When I choose "Remove" for C
    Then the confirmation says "C still holds your group key. If it is lost or not in your hands, choose 'Lost or stolen' to change your recovery phrase."

  Scenario: Report a device lost and change the phrase
    Given A, B, and C are in sync
    When I report C lost on A and confirm
    Then A shows a new 24-word phrase once with a print option
    And A and B show a new group fingerprint
    And entering the old phrase on a fresh PC creates a group that A and B refuse to sync with

  Scenario: An offline device catches up after a phrase change
    Given B was offline when the phrase was changed on A
    When B comes online 3 days later
    Then B receives its new certificate and the new content key and syncs normally without asking for the phrase

  Scenario: An offline device after the window
    Given B was offline for more than 90 days after the phrase change
    When B comes online
    Then B shows "Your group's recovery phrase changed. Enter the new phrase to keep syncing."

  Scenario: Old backups are called out
    Given a folder backup was made before the phrase change
    When the phrase change finishes
    Then I see "Backups made before today open with your old phrase. Delete them if the old phrase may be exposed."
```

## Design notes

- **Keys on every device:** each device keeps the group signing key, the current content key, and the backup key sealed in its store ([security: key hierarchy](../architecture/security.md#key-hierarchy)), so any device can sign revocations and epochs without the phrase. For the same reason a removed device could certify itself back in, which is why "Lost or stolen" rotates the phrase by default ([security: rotation](../architecture/security.md#rotation-and-device-removal)).
- **Crates:** `identity-domain` (Revocation, EpochRecord, GroupMigration; signature rules), `identity-application` (`RemoveDevice`, `ReportLost`, `ApplyMigration`), `sync-application` (epoch switch, migration delivery, old-group acceptance window), `store` (epoch key table). Crypto: `x25519-dalek` + HKDF + XChaCha20-Poly1305 (sealed box style) to encrypt keys to each device.
- **Ordering:** the revocation and the new epoch are written before any new op; ops from the revoked device with an HLC after the revocation are rejected (#069).
- **Data:** migration adds `revocations`, `epoch_keys` (sealed, device-local), updates to the sealed group secrets from #068, and `group_history` (old group ids during the window).
- **Zeroization:** the new phrase and its seed live in `secrecy` wrappers, zeroized after the flow; the new phrase is shown once and never logged or stored.
- **UI:** `src/devices/` adds the remove and lost flows with plain warnings and the new-phrase sheet (reusing onboarding's printable sheet).

## QA procedure

Three Windows 11 PCs (A, B, C) in one group, syncing over the LAN; C can be a VM.

1. On A, open Devices → C → Remove, read the warning, and confirm. Observe C showing "removed" and the offer to delete data.
2. Create a reminder on A; observe it on B and not on C. Restart C; observe its connection being refused in A's Devices log.
3. Rejoin C with the phrase (allowed: not reported lost). On A, choose C → "Lost or stolen". Write down the new phrase from the sheet. Observe A and B with a new fingerprint.
4. On C (or a fresh VM), uninstall with profile deletion and onboard with the old phrase. Observe that A and B never list it.
5. Turn B off before step 3 in a second run; turn it on after the phrase change; observe B catching up without a prompt.
6. Make a folder backup before a phrase change and confirm the warning after it.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] security.md and sync.md updated (migration window, X25519 key in the certificate)
