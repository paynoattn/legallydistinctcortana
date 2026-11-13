# 084. Self-hostable relay for devices on different networks

| Milestone | Size    | Depends on | Labels                            |
| --------- | ------- | ---------- | --------------------------------- |
| Post-MVP  | unsized | #070, #072 | area/sync, type/feature, post-mvp |

## Story

As a person whose laptop and desktop are rarely on the same network and who does not use Tailscale, I want to run a small relay on a server I control so my devices can sync through it, so that sync works anywhere without trusting a third party with my data.

## Context

[ADR 0006](../architecture/adr/0006-transport.md): no relays in MVP; devices on different networks without Tailscale cannot sync until a relay exists; reconsider iroh when relays are added. [sync: discovery](../architecture/sync.md#discovery). The vision rules out any server of ours in the data path, so the relay is self-hosted only.

## Scope

- A `relay` binary (and container image) that forwards end-to-end encrypted QUIC streams between devices of a group; it never holds keys and sees only ciphertext, IP addresses, and timing.
- Devices register with a group-derived relay token (from a new HKDF label) so the relay serves only that group; relay address configured in Settings → Devices and synced.
- Decide in a new ADR between our own relay and iroh's relay and hole-punching stack (license, maintenance, and dependency surface).

Out of scope: a project-hosted public relay; mobile push.

## Acceptance criteria

```gherkin
Feature: Self-hostable relay for devices on different networks

Scenario: Sync through a relay
  Given PC A at home and PC B on a phone hotspot, no Tailscale, and a relay on a VPS
  When both are configured with the relay address
  Then a reminder created on A appears on B within 10 seconds

Scenario: The relay cannot read data
  Given a relay operator captures all traffic
  Then the capture contains no plaintext op content and no group key material
```

## Design notes

- New crates or modules in `sync-infrastructure` (client) and a `relay` composition root; the relay depends on no context crates.
- Revoked devices are refused by the peers as today; the relay also drops tokens after a phrase rotation (#072).
- Document deployment (Docker, systemd) in the user guide.

## QA procedure

1. Deploy the relay on a small VPS. Configure both PCs. Put them on different networks with Tailscale off. Create and edit reminders on each and observe sync.
2. Run Wireshark on the VPS and confirm only encrypted QUIC traffic.
