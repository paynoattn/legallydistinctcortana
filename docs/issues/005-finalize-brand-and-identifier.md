# 005. Finalize brand, product name, and bundle identifier

| Milestone      | Size | Depends on | Labels                 |
| -------------- | ---- | ---------- | ---------------------- |
| M0 Foundations | S    | none       | area/brand, type/chore |

## Story

As the maintainer, I want the product name, wake phrase, and bundle identifier decided and wired into the three allowed places, so that testers' data folders, autostart entries, and toast registrations never have to move.

## Context

[ADR 0011](../architecture/adr/0011-brand-neutral-code.md) keeps code brand-neutral and limits the name to `crates/desktop/tauri.conf.json`, `src/shared/brand.ts`, and the installer config. The identifier (`dev.localassistant.desktop` today) decides `%APPDATA%\<identifier>`, the AppUserModelID for toasts, and the protocol handler, so changing it after testers install moves their profile and breaks notifications. Five concepts are in [`../brand/`](../brand/); the recommendation is Lampion, with Avocet as runner-up.

## Scope

- Human decision on the name, recorded in a new ADR (0012) with the trademark and package-name evidence.
- Clearance checks: USPTO, EUIPO, WIPO Global Brand Database (Nice classes 9 and 42), domain availability, and the names on crates.io, npm, winget, the Microsoft Store, the Chrome Web Store, Edge Add-ons, and Firefox AMO.
- Wake-phrase sanity check with the wake-word spike owner (#061): syllable count, phonetic distinctness, false-wake risk from common words.
- Wire the decision:
  - `crates/desktop/tauri.conf.json`: `productName`, window titles, `identifier`.
  - `src/shared/brand.ts`: `productName`, `wakePhrase`, `urlScheme`, colors that the tokens import.
  - Icons from the chosen concept into `crates/desktop/icons/` (app icon, tray 16 px, and the Windows sizes Tauri needs).
- A gate that fails if "Cortana" appears in any shipped string, or if the product name appears outside the three allowed files.

Out of scope: design tokens beyond the brand colors (#021, #066), installer config (#075), store listings (#029, #075).

## Acceptance criteria

```gherkin
Feature: One place for the brand

  Scenario: The product name appears only in allowed files
    Given the brand check runs over crates/, src/, extensions/, and the installer config
    When the chosen product name appears in a file other than tauri.conf.json, src/shared/brand.ts, or the installer config
    Then "make lint" fails and lists the file and line

  Scenario: The banned trademark never ships
    When "Cortana" (any case) appears in crates/, src/, extensions/, or a built asset
    Then "make lint" fails with a trademark message

  Scenario: The UI reads the name from brand.ts
    Given the app is opened with the fake IPC adapter
    Then the main window title and the onboarding greeting show the chosen product name

  Scenario: The identifier decides the data folder
    Given a clean Windows user profile
    When the app starts for the first time
    Then its profile folder is created under %APPDATA%\<final identifier>
```

## Design notes

- The brand check can be a `cargo xtask brand` subcommand run by `lint-rs`, or an ESLint rule plus a script for non-TS files. Both touch protected files (`xtask/**` or `eslint.config.mjs`, and the `Makefile`), so the PR needs human approval. The repo name and `docs/` are exempt; docs may name the product and Cortana freely.
- The identifier should be reverse-DNS on a domain the project controls (verify ownership before choosing). It must be final before #006 writes `profile.db` on any tester's machine and before #034 registers an AUMID.
- The vision doc still says the name lives in `src-tauri/tauri.conf.json`; update it to `crates/desktop/tauri.conf.json` in the same PR.
- No privacy data classes are involved.

## QA procedure

1. Read ADR 0012 and confirm it lists the clearance sources checked and their dates.
2. Run `make dev` on Windows 11. The window title, tray tooltip, and taskbar name show the chosen name and icon.
3. Search the repo for the old placeholder "Assistant" in `crates/` and `src/`. It appears only in the allowed files, or not at all.
4. Temporarily add "Cortana" to a string in `src/` and run `make lint`. It fails with the trademark message. Revert.
5. Check `%APPDATA%`. The folder name matches the final identifier.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed (ADR 0012, ADR 0011 status, vision.md path)
