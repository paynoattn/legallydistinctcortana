# 069. Op log and HLC last-writer-wins replication

| Milestone | Size | Depends on       | Labels                              |
| --------- | ---- | ---------------- | ----------------------------------- |
| M7 Sync   | L    | #006, #002, #068 | area/sync, area/store, type/feature |

## Story

As a person who uses two PCs, I want every change I make to my reminders, routines, memory, and shared settings to be recorded as a signed, encrypted operation that my other devices can merge in any order, so that all my devices converge on the same data even after days offline.

## Context

[ADR 0004](../architecture/adr/0004-storage-and-replication.md) and [sync: replication protocol](../architecture/sync.md#replication-protocol). The hybrid logical clock comes from `kernel` ([#002](002-kernel-crate.md)); the encrypted store from [#006](006-encrypted-store.md). This issue builds the op log and the replication session over an abstract channel; [#070](070-transport-discovery.md) carries it over the network.

## Scope

- **Op append:** every synced mutation writes the materialized row and an op `(hlc, device_id, entity_type, entity_id, field, value | tombstone, schema_version, epoch)` in one store transaction. The payload is encrypted with the epoch's content key (XChaCha20-Poly1305) and the op is signed by the device key.
- **Merge rules:** last-writer-wins per field by HLC (wall ms, counter, device id); entity deletes are tombstones kept 90 days; sets use add-wins per element. Applying the same op twice is a no-op.
- **History:** overwritten field values stay visible in the item's history for 30 days.
- **Replication session** over a `PeerChannel` port: `Hello { device_cert, epoch, version_vector }`, send missing ops in HLC order in batches ≤ 256 KB, apply, materialize, ack with the new version vector.
- **Verification before apply:** signature valid, certificate chains to the group, device not revoked as of the op's HLC, epoch key known.
- **Schema evolution:** an op with a newer `schema_version` is stored unapplied and the user sees "Update the app on this device to sync <entity>".
- **Compaction:** after every device acknowledges a version vector, ops older than 30 days fold into per-entity snapshots; a device silent for 90 days is dropped from the vector after a warning.
- **Adoption:** the synced entities in [sync: what syncs](../architecture/sync.md#what-syncs): reminders, routines, memory, shared settings, feed subscriptions and interest profile, mail digests and triage results, calendar alert state, delivered marks, connector leases, project list, device certificates.
- **Diagnostics:** `headless ops export --since <vv>` and `headless ops import <file>` (debug builds only) to replicate between two profiles without a network, for QA and tests.

Out of scope: network transport (#070), presence and delivery (#071), key rotation (#072), rich-text CRDT documents (post-MVP).

## Acceptance criteria

```gherkin
Feature: Op log replication

  Scenario: A new reminder replicates
    Given devices A and B in the same group with empty stores
    When I create the reminder "Take out the trash" on A and A and B replicate
    Then B shows the reminder with the same id, title, and schedule

  Scenario: Concurrent edits to the same field converge
    Given the reminder exists on A and B and they are disconnected
    When I rename it to "Trash" on A, and one minute later to "Bins" on B
    And A and B replicate
    Then both show "Bins"
    And the reminder's history on both shows "Trash" as overwritten

  Scenario: Concurrent edits to different fields both survive
    Given A and B are disconnected
    When I change the title on A and the time on B
    And they replicate
    Then both show the new title and the new time

  Scenario: Delete wins over an older edit
    Given A and B are disconnected
    When I edit the reminder on B and later delete it on A
    And they replicate
    Then the reminder is deleted on both and B's edit appears in the deleted item's history

  Scenario: Replaying a batch changes nothing
    Given B has applied a batch of 500 ops from A
    When the same batch is delivered again
    Then B's materialized data and version vector are unchanged

  Scenario: A tampered op is rejected
    Given an op from A whose ciphertext was modified in transit
    When B receives it
    Then B rejects it, applies nothing from that op, and records "invalid signature from <A's name>"

  Scenario: An op from a newer schema waits
    Given A runs a newer app version and sends an op with schema_version 3 for reminders
    And B understands schema_version 2
    When B receives it
    Then B stores the op unapplied and shows "Update the app on this device to sync reminders"
```

## Design notes

- **Crates:** `kernel` (HLC, already present; add the `Op` envelope types if not there), `sync-domain` (merge rules, version vectors, batching, compaction eligibility; pure and property-tested for commutativity, associativity, idempotence, and convergence of random op interleavings), `sync-application` (`ReplicationSession` use case; ports `OpLog`, `PeerChannel`, `CertificateDirectory`), `store` (op append inside the caller's transaction, apply and materialize hooks, migrations). Contexts register a `SyncedEntity` mapping (entity type, fields, set fields) with `store` from their infrastructure crate; no context reads another's tables.
- **Keys:** ops are signed with the device key and verified against the certificates from [#068](068-devices-and-pairing.md); the content key for each epoch comes from the sealed group secrets.
- **arch.toml** (protected: needs a human's approval): `sync-domain` → `kernel`; `sync-application` → `sync-domain`, `kernel`, `identity-domain`; `store` gains no context dependencies.
- **Migrations** (in `store`): `ops` (hlc, device_id, entity_type, entity_id, field, schema_version, epoch, nonce, ciphertext, sig; primary key `(device_id, hlc)`), `version_vector`, `peer_acks`, `field_history` (30-day retention), `pending_ops` (unknown schema), `entity_snapshots` (compaction output).
- **Crypto:** `chacha20poly1305` (XChaCha20-Poly1305, random 192-bit nonce per op), AAD = the op header so headers cannot be swapped between ciphertexts; `ed25519-dalek` signature over header + ciphertext. Wire encoding `postcard` (no `serde_json` in domain or application crates).
- **Clock skew:** the HLC absorbs skew; ops with a wall time more than 24 h ahead of local time are applied but logged as a skew warning on the Devices page.
- **Budgets:** applying 10,000 ops ≤ 2 s on a mid-range laptop; op append adds ≤ 1 ms to a single-row write.
- **Privacy:** ops carry `Mail` (digests, triage results, not bodies), `Memory`, `Calendar`, `Feeds`, and `Projects` data, all encrypted to the group; nothing leaves the user's devices.

## QA procedure

One Windows 11 PC is enough (two profiles); repeat across two PCs with a USB stick if available.

1. Start `headless --profile A` and `headless --profile B` from the same recovery phrase (debug build).
2. In A's UI (or local API), create the reminder "Take out the trash". Run `headless --profile A ops export --since 0 > a.ops` and `headless --profile B ops import a.ops`. Open B; observe the reminder.
3. Rename it to "Trash" in A and, a minute later, to "Bins" in B. Exchange ops both ways. Observe "Bins" on both and "Trash" in the history.
4. Import `a.ops` into B a second time; observe no change.
5. Flip one byte inside `a.ops` (hex editor) and import; observe the "invalid signature" entry and no change.
6. Delete the reminder in A after editing it in B; exchange; observe it deleted on both, with B's edit in history.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] Property tests for merge convergence in `sync-domain`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
