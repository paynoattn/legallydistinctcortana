# 075. Installer, infer-host flavors, signing, updater

| Milestone  | Size | Depends on       | Labels                                    |
| ---------- | ---- | ---------------- | ----------------------------------------- |
| M8 Release | L    | #015, #029, #005 | area/release, area/security, type/feature |

## Story

As a new user on a Windows 11 laptop without administrator rights, I want to download one signed installer from GitHub Releases, install it in a minute, get the right inference build for my hardware, and receive verified updates, so that I can start using the assistant safely and keep it current.

## Context

MVP done-criterion 1 and the M8 exit demo. [os-integration: installer and runtime requirements](../architecture/os-integration.md#installer-and-runtime-requirements); supply-chain and update threats in [security: threat model](../architecture/security.md#threat-model-stride-summary). `infer-host` flavors come from [#015](015-infer-host.md); the native messaging host from [#029](029-browser-extension.md). The bundle identifier must be final before the first public release ([ADR 0011](../architecture/adr/0011-brand-neutral-code.md), [#005](005-finalize-brand-and-identifier.md)).

## Scope

- **NSIS installer** (per user, no admin, `%LOCALAPPDATA%\Programs\<app>`), the default download for x64 and ARM64. **MSI** for managed deployment. The WebView2 Evergreen bootstrapper is included for machines without the runtime.
- **infer-host flavors:** the installer ships CPU+Vulkan; it runs `infer-host --probe` and, on NVIDIA GPUs, offers the CUDA flavor as an optional verified download. QNN and OpenVINO flavors are [#088](088-npu-builds.md).
- **Registrations** (all HKCU): AUMID and Start-menu shortcut with the activation setup chosen by spike [#064](064-spike-actionable-toasts.md); `<app>://` protocol handler; native messaging host manifests for Chrome, Edge, and Firefox; autostart if chosen in onboarding.
- **Signing:** Authenticode for the installer and every executable and DLL we build (`desktop`, `infer-host` flavors, `native-host`, `mcp-server`, `headless`), through SignPath Foundation or Azure Trusted Signing (verify eligibility and terms for this project).
- **Updater:** Tauri updater with a minisign key kept offline; `latest.json` and signatures on GitHub Releases. Check at start and daily; show release notes; install on the user's consent; restart. Setting to turn update checks off. Each check is an audit-log entry ([#074](074-privacy-dashboard.md)).
- **Release workflow:** on a version tag, CI builds reproducibly with `--locked` and `--frozen-lockfile`, runs `make gauntlet`, signs, publishes a draft release with SHA-256 sums, and a human publishes it.
- **Uninstall:** removes binaries, registrations, and models; asks before deleting the profile.

Out of scope: Microsoft Store/MSIX packaging (the sparse package for #058 is post-MVP); auto-install without consent; NPU flavors (#088).

## Acceptance criteria

```gherkin
Feature: Install, sign, update

  Scenario: Install without administrator rights
    Given a clean Windows 11 VM signed in as a standard user
    When I run the NSIS installer
    Then no UAC prompt appears
    And the app is installed under "%LOCALAPPDATA%\Programs" with a Start-menu shortcut
    And onboarding opens when the installer finishes

  Scenario: Everything we ship is signed
    When I check every .exe and .dll from our build in the install folder with "Get-AuthenticodeSignature"
    Then each has status "Valid" and the project's publisher name

  Scenario: The CUDA flavor is offered on NVIDIA hardware
    Given a PC with an NVIDIA RTX GPU
    When the installer's hardware probe runs
    Then it offers "Faster on your NVIDIA GPU (download)" and installs the CUDA flavor only after its SHA-256 matches

  Scenario: Verified update
    Given version 0.1.0 is installed and 0.1.1 is published with a valid signature
    When the update check runs and I accept
    Then 0.1.1 is installed, the app restarts, and my profile, models, and settings are intact

  Scenario: Tampered update is rejected
    Given an update whose signature does not match the embedded public key
    When the updater downloads it
    Then it is discarded, the current version keeps running, and I see "The update could not be verified"

  Scenario: Browser extension works after install
    Given Chrome, Edge, and Firefox are installed and the extension is added
    When I choose "Read aloud" on a selection
    Then the native host connects to the app without further setup

  Scenario: Update checks can be turned off
    Given update checks are off
    When the app runs for 24 hours
    Then it makes no request to GitHub and the audit log has no update-check entry

  Scenario: Uninstall keeps the profile unless asked
    When I uninstall and choose "Keep my data"
    Then binaries, protocol handler, native host manifests, autostart entry, and models are removed
    And the profile database remains and is used after reinstalling
```

## Design notes

- **Config:** `crates/desktop/tauri.conf.json` (protected: human approval) for bundle targets, NSIS hooks, updater endpoint and public key; NSIS hook scripts in `crates/desktop/installer/`. The release workflow in `.github/workflows/` is protected as well.
- **Size:** installer around 10–15 MB before models ([ADR 0001](../architecture/adr/0001-tauri-and-rust-core.md)); the CUDA flavor is a separate download because the CUDA runtime is large.
- **Models:** the installer ships only the VAD and default wake-word model ([voice-and-speech](../architecture/voice-and-speech.md#speech-to-text-engines-in-infer-host)); everything else comes from the model manager.
- **ARM64:** build and sign `aarch64-pc-windows-msvc`. CUDA is x64 only; verify whether the Vulkan flavor is usable on Snapdragon GPUs or whether ARM64 ships CPU-only until #088.
- **Keys:** the minisign private key and signing credentials never live in the repository or CI logs; release signing is a protected CI environment with required human approval.
- **Firewall:** sync ([#070](070-transport-discovery.md)) listens on UDP; record whether the installer can or should pre-create a rule without admin (verify), and document the first-run prompt otherwise.

## QA procedure

1. On a clean Windows 11 x64 VM as a standard user, download the installer from a draft GitHub release, check its signature in file Properties → Digital Signatures, and install. Observe no UAC prompt and onboarding at the end.
2. In PowerShell run `Get-ChildItem "$env:LOCALAPPDATA\Programs\<app>" -Recurse -Include *.exe,*.dll | Get-AuthenticodeSignature` and confirm every file we build is Valid.
3. On a PC with an NVIDIA GPU, observe the CUDA offer; accept and confirm dictation uses CUDA in Settings → Models.
4. Install 0.1.0, publish a test 0.1.1 to a staging release channel, trigger "Check for updates", accept, and confirm the version and data afterwards.
5. Serve a staging update with a bad signature; confirm rejection.
6. Install the extension in Edge, select text on a page, and choose Read aloud → Calm.
7. Uninstall with "Keep my data"; check `reg query HKCU\Software\Classes\<app>` and the native host keys are gone; reinstall and confirm data is back.
8. Repeat steps 1–2 on an ARM64 device if available.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check` (installer scenarios run in the release QA job on a Windows VM)
- [ ] `make mutate SCOPE=changed` at 100% for any Rust or TypeScript logic added (probe selection, update policy)
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] os-integration.md updated with the final registration list and signing provider
