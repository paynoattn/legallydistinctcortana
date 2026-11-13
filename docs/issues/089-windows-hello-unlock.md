# 089. Windows Hello unlock for the store

| Milestone  | Size | Depends on | Labels                                                 |
| ---------- | ---- | ---------- | ------------------------------------------------------ |
| M8 Release | M    | #006, #092 | area/security, area/store, area/platform, type/feature |

## Story

As a privacy-conscious user who shares a Windows account or leaves my laptop unlocked, I want the assistant's data to open only after Windows Hello confirms it's me, so that someone at my keyboard cannot read my mail summaries, memory, or reminders.

## Context

[Security: key hierarchy](../architecture/security.md#key-hierarchy): the store key is random per device and wrapped by DPAPI (CurrentUser), with an optional "require Windows Hello to open" that wraps it again with a Windows Hello-backed key through `KeyCredentialManager`. [ADR 0005](../architecture/adr/0005-identity-and-keys.md). #006 built the DPAPI path and the `StoreKeyWrapper` port.

## Scope

- `platform-windows`: a `HelloKeyWrapper` implementing `StoreKeyWrapper` with `Windows.Security.Credentials.KeyCredentialManager`: create or open a per-app Hello key, have it sign a fixed per-profile challenge, derive a wrapping key from the signature with HKDF, and wrap the DPAPI-unwrapped store key with XChaCha20-Poly1305. The FFI lives in `src/ffi/` under ADR 0012.
- `store`: a wrapper chain (DPAPI, then Hello) recorded in a small header next to `store.key`, so startup knows which unwrap steps to run.
- Recovery escrow: when Hello is turned on, the store key is also wrapped with the backup key (held sealed in the store, see security.md) and kept beside `store.key`, so typing the recovery phrase can unlock the store if Hello is unavailable or reset.
- Startup when Hello is on: the core does not open the store or start the scheduler, connectors, or sync until unlock. The tray icon shows a locked state; clicking it, or any hotkey, prompts Hello.
- Optional "Lock again when Windows locks" (device-local setting): on session lock, close the store and drop the key from memory.
- `src/settings` → Security: "Require Windows Hello to open" toggle (prompts Hello to turn on and off), the relock option, and a plain warning about what happens if Hello is reset.

Out of scope: Linux Secret Service unlock (`keyring`, later), unlocking with a FIDO key, per-feature Hello prompts (for example before sending mail).

## Acceptance criteria

`features/core/hello-unlock.feature` with a fake Hello adapter; the real adapter is covered by the QA procedure.

```gherkin
Feature: Windows Hello unlock

  Scenario: Turn on Hello gating
    Given Windows Hello is set up
    When I turn on "Require Windows Hello to open" and pass the Hello prompt
    Then store.key's header lists DPAPI then Hello
    And a phrase escrow file exists

  Scenario: Start with Hello on
    Given Hello gating is on
    When the app starts at sign-in with --background
    Then the tray shows "Locked" and no scheduler job, connector poll, or sync session runs
    And after I pass the Hello prompt the store opens and missed reminders run catch-up

  Scenario: The user cancels the Hello prompt
    Given Hello gating is on
    When I cancel the prompt
    Then the app stays locked and offers "Try again" and "Use recovery phrase"

  Scenario: Hello is not set up on this PC
    Given the PC has no Windows Hello PIN or biometrics
    When I open Settings → Security
    Then the toggle is disabled with "Set up Windows Hello in Windows Settings first" and a link

  Scenario: Hello key was reset
    Given Hello gating is on and the Hello key no longer exists
    When the app starts
    Then I see "Windows Hello was reset. Unlock with your recovery phrase."
    And entering the correct phrase opens the store and asks to set up Hello again

  Scenario: Wrong recovery phrase
    When I unlock with a phrase from another group
    Then the store stays locked with "That phrase does not belong to this profile"

  Scenario: Relock on Windows lock
    Given "Lock again when Windows locks" is on
    When the Windows session locks
    Then the store closes and the key is zeroized
    And the next hotkey after unlock prompts Hello
```

## Design notes

- Verify before coding: that `KeyCredential.RequestSignAsync` over the same challenge yields a deterministic signature (RSA PKCS#1 v1.5 is deterministic; check the algorithm Hello uses), that it works for a per-user, unpackaged desktop app, and how it behaves over Remote Desktop. If signatures are not deterministic, use a different binding (for example a Hello-protected key in the TPM through NCrypt with a Hello UI policy).
- `arch.toml`: no new crates. `platform-windows` holds the adapter, `store` the wrapper chain; `windows` stays a direct dependency of `platform-windows` only.
- While locked, events and IPC commands that need the store return `store_locked`. The UI shows one locked screen instead of failing per feature.
- Reminders do not fire while the store is locked. Settings must say so plainly when the user turns gating on, especially with autostart.
- Key material uses `secrecy` and `zeroize`; the Hello signature and derived key never leave memory and are never logged.
- Security review required (CODEOWNERS on crypto).

## QA procedure

1. On a laptop with a Hello PIN, open Settings → Security, turn on "Require Windows Hello to open", and pass the prompt.
2. Quit and restart the app. The tray shows Locked and the main window shows the unlock screen. Pass Hello. Reminders and Today load.
3. Restart and cancel the prompt. The app stays locked and offers the phrase.
4. Turn on "Lock again when Windows locks". Press Win+L, sign back in, and press the dictation hotkey. Hello prompts first.
5. In Windows Settings, remove and re-add the PIN (which resets Hello keys), then start the app. The "Windows Hello was reset" screen appears; the recovery phrase unlocks it.
6. On a PC without Hello, the toggle is disabled with the setup link.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed (security.md: escrow and locked startup)
