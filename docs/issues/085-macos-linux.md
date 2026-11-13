# 085. macOS and Linux platform adapters

| Milestone | Size                                      | Depends on                   | Labels                                      |
| --------- | ----------------------------------------- | ---------------------------- | ------------------------------------------- |
| Post-MVP  | unsized (XL: split per platform and port) | #012, #020, #028, #034, #075 | area/os-integration, type/feature, post-mvp |

## Story

As a person who uses a Mac or a Linux machine next to a Windows PC, I want the same assistant on those systems, syncing with my Windows devices, so that my reminders, voice, and briefing follow me across all of them.

## Context

The [vision](../product/vision.md#platforms): Windows first, macOS and Linux follow. [os-integration](../architecture/os-integration.md): all OS calls go through `platform-ports`, and nothing above the port may `cfg(windows)`. Tauri keeps the door open ([ADR 0001](../architecture/adr/0001-tauri-and-rust-core.md)).

## Scope

- `platform-macos` and `platform-linux` crates implementing every `platform-ports` trait:
  - hotkeys with key up and down: macOS event tap (needs Accessibility permission); Linux X11 via XInput or XRecord, Wayland via the GlobalShortcuts portal (no key-up on some compositors; verify)
  - selection reading: macOS Accessibility API; Linux AT-SPI with clipboard fallback
  - text injection, foreground app, notifications with actions (macOS `UNUserNotificationCenter`; Linux freedesktop notifications or the portal), focus state, autostart, power and session events, audio devices
- Store key wrapping with the macOS Keychain and Linux Secret Service instead of DPAPI.
- `infer-host` flavors: Metal (whisper.cpp, llama.cpp) on macOS; CPU, Vulkan, and CUDA on Linux.
- Installers: signed and notarized `.dmg` for macOS; AppImage and `.deb` for Linux; updater on both.
- CI runs `make check` on macOS and Linux for the new adapters.

Out of scope: iOS and Android (a companion is plausible later through the sync protocol).

## Acceptance criteria

```gherkin
Feature: macOS and Linux platform adapters

Scenario: Dictate on macOS
  Given the app on macOS with Accessibility permission granted
  When I hold the dictation key in TextEdit and speak
  Then the text appears at the cursor

Scenario: Sync across platforms
  Given a Windows PC and a Mac in the same group on the LAN
  When I create a reminder on the Mac
  Then it appears on the Windows PC
```

## Design notes

- Split per platform and per port group (input, output, notifications, packaging) before work starts.
- `arch.toml` rule: only `platform-macos` depends on `objc2`/Apple frameworks; only `platform-linux` on `zbus`/`atspi`/X11 crates.
- A spike on Wayland global hotkeys and selection reading should come first.

## QA procedure

1. On a Mac and on Ubuntu (X11 and Wayland sessions), run the M1–M3 QA procedures: dictation, read aloud, reminders with actionable notifications.
2. Pair each with a Windows PC and run the sync QA procedure from #070.
