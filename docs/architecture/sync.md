# Multi-device sync

Your devices form a private group. They find each other on the LAN or over Tailscale, authenticate with keys derived from your 24-word recovery phrase, and replicate an encrypted operation log. No server of ours exists. Decision records: [ADR 0004](adr/0004-storage-and-replication.md), [ADR 0005](adr/0005-identity-and-keys.md), [ADR 0006](adr/0006-transport.md). Keys and threats: [security](security.md).

## Concepts

| Term                | Meaning                                                                                                                                                      |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Recovery phrase** | 24 BIP-39 words (256 bits of entropy). Created on the first device and shown once with a printable sheet. It is the root of the group.                       |
| **Group**           | All devices that know the current recovery phrase. Identified by `group_id = hash(group signing public key)`.                                                |
| **Device**          | One installation. It has a random Ed25519 **device key** and a **device certificate** signed by the group key: device id, name, platform, and creation time. |
| **Content key**     | Symmetric key (XChaCha20-Poly1305) that encrypts ops. It has an **epoch** and is rotated when a device is removed.                                           |
| **Op**              | One field-level change: `(hlc, device_id, entity_type, entity_id, field, value                                                                               | tombstone)`, encrypted with the content key and signed by the device key. |

## Joining

1. **First device:** generate the phrase, derive keys ([security](security.md)), create the group, and issue its own device certificate.
2. **Another device, with the phrase:** type the 24 words. The device derives the same group key, issues its own certificate, and announces itself.
3. **Another device, without typing words (pairing code):** the existing device shows a 6-digit code plus a QR. The new device connects over the LAN and runs **SPAKE2** with the code. On success the existing device certifies the new device key and sends the group signing key, the current content key and epoch, and the backup key inside the PAKE-keyed channel. The phrase never crosses the network. Codes expire after 2 minutes and allow 3 attempts. **Pairing channel:** while the code is shown, the existing device advertises a short-lived `_<app>-pair._udp` mDNS service and listens on an ephemeral QUIC endpoint; SPAKE2 is bound to the TLS session (exporter secret) so a relay in the middle cannot splice two sessions. Pairing therefore works before the regular sync transport is configured.

## Discovery

- **LAN:** mDNS service `_<app>-sync._udp.local` (via `mdns-sd`). The TXT record carries only `group_hint = first 4 bytes of HMAC-SHA256(discovery key, UTC day)`, which rotates daily so the group is not trackable across networks, plus the port. Nothing identifying is broadcast.
- **Tailscale:** when Tailscale is installed, read peers from the local API (`tailscale status --json`) and probe each peer's sync port. MagicDNS names and 100.x addresses both work. Tailscale already encrypts the tunnel; our protocol runs inside it unchanged, so a compromised tailnet still sees only our ciphertext.
- **Manual:** add a peer by host:port. Useful for a `headless` node on a home server.
- No relays in MVP. A self-hostable relay for devices that are never on the same network or tailnet is post-MVP.

## Transport

QUIC (`quinn`) with TLS 1.3 (`rustls`) using **raw public keys**: each side presents its device key and certificate. The verifier accepts a peer only if its certificate chains to the group key and its device id is not on the revocation list. One bidirectional stream per sync session, length-prefixed messages (postcard or msgpack). UDP port configurable; the default is chosen at install and stored in the group settings. **Windows Firewall:** the first inbound listen triggers the firewall consent prompt, which on many machines needs an administrator; the per-user installer cannot pre-create the rule, the MSI can. Onboarding explains the prompt before it appears, and sync falls back to outbound-only connections (the other peer listens) when the rule is missing (verify in #070).

## Replication protocol

```
A → B: Hello { device_cert, epoch, version_vector: {device_id → max hlc seen} }
B → A: Hello { ... }
both : send ops the other is missing (by version vector), in HLC order, batched (≤ 256 KB)
both : apply → materialize → ack with the new version vector
```

- **Conflict resolution:** last-writer-wins per field, ordered by hybrid logical clock (wall time ms, logical counter, device id). Deletes are tombstones kept 90 days. Sets (tags, list membership) use add-wins via per-element ops. This covers reminders, settings, memory, routines, feed subscriptions, read and delivered marks, and connector leases.
- **Rich text** (notes, post-MVP) will use a CRDT document (Loro or Automerge) stored as an op payload.
- **Compaction:** after every device in the group acknowledges a version vector, ops older than 30 days are folded into snapshots per entity.
- **Schema evolution:** ops carry `schema_version`. A device that receives a newer schema it does not understand stores the op without applying it and asks the user to update.
- **Backups:** "Back up to folder" writes an encrypted snapshot (content key wrapped by the backup key derived from the phrase) to any path, such as OneDrive, a USB drive, or a NAS. A restore needs the phrase.

## Presence and single delivery

Two mechanisms, two questions. **Leases** (scheduler and connectors) decide _which device does the work_: polls a mail account, computes a digest. **Single delivery** decides _which device tells the user_. Delivery never consults leases, so a sleeping lease holder cannot cause a missed notification; another device takes the lease after 2× its cadence, and every device evaluates due reminders itself.

- Each device publishes a presence op on user input (debounced to once a minute), screen lock, and sleep: `{device_id, last_active, locked, on_battery}`.
- When a reminder, heads-up, or digest is due, every online device computes the same **delivery target**: the unlocked device with the most recent input, with ties broken by device id. Only that device notifies. It writes `delivered(item_id, device_id, hlc)`. Other devices that see it suppress or clear their copy.
- If the target does not write `delivered` within 20 s (asleep or offline), the next device in order delivers. Worst case is a duplicate, never a miss.
- "Send this to my laptop" and "read this on my desktop" are ops addressed to one device, delivered on its next sync.

## What syncs

| Syncs                                                                                                                                                                                                                                                                                                               | Device-local                                                                                                                                                                                     |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Reminders, routines, memory, settings marked as shared, feed subscriptions and interest profile, mail digests and triage results (not bodies), calendar event summaries (title, time, location, join link) from the polling device, calendar alert state, delivered marks, presence, connector leases, project list | Feed items (every device polls feeds itself; they are public data), model files, audio, dictation history (opt-in to sync), OAuth tokens (opt-in), hotkeys, per-device hardware settings, caches |
