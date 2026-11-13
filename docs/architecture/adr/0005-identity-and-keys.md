# 0005. Identity: 24-word phrase and device keys

Status: proposed

## Context

Users want to connect their computers end to end with no account, recover after losing a device, and understand what they are protecting.

## Decision

- A **24-word BIP-39 phrase** (256-bit entropy) is the root. HKDF with versioned labels derives the group signing key, group id, epoch-0 content key, backup key, and discovery key.
- Each device has a **random device key**, certified by the group key. The phrase is not stored after setup.
- Day-to-day unlock uses a random store key wrapped with DPAPI, optionally gated by Windows Hello.
- Removing a device rotates the content key; reporting it lost also rotates the phrase.

Details: [security](../security.md), [sync](../sync.md).

## Consequences

- Familiar to anyone who has used a crypto wallet or a password manager's emergency kit, and printable.
- Losing the phrase and every device means losing the data. Setup says so plainly and offers encrypted backups.
- BIP-39 wordlists give us 10 languages for free.
