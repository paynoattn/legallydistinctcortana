# 0006. Transport and discovery

Status: proposed

## Decision

QUIC (`quinn`) + TLS 1.3 (`rustls`) with raw public keys, verified against the group's device certificates. Discovery uses three sources: mDNS on the LAN (a rotating, non-identifying hint), Tailscale peers from the local Tailscale API, and manual host:port. No relay servers in MVP.

## Consequences

- Works on home and office LANs, and anywhere both devices are on the same tailnet, which is what the product asks for.
- Devices on different networks without Tailscale cannot sync until a relay exists (post-MVP, self-hostable).

## Alternatives

- **iroh:** a strong P2P library with hole punching and relays. Reconsider when relays are added. For MVP, our own QUIC stack keeps the dependency surface and the trust story small.
- **Syncthing-style TCP + TLS:** fine, but QUIC gives multiplexing and better resumption on flaky Wi-Fi.
- **Noise over TCP:** viable; rustls with raw public keys reuses audited TLS code.
