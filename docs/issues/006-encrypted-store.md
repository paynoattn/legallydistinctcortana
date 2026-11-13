# 006. Encrypted store: SQLCipher, migrations, DPAPI key wrapping

| Milestone      | Size | Depends on | Labels                                  |
| -------------- | ---- | ---------- | --------------------------------------- |
| M0 Foundations | M    | #002, #092 | area/store, area/security, type/feature |

## Story

As a privacy-conscious user, I want everything the assistant stores encrypted on disk and unlocked without typing my recovery phrase, so that a stolen disk reveals nothing and daily use stays effortless.

## Context

[Overview: data and persistence](../architecture/overview.md#data-and-persistence), [security: storage encryption](../architecture/security.md#storage-encryption) and [key hierarchy](../architecture/security.md#key-hierarchy), [ADR 0004](../architecture/adr/0004-storage-and-replication.md). One SQLCipher database per profile at `%APPDATA%\<app>\profile.db`; a random 256-bit store key per device, wrapped by DPAPI (CurrentUser).

## Scope

- `crates/store` (infrastructure library):
  - Opens `profile.db` with `rusqlite` + bundled SQLCipher, keyed with the raw 256-bit key (`PRAGMA key = "x'…'"`, so no passphrase KDF runs on every open). WAL mode, foreign keys on.
  - Migration runner: each context registers `(context, version, sql)` migrations from its infrastructure crate; the composition root passes them in. A `schema_migrations(context, version, checksum, applied_at)` table. Each migration runs in a transaction. A changed checksum for an applied migration stops startup.
  - Connection handling: one writer and a small reader pool on dedicated threads, with an async facade (`spawn_blocking`) for callers.
  - Sealed secrets: `seal(purpose, plaintext)` / `open(purpose, sealed)` with a per-secret key from HKDF(store key, purpose) and XChaCha20-Poly1305. #040 stores OAuth tokens and passwords with it.
- `StoreKeyWrapper` port in `platform-ports`, with a DPAPI adapter in `platform-windows` (`CryptProtectData`, CurrentUser scope). The wrapped key lives next to the database as `store.key`.
- A key-file adapter (`FileKeyWrapper`) for `headless` in tests, on Linux CI, and on servers, where security.md documents it as weaker (file readable only by the service user). Release desktop builds never wire it.
- Store errors surface as typed errors that the desktop maps to a recovery screen.

Out of scope: the op log (#069), Windows Hello gating (#089), Secret Service on Linux desktops, context tables (each context adds its own), backups (#073).

## Acceptance criteria

`features/core/store.feature`, against `headless` with the file key wrapper, plus a Windows-only integration test for DPAPI.

```gherkin
Feature: Encrypted store

  Scenario: First run creates an encrypted profile
    Given no profile exists
    When the core starts
    Then profile.db and store.key are created
    And profile.db does not start with the plain SQLite header "SQLite format 3"

  Scenario: Restart unlocks without user input
    Given a profile created on this account
    When the core restarts
    Then the store opens and previously written rows are readable

  Scenario: Profile copied to another Windows account
    Given profile.db and store.key copied from another user account
    When the desktop app starts
    Then the store refuses to open with "This profile belongs to another Windows account"
    And the app offers "Restore from recovery phrase or backup"
    And the original files are left untouched

  Scenario: A failed migration leaves no partial schema
    Given a registered migration whose second statement fails
    When the core starts
    Then startup stops with the context, version, and SQLite error
    And schema_migrations has no row for that version

  Scenario: An edited migration is detected
    Given migration reminders/1 was applied
    When its SQL text changes in a later build
    Then startup stops with "migration reminders/1 changed after it was applied"

  Scenario: Sealed secrets round-trip and resist tampering
    When a secret is sealed for purpose "imap-password" and opened again
    Then the original text is returned
    And opening it with purpose "oauth-refresh" or after flipping one byte fails
```

## Design notes

- `arch.toml`:
  ```toml
  [crates.store]
  allow = ["kernel", "platform-ports"]
  ban = ["tauri", "windows", "reqwest"]

  [crates.platform-ports]
  allow = ["kernel"]
  ban = ["tokio", "reqwest", "rusqlite", "serde_json", "tauri", "windows", "uuid"]

  [crates.platform-windows]
  allow = ["platform-ports", "kernel"]
  ban = ["tauri", "rusqlite", "reqwest"]
  ```
- DPAPI needs the `windows` crate, which only `platform-windows` may use, so the store gets the key through the port. Whichever of #006 and #012 lands first creates `platform-ports` and `platform-windows`.
- The DPAPI calls are `unsafe` FFI in `platform-windows/src/ffi/`, under the policy from [ADR 0012](../architecture/adr/0012-unsafe-and-ffi.md) and #092: one opted-in module, a `// SAFETY:` comment on each block, and thin wrappers whose logic lives in tested safe code.
- Build: `rusqlite` with `bundled-sqlcipher-vendored-openssl` compiles OpenSSL from source. Verify it builds with MSVC on `windows-latest` and that cargo-deny accepts the licenses (SQLCipher is BSD-style, OpenSSL 3 is Apache-2.0). Any exception goes into ADR 0003.
- Key material is held in `secrecy::SecretBox` and zeroized; it never reaches logs (`kernel::Redacted`).
- The store is shared infrastructure; contexts own their tables and never read another context's tables.
- Performance: open plus migrations under 100 ms on a warm SSD for an empty profile.

## QA procedure

1. Delete `%APPDATA%\<app>` and run the dev build. `profile.db` and `store.key` appear.
2. Open `profile.db` in DB Browser for SQLite. It reports the file is encrypted or not a database.
3. Restart the app. It opens without prompting.
4. Copy the folder to a second Windows account and start the app there. The recovery screen appears, and the copied files are unchanged.
5. Fill the disk to under 1 MB free (a small VHD works) and start the app. It shows a clear disk-full error instead of crashing.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed (security.md if the key file layout differs)
