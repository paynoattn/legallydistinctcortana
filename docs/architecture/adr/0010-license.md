# 0010. License

Status: proposed (needs the owner's decision)

## Decision

Dual-license the code **MIT OR Apache-2.0**, the Rust ecosystem default. It is permissive, Apache-2.0 adds a patent grant, and it is compatible with the dependency licenses allowed in `deny.toml`.

Model weights we publish carry their own licenses. Each must allow redistribution, and it can be no more permissive than its training data allows.

## Alternatives

- **GPL-3.0 / AGPL-3.0:** keeps forks open, including anyone repackaging the app as a paid product. Choose this if preventing closed forks matters more than adoption.
- **MIT only:** simplest, but no patent grant.
