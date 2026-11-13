# 070. Transport: QUIC, mDNS, Tailscale, manual peers

| Milestone | Size | Depends on | Labels                                 |
| --------- | ---- | ---------- | -------------------------------------- |
| M7 Sync   | L    | #068, #069 | area/sync, area/security, type/feature |

## Story

As a person with a desktop at home and a laptop I carry, I want my devices to find each other on the same network or over my tailnet and sync automatically with end-to-end encryption, so that a reminder I add on one appears on the other within seconds and no stranger's machine can join in.

## Context

[ADR 0006](../architecture/adr/0006-transport.md), [sync: discovery and transport](../architecture/sync.md#discovery), [security: threat model](../architecture/security.md#threat-model-stride-summary). This issue carries the replication session from [#069](069-op-log-replication.md) between devices certified by [#068](068-devices-and-pairing.md). No relays in MVP ([#084](084-relay.md) is post-MVP).

## Scope

- **QUIC endpoint** (`quinn`) on the group's UDP sync port, TLS 1.3 (`rustls`) with raw public keys (RFC 7250) on both sides. A custom verifier accepts the peer only after it presents a device certificate that chains to the group key, whose `ed25519_pub` equals the TLS key, and whose device id is not revoked.
- **Sessions:** one connection per peer, one bidirectional stream per replication session, length-prefixed `postcard` messages. A session starts on connect and on local changes (debounced 1 s). Keep-alive 15 s; reconnect with backoff from 1 s to 60 s; reconnect immediately on resume and network change (`PowerEvents`).
- **LAN discovery:** `mdns-sd` service `_<app>-sync._udp.local`; TXT carries only the port and a 4-byte group hint, `HMAC-SHA256(discovery key, UTC day)[..4]`, that rotates every UTC day.
- **Windows Firewall:** onboarding explains the firewall consent prompt before the first inbound listen. If the inbound rule is missing, the device runs outbound-only: it connects to peers that listen, and the Devices page says so.
- **Tailscale discovery:** if Tailscale is installed, run `tailscale status --json` every 60 s, and probe online peers' 100.x addresses (or MagicDNS names) on the sync port.
- **Manual peers:** add `host:port` in Settings → Devices (for example a `headless` node on a home server).
- **Devices page:** per device, "Connected via LAN / Tailscale / manual", last sync time, and sync errors. Settings: sync port, LAN discovery on/off, Tailscale discovery on/off, manual peers.
- **Pause sync** toggle in the tray menu.

Out of scope: relays and hole punching (#084), presence and delivery (#071), revocation flows (#072).

## Acceptance criteria

```gherkin
Feature: Sync transport and discovery

  Scenario: Two devices on the same LAN find each other and sync
    Given PC A and PC B are in the same group on the same LAN with Tailscale off
    When both apps are running
    Then each lists the other as "Connected via LAN" within 10 seconds
    And a reminder created on A appears on B within 5 seconds

  Scenario: Two devices on different networks sync over Tailscale
    Given PC A and PC B are on different networks and both are on the same tailnet
    When both apps are running
    Then each lists the other as "Connected via Tailscale" within 90 seconds
    And a reminder created on A appears on B within 5 seconds of the connection

  Scenario: A manual peer
    Given a headless node at "nas.local:47123" in my group
    When I add "nas.local:47123" as a manual peer
    Then the node is listed as "Connected via manual"

  Scenario: A device from another group is refused
    Given PC C on the same LAN belongs to a different group
    When C tries to connect to A
    Then the handshake fails, no ops are exchanged, and C does not appear in A's device list

  Scenario: A revoked device is refused
    Given PC B's certificate is on the group's revocation list
    When B connects to A
    Then A rejects the handshake and records "Refused connection from removed device <B's name>"

  Scenario: The network drops mid-sync
    Given A is sending 5,000 ops to B
    When the Wi-Fi drops for 30 seconds and comes back
    Then the session resumes, B ends with every op exactly once, and the version vectors match

  Scenario: The mDNS record does not identify the group
    Given A advertises on the LAN
    When the TXT record is captured on two consecutive UTC days
    Then it contains only the port and a 4-byte hint, and the hint differs between the days

  Scenario: Tailscale is not installed
    Given Tailscale is not installed on A
    Then Settings → Devices shows "Tailscale: not installed" and LAN sync still works

  Scenario: No inbound firewall rule
    Given the firewall prompt on B was dismissed, so B cannot accept inbound connections
    And A can accept inbound connections
    When both apps are running on the same LAN
    Then B connects to A, they sync, and B's Devices page shows "Outbound only: other devices cannot reach this PC"
```

## Design notes

- **Crates:** `sync-domain` (peer state machine, backoff schedule, daily hint derivation, discovery merge and dedupe of the same device seen via several paths; pure), `sync-application` (`SyncNode` use case; ports `Transport`, `Discovery`, `PeerChannel` from #069, `NetworkEvents`), `sync-infrastructure` (`quinn`, `rustls`, `mdns-sd`, Tailscale CLI adapter). `arch.toml` entries are protected: needs a human's approval.
- **rustls raw public keys:** verify the rustls version and feature that supports RFC 7250 on both client and server at implementation time; if unavailable, fall back to a self-signed certificate wrapping the device key, with the same verifier logic. Record the choice in ADR 0006.
- **Group hint:** `HMAC-SHA256(discovery_key, utc_date)[..4]` with the "discovery/v1" key ([sync: discovery](../architecture/sync.md#discovery)), so the hint cannot be computed from a leaked `group_id`.
- **Tailscale:** spawn `tailscale status --json` by absolute path with a fixed argument list (no shell); parse with `serde_json` in `sync-infrastructure` only. Probing presents our raw public key to tailnet peers running the app; note this for [#078](078-security-review.md).
- **Port:** sync.md says the UDP port is chosen at install and stored in group settings; this issue picks a free port in the dynamic range at group creation and syncs it. **Windows Firewall** ([sync: transport](../architecture/sync.md#transport)): the first inbound listen triggers the consent prompt, which on many machines needs an administrator; the per-user installer cannot pre-create the rule, the MSI can. Verify the prompt on Windows 11 for private and public network profiles. Without the rule the device runs outbound-only; at least one side of each pair must accept inbound traffic.
- **Defense in depth:** op payloads stay content-key encrypted inside TLS, so a compromised tailnet or LAN sees only ciphertext.
- **Budgets:** idle sync adds < 0.2% CPU and no wakeups beyond the 15 s keep-alive per peer; sync traffic for a quiet day < 1 MB.
- **Privacy:** sync to the user's own devices is not an off-device call in the audit log; the privacy dashboard ([#074](074-privacy-dashboard.md)) lists it separately as "Your devices".

## QA procedure

Two Windows 11 PCs (A, B) in the same group from #068; Tailscale accounts optional for steps 5–6.

1. Put A and B on the same Wi-Fi with Tailscale off. Start both. Within 10 s each Devices page shows the other as "Connected via LAN". Accept the Windows Firewall prompt if shown and note whether it asked for administrator rights. In a second run, dismiss the prompt on B and observe B syncing as "Outbound only".
2. Create a reminder on A; observe it on B within 5 s. Edit it on B; observe the change on A.
3. On A, capture with Wireshark using the `mdns` display filter and confirm the TXT record has only the port and a short hint.
4. Disable Wi-Fi on B for 30 s while creating 20 reminders on A; re-enable; observe all 20 on B, no duplicates.
5. Move B to a phone hotspot. Install Tailscale on both and sign in to the same tailnet. Within 90 s observe "Connected via Tailscale"; repeat step 2.
6. Quit Tailscale on B; observe the state change to disconnected, and reconnection after it starts again.
7. On a third PC from a different group on the same LAN, confirm neither A nor B lists it.
8. Pause sync from A's tray; create a reminder; observe it does not reach B until sync is resumed.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] ADR 0006 and sync.md updated (raw public key support, port choice, verified firewall behavior)
