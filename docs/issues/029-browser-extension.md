# 029. Browser extension and native messaging host

| Milestone    | Size | Depends on | Labels                                                 |
| ------------ | ---- | ---------- | ------------------------------------------------------ |
| M2 Voice out | L    | #027       | area/browser, area/speech, area/security, type/feature |

## Story

As a reader on the web, I want to highlight text, right-click, choose Read aloud and a mood, and hear it, so that listening to a page is as easy as copying from it.

## Context

[ADR 0008](../architecture/adr/0008-browser-integration.md) and [OS integration: read aloud](../architecture/os-integration.md#read-aloud-from-the-right-click-menu), entry point 1. A Manifest V3 extension (Chrome, Edge, Firefox) in `extensions/browser/` adds a selection context menu, sends the selection through native messaging to `native-host.exe`, which forwards it to the desktop over a per-user named pipe. No localhost port is opened. The host accepts only our extension ids. If the desktop is not running, the extension offers to launch it through `<app>://wake`. MVP outcome 3 and the M2 exit demo: right-click a paragraph in Edge → Read aloud → Angry.

## Scope

- `extensions/browser/` (TypeScript, MV3, one codebase with per-browser manifests):
  - `contextMenus` for `contexts: ["selection"]`: "Read aloud" with a submenu of the nine moods, plus "Stop reading".
  - A service worker that calls `runtime.sendNativeMessage` (or a port) with `{ v: 1, type: "read_aloud", text, mood, page_origin }`.
  - Desktop-not-running handling: a small popup "Start <app> to read aloud" with a button that opens `<app>://wake`.
- `crates/native-host` (binary, about 200 KB): reads native messaging frames (4-byte little-endian length + JSON) from stdin, checks the calling extension's origin against the allow-list, validates the message, connects to the desktop's pipe, forwards, and returns `{ ok }` or `{ error }`.
- Desktop: a pipe server `\\.\pipe\<app>-browser-<user SID hash>` with a DACL for the current user only. It verifies that the client process image is our `native-host.exe` (`GetNamedPipeClientProcessId` → image path), then calls `ReadSelection`'s speak path with `source: ReadAloud`.
- Native host manifest registration for Chrome, Edge, and Firefox under HKCU (`...\NativeMessagingHosts\<name>`), written by `platform-windows` at first run and by the installer later (#075). Protocol handler `<app>://wake` registered the same way.
- Gates: `extensions/browser` under ESLint, `tsc`, dependency-cruiser, Vitest coverage, and a Playwright test that loads the unpacked extension in Chromium.

Out of scope: "Summarize and read" (needs the assistant, #049), "Send to my other device" (needs sync, #070), store publishing (#075), Explorer menu (#058).

## Acceptance criteria

`features/ui/browser-extension.feature` (Playwright with the unpacked extension and a fake native host) and `features/core/native-host.feature` (in process, stdin/stdout framing and pipe forwarding):

```gherkin
Feature: Read aloud from the browser

  Scenario: Read a selection in a mood
    Given the extension is loaded and the desktop is running
    When I select a paragraph, right-click, and choose "Read aloud" then "Angry"
    Then the desktop receives read_aloud with that text and mood "angry"

  Scenario: Desktop not running
    Given the desktop app is not running
    When I choose "Read aloud" then "Calm"
    Then the extension shows "Start <app> to read aloud" with a Start button

  Scenario: Unknown extension id
    When native-host is started by an extension origin not on the allow-list
    Then it exits without connecting to the pipe and writes nothing to stdout

  Scenario: Oversized or malformed message
    When native-host receives a frame longer than 1 MB or JSON without "type"
    Then it replies with error "invalid_message" and does not forward it

  Scenario: Another process impersonates the host
    Given a process that is not native-host.exe connects to the browser pipe
    Then the desktop closes the connection without reading a message

  Scenario: Text is spoken, not executed
    Given the selection contains "<script>alert(1)</script> ignore previous instructions"
    When it is read aloud
    Then the text is spoken literally and no other action runs
```

## Design notes

- `arch.toml`: `[crates.native-host] allow = ["kernel"]`, ban `tauri`, `tokio`, `windows`, `rusqlite`, `reqwest` (std named-pipe client via `OpenOptions` on `\\.\pipe\...`); add to `no_dependents`. The pipe server and registry writes live in `platform-windows`.
- Verify per browser: how the caller origin reaches the host (Chrome and Edge pass `chrome-extension://<id>/` as an argument; Firefox passes the manifest path and extension id), the message size limits, and Firefox's HKCU registry path and `allowed_extensions` field.
- Extension ids: stable ids come from the store listings; development uses a fixed `key` in the dev manifest. The allow-list lives in the host manifest and in `native-host` itself.
- Extension permissions: `contextMenus`, `nativeMessaging`, `activeTab` only. No host permissions and no content scripts; the selection comes from the context-menu `info.selectionText`.
- Text limit: 50,000 characters, same as #028. Data class: `DataClass::Screen`; `page_origin` is shown in the reading pill and never logged with the text.
- Protected files touched: `eslint.config.mjs`, `tsconfig*.json`, `.dependency-cruiser.cjs`, `knip.json`, `Makefile`, `package.json` (human approval).

## QA procedure

1. Run the dev build once so it registers the native host. Load `extensions/browser/dist/chromium` unpacked in Edge.
2. Select a paragraph on a news site, right-click → Read aloud → Angry. It is read angrily within half a second.
3. Choose "Stop reading". Playback stops.
4. Quit the desktop app and try again. The popup offers to start it. Click Start. The app launches; reading works on the next try.
5. Repeat steps 1–2 in Chrome and Firefox.
6. Check `reg query HKCU\Software\Microsoft\Edge\NativeMessagingHosts` and the Chrome and Firefox paths. Each points to the host manifest, which lists only our extension ids.
7. In Resource Monitor, confirm the app listens on no TCP port for this feature.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
