# 009. Tray, autostart, single instance, redacted logging

| Milestone      | Size | Depends on | Labels                                       |
| -------------- | ---- | ---------- | -------------------------------------------- |
| M0 Foundations | S    | #004       | area/platform, area/foundation, type/feature |

## Status

**Tray, close to tray, single instance, and start at sign-in implemented** on branch `feat/overlay-tray-settings` (2026-11-18). It differs from the design below:

- The tray menu is the voice server's status (greyed out), Settings, Pause shortcuts, and Quit. Left click opens Settings. The tooltip names the app and the status, checked every 30 seconds and whenever the voice server settings change (`speech_infrastructure::check_servers`).
- Closing the main or settings window hides it. The main window starts hidden and is shown unless the app starts with `--background`.
- A second launch opens the running app's settings window. Arguments are not forwarded yet.
- Start at sign-in is the "Start at sign-in" setting (#010), with `--background` only; there is no Today mode yet. Debug builds never change the sign-in items. If Windows refuses the change, the save is refused on that field.
- Open: `tauri-plugin-autostart` 2.7 (through `auto-launch` 0.6) writes the HKCU Run value as `<exe path> --background` without quoting the path. It still starts the app when the path has spaces, but Windows tries each shorter prefix first, so a file such as `…\Programs\My.exe` placed in the user's profile would run instead. Replace it with a Run-key adapter in `platform-desktop` that quotes the path.
- Not done: the brand's 16 px tray icon (it uses the window icon), file logging with redaction and rotation, and the stall watchdog.
- QA: [`qa/procedures/tray-and-settings.md`](../../qa/procedures/tray-and-settings.md).

## Story

As a user, I want the assistant to live quietly in my tray, start when I sign in, and never run twice, so that reminders and hotkeys work all day without a window open.

## Context

[Overview: runtime topology](../architecture/overview.md#runtime-topology) (desktop starts at login with `--background` or `--today`), [os-integration: Autostart](../architecture/os-integration.md), [security: storage encryption](../architecture/security.md#storage-encryption) (logs are redacted and rotated 5 × 5 MB), and [ADR 0002](../architecture/adr/0002-process-model.md) (log event-loop stalls over 250 ms).

## Scope

- **Tray icon** with the brand's 16 px icon and a menu: Open, Today (disabled until #066), Pause hotkeys, Settings, Quit. Left click opens the main window.
- **Close to tray:** closing the main window hides it; Quit exits the process.
- **Single instance** (`tauri-plugin-single-instance`): a second launch focuses the running instance and forwards its arguments (`--today` opens Today when available).
- **Autostart** (`tauri-plugin-autostart`, HKCU Run): off by default in dev, chosen during onboarding later. Modes: "start hidden in tray" (`--background`) or "show Today at sign-in" (`--today`).
- **Launch flags:** `--background` creates windows hidden; no flag shows `main`.
- **Logging:** `tracing` to `%LOCALAPPDATA%\<app>\logs\desktop.log`, JSON lines, rotated at 5 MB, 5 files kept. A redaction layer drops any field typed `kernel::Redacted` and any field named in a deny-list (`text`, `transcript`, `body`, `subject`, `token`, `password`, `phrase`).
- **Stall watchdog:** a timer task measures tokio scheduling delay and logs a warning with the delay when it exceeds 250 ms.

Out of scope: the Today window (#066), the settings UI for autostart (#010), crash reports (post-MVP).

## Acceptance criteria

`features/core/logging.feature` for the redaction and rotation logic (in process), and the rest as a scripted Windows QA run.

```gherkin
Feature: Tray, single instance, and logs

  Scenario: Background launch shows no window
    When the app starts with "--background"
    Then the tray icon appears and no window is visible

  Scenario: Second launch focuses the first
    Given the app is running in the tray
    When the user starts it again from the Start menu
    Then no second process remains and the main window is shown

  Scenario: Autostart registers the chosen mode
    When autostart is enabled with "show Today at sign-in"
    Then the HKCU Run entry launches the app with "--today"
    And disabling autostart removes the entry

  Scenario: Sensitive fields never reach the log file
    Given a log event with field transcript = "my PIN is 4321" and a Redacted subject
    When it is written
    Then the log line contains "[redacted]" and not "4321"

  Scenario: Logs rotate at 5 MB and keep 5 files
    Given 30 MB of log output
    Then the logs folder holds at most 5 files, each at most 5 MB

  Scenario: Event-loop stalls are reported
    Given a task blocks the runtime for 400 ms
    Then a warning "core stall 400 ms" is logged
```

## Design notes

- Lives in `desktop` (wiring) plus a small `observability` module that is unit-tested. Put the redaction layer and rotation policy in a library crate (`telemetry-local`, allow `["kernel"]`, ban `tauri`, `windows`) so they count toward coverage; `desktop` is excluded from coverage.
- `tracing-appender` rotates by time, not size (verify); size-based rotation needs a small writer of our own or a crate like `file-rotate` (check its license with cargo-deny).
- Plugins need capability entries for `main` only (protected `capabilities/**`). `Cargo.toml` changes need human approval.
- No privacy data classes leave the device. Logs stay local; the redaction rule is the privacy control.
- Idle budget: the tray-resident app with no models loaded stays under 1% CPU and 300 MB RAM (measured in QA, enforced later by #077).

## QA procedure

1. Run the dev build with `--background`. Only the tray icon appears.
2. Launch it again from a terminal. No second process appears in Task Manager; the main window is shown.
3. Close the main window. The app stays in the tray. Choose Quit. The process exits.
4. Enable autostart (temporary dev toggle or `reg query HKCU\Software\Microsoft\Windows\CurrentVersion\Run`). Sign out and in. The app starts in the tray.
5. Open `%LOCALAPPDATA%\<app>\logs\desktop.log`. Entries are JSON lines with no message bodies, transcripts, or tokens.
6. Watch Task Manager for 5 minutes idle. CPU stays under 1% and memory under 300 MB.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
