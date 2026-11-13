# 079. User guide and contributor guide

| Milestone  | Size | Depends on | Labels                  |
| ---------- | ---- | ---------- | ----------------------- |
| M8 Release | M    |            | area/docs, type/feature |

## Story

As a new user, I want a short guide that gets me from download to my first dictation, read-aloud, reminder, and synced second PC, and explains what stays private; as a new contributor, I want a guide that gets me from clone to `make check` green on Windows and explains how the codebase is cut, so that neither of us has to ask in an issue first.

## Context

The product is free and open source with no support desk; the docs are the support. Architecture and engineering docs exist ([overview](../architecture/overview.md), [gate contract](../engineering/gate-contract.md), [principles](../engineering/principles.md)); there is no user-facing guide and no CONTRIBUTING.md. No dependencies in the index, but each page should be written after its feature ships; start with the contributor guide and the outline.

## Scope

- **User guide** in `docs/user/`, one page per task:
  - Install and first run (no admin, model download, what the recovery phrase is and how to store it)
  - Dictation (hotkeys, modes, dictionary and snippets, elevated windows)
  - Read aloud (hotkey, browser extension install for Edge, Chrome, Firefox, moods and voices)
  - Reminders (phrases that work, recurrence, snooze and done)
  - Accounts: Outlook/Microsoft 365, Gmail (app password, and the bring-your-own OAuth client guide if [#062](062-spike-google-oauth-verification.md) keeps it), iCloud (app-specific password), Proton Mail through Bridge, generic IMAP and CalDAV
  - The assistant: what it can do, confirmations, undo, memory notebook, choosing local or cloud models
  - Today view and the spoken briefing (keyboard and gamepad controls)
  - Your devices: joining with the phrase or a code, LAN and Tailscale, firewall prompt, removing a lost device, backups
  - Privacy: the dashboard, data classes, "Local only", what is never collected
  - Troubleshooting: microphone, hotkey conflicts and antivirus, GPU selection, sync not connecting
- **Contributor guide:** `CONTRIBUTING.md` at the root plus `docs/contributing/`: Windows and Linux setup (`make setup`), the gate contract and the gauntlet, the architecture tour (contexts, layers, `arch.toml`, UI features), how to add a connector, a tool, a TTS voice, and a UI feature, the ADR process, protected gate files and why, and `training/` setup for ML contributors. A `CODE_OF_CONDUCT.md` (Contributor Covenant).
- **Docs gates:** internal links checked with `lychee --offline` in CI; every image has alt text; prettier formatting.
- Product name: use the placeholder from `src/shared/brand.ts` until [#005](005-finalize-brand-and-identifier.md) lands, then replace in one pass. "Cortana" appears only in historical context, never as the product name ([ADR 0011](../architecture/adr/0011-brand-neutral-code.md)).

Out of scope: a documentation website (Markdown in the repository is enough for v0.1); translated docs; video tutorials.

## Acceptance criteria

```gherkin
Feature: Documentation

  Scenario: Internal links resolve
    Given the Markdown files under docs/ and the root guides
    When the link check runs in offline mode
    Then every relative link and anchor resolves

  Scenario: A contributor reaches a green gate from the guide
    Given a clean Windows 11 machine with only Git, Rust via rustup, Node, and pnpm installed as CONTRIBUTING.md lists
    When a person follows CONTRIBUTING.md step by step
    Then "make check" passes without steps that are not in the guide

  Scenario: A user installs and dictates from the guide
    Given a person who has not used the app
    When they follow "Install and first run" and "Dictation"
    Then they dictate a sentence into Notepad without asking for help

  Scenario: Every Settings page links to its guide
    When I open any Settings page
    Then its help link opens the matching page of the user guide in the browser

  Scenario: No forbidden product name
    When the docs are searched for "Cortana" outside docs/product and docs/architecture
    Then no match describes the product by that name
```

## Design notes

- Help links from the UI go through a single map in `src/shared/` (page id → URL on the repository's default branch), opened with the OS browser through a validated Rust command, never navigated inside a webview.
- Screenshots in `docs/user/images/`, taken at 100% scale in light mode, with alt text; regenerate them from Playwright runs against the fake IPC adapter where possible so they stay current.
- Style: plain, direct English; second person; short steps with one action each; every Windows path written in full.
- Add `lychee` to the CI docs job; `.github/**` is protected, so a human approves the workflow change.

## QA procedure

1. Give the user guide to a person who has not used the app. On a fresh Windows 11 laptop, they install, dictate into Notepad, read a web paragraph aloud in Edge, and create a recurring reminder, using only the guide. Note every place they get stuck and fix the page.
2. Give CONTRIBUTING.md to a developer new to the repository. On a clean Windows 11 machine, they reach `make check` green. Fix any missing step.
3. Click every Settings help link and confirm the right page opens.
4. Run the link check locally and confirm it passes.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check` (link check and help-link map)
- [ ] `make mutate SCOPE=changed` at 100% for the help-link map
- [ ] Both QA walkthroughs done with real people and their fixes merged
- [ ] README links to the user guide and CONTRIBUTING.md
