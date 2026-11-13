# MVP plan

The MVP is **v0.1: a Windows 11 assistant someone can use all day instead of OpenWhispr plus their calendar and mail notifications**, with nothing leaving their machines by default. It covers dictation, read-aloud with moods, reminders, calendar and mail awareness, a voice assistant that can act, the Today screen, and sync between the user's own computers.

Architecture: [`../architecture/overview.md`](../architecture/overview.md). Issues: [`../issues/`](../issues/).

## What "done" means for the MVP

A new user on a Windows 11 laptop with a mid-range GPU or a Copilot+ NPU can:

1. Install without admin rights, create a recovery phrase, and finish onboarding in under 5 minutes, fully offline except model downloads.
2. Hold a key, speak, and see clean text appear in any app within 500 ms of release (GPU), with no network.
3. Select text on a web page, right-click → Read aloud → Alert, and hear it in that mood. The same works with a hotkey in any app.
4. Say or type "remind me every Thursday at 7pm to take out the trash" and get a toast with Snooze and Done at 7pm every Thursday, on whichever of their computers they are using.
5. Connect Outlook or Gmail or any IMAP account (including Proton through Bridge) and get a digest of mail that needs a reply at the interval they chose, summarized by a local model.
6. Connect Google, Microsoft, or iCloud calendars and get a spoken or toast heads-up before meetings with the join link.
7. Ask the assistant "clean up my downloads", see a preview grouped by type and age, confirm, and undo it later.
8. Sign in and land on a full-screen Today view: schedule, mail that needs them, news for their interests, and the state of their projects. Esc or B on a gamepad drops them to the desktop.
9. Type the 24 words on a second PC and have reminders, settings, memory, and routines sync over the LAN or Tailscale, end to end encrypted.
10. Open the privacy dashboard and see that nothing was sent anywhere, or exactly what was and to whom.

Every item above is an acceptance feature (`features/`) and a QA procedure (`qa/procedures/`) before the MVP ships.

## Scope (MoSCoW)

| Must (v0.1)                                                                                                                                                | Should (v0.1 if on track)                                         | Could (v0.2)                                                        | Won't (not before 1.0)            |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- | ------------------------------------------------------------------- | --------------------------------- |
| Push-to-talk and toggle dictation, local whisper.cpp (CPU/Vulkan/CUDA), cleanup rules, dictionary, paste with clipboard restore, overlay                   | Parakeet via sherpa-onnx; LLM cleanup pass; translation dictation | NPU builds (QNN, OpenVINO); dictation history                       | Meeting recording and diarization |
| Read selection aloud (hotkey + browser extension) with moods via prosody fallback; Kokoro + Piper                                                          | Expressive fine-tuned mood voice (if it passes its eval)          | Explorer context menu (#058)                                        | Mobile apps                       |
| Reminders with NL parsing and recurrence, actionable toasts, spoken alerts, local + Microsoft To Do                                                        | Google Tasks, CalDAV VTODO                                        | iCloud Reminders (pending spike #063); location and person triggers | Smart-home control beyond MCP     |
| Calendar: Graph, Google, CalDAV/iCloud, ICS; heads-up with join link                                                                                       | Travel-time lead                                                  |                                                                     |                                   |
| Mail: IMAP/SMTP (iCloud, Proton Bridge, generic), Graph, Gmail via IMAP app password; triage + digests                                                     | Gmail API (pending #062); POP3                                    | JMAP; IMAP IDLE push                                                |                                   |
| Assistant: router (Ollama, LM Studio, llama-server, OpenAI-compatible, Claude, OpenAI, DeepSeek, Gemini), tool loop, confirmations, taint tracking, memory | Routines; MCP client and server; Claude Code / Codex tools        | Wake word (#057)                                                    | Autonomous multi-hour agents      |
| Files cleanup with undo; timers; open apps                                                                                                                 | System toggles (volume, focus)                                    |                                                                     |                                   |
| Today full-screen at sign-in with schedule, mail, news, projects; gamepad navigation                                                                       | Spoken briefing                                                   | Weather                                                             |                                   |
| Feeds: RSS/Atom/JSON, OPML, interest ranking                                                                                                               | HN/Reddit/GitHub presets                                          | Newsletters via mail label                                          |                                   |
| Recovery phrase, encrypted store, LAN + Tailscale sync, single delivery across devices, device removal                                                     | Pairing code (SPAKE2); encrypted folder backup                    | Self-hostable relay                                                 | Cloud service of any kind         |
| Privacy dashboard, installer, signing, updater, accessibility pass                                                                                         | Persona fine-tuned LLM (if it passes its eval)                    |                                                                     |                                   |

## Milestones

Each milestone ends with a demo on a clean Windows 11 VM plus a real laptop, `make gauntlet` green, and the QA procedures for its features run by a person.

| Milestone              | Goal                                                                                       | Exit demo                                                                                   |
| ---------------------- | ------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------- |
| **M0 Foundations**     | Gated monorepo, process skeleton, encrypted store, identity, typed IPC, multi-window shell | Install dev build, create phrase, see tray icon; `make check` green on Windows and Linux CI |
| **M1 Dictation**       | OpenWhispr-parity dictation, local                                                         | Dictate into Notepad, VS Code, Teams, a browser text area offline                           |
| **M2 Voice out**       | Read-aloud with moods everywhere                                                           | Right-click a paragraph in Edge → Read aloud → Angry; hotkey read in Word                   |
| **M3 Reminders**       | Scheduler, reminders, notifications                                                        | "Every Thursday 7pm trash" fires; snooze works; survives sleep and reboot                   |
| **M4 Calendar & mail** | Accounts, heads-ups, digests                                                               | Outlook + Gmail(IMAP) + iCloud calendar connected; 10-minute heads-up; hourly digest        |
| **M5 Assistant**       | Voice/text assistant that acts safely                                                      | "Clean up my downloads" with preview and undo; injection suite green                        |
| **M6 Today**           | Full-screen briefing, feeds, projects                                                      | Sign in → Today; navigate with a controller; "read it to me"                                |
| **M7 Sync**            | Multi-device                                                                               | Second PC joins with the phrase; reminder fires once, on the active PC                      |
| **M8 Release**         | Shippable v0.1                                                                             | Signed installer from GitHub Releases; updater; accessibility and security sign-off         |

Parallel tracks: **ML** (#060, #061, #080–#082) runs from M1 and feeds M2 and M5. **Spikes** (#062–#064) run in M0–M1 so their answers arrive before the milestones that depend on them.

## Critical path

```
001 → 092 → 002 → 006 → 004 → 008 → 007
                                      ├→ 012 → 013 → 015 → 016 → 019 → 020 → 021   (M1)
                                      ├→ 025 → 026 → 027 → 028 → 029                 (M2)
                                      ├→ 031 → 032 → 033 → 034                       (M3)
                                      ├→ 040 → 041 / 042 → 045                       (M4)
                                      ├→ 047 → 049 → 051 → 052                       (M5)
                                      ├→ 066  (M6; needs 041, 045, 059, 065)
                                      └→ 069 → 070 → 071                             (M7; needs 006, 007)
```

## Risks

| Risk                                                    | Impact                                     | Mitigation                                                                           |
| ------------------------------------------------------- | ------------------------------------------ | ------------------------------------------------------------------------------------ |
| Gmail restricted scopes need Google verification        | Gmail API unavailable to most users        | IMAP + app password in MVP; bring-your-own OAuth client guide; spike #062            |
| iCloud Reminders has no API                             | Feature gap for Apple users                | Spike #063; ship without it rather than with a fragile private API                   |
| Actionable toasts from a Tauri app need COM activation  | Snooze and Done buttons may not work       | Spike #064; fallback: toast opens a small in-app reminder card with the same actions |
| Expressive TTS models with clean licenses may be scarce | Moods sound approximate                    | Prosody fallback ships first; commission data (#080)                                 |
| NPU execution providers vary by vendor and driver       | NPU path unreliable                        | GPU and CPU paths are the MVP; NPU builds are "Could"                                |
| Low-level keyboard hooks are flagged by some antivirus  | Hotkeys blocked                            | Signed binaries; hook only when needed; `RegisterHotKey` fallback for toggle mode    |
| Strict gates (100% mutation) slow early velocity        | Missed dates                               | `SCOPE=changed` mutation in PRs; gauntlet before milestones; never loosen gates      |
| Prompt injection through mail and feeds                 | The assistant acts against the user        | ADR 0009 taint model and acceptance suite from M5's first issue                      |
| Windows Firewall consent needs admin for inbound UDP    | Sync listener blocked on per-user installs | Explain before the prompt; outbound-only fallback; MSI adds the rule (#070)          |
| espeak-ng (GPL-3.0) is the common TTS phonemizer        | License conflict with MIT/Apache           | Permissive G2P only; never link espeak-ng (ADR 0007)                                 |
| Latency bench needs a GPU runner                        | Budgets unchecked in CI                    | Self-hosted runner for `main` only, never for fork PRs (#077)                        |

## Issue index

Size: S ≤ 2 days, M ≤ 1 week, L ≤ 2 weeks, XL needs splitting before work starts. Every issue is a story for the gauntlet (`/gauntlet`): specifier → coder → cleaner → hardener → qa.

### M0 Foundations

| #   | Issue                                                                                                                     | Size | Depends on    |
| --- | ------------------------------------------------------------------------------------------------------------------------- | ---- | ------------- |
| 001 | [Repository bootstrap: combined starters, gates green on Windows and Linux](../issues/001-repository-bootstrap.md)        | M    |               |
| 002 | [Kernel crate: ids, clock, HLC, data classes, privacy levels, mood](../issues/002-kernel-crate.md)                        | S    | 001           |
| 003 | [Replace the starter example with reminders, local-api, and headless skeletons](../issues/003-replace-starter-example.md) | M    | 002           |
| 004 | [Multi-window shell with per-entry bundle budgets](../issues/004-multi-window-shell.md)                                   | M    | 001           |
| 005 | [Finalize brand, product name, and bundle identifier](../issues/005-finalize-brand-and-identifier.md)                     | S    |               |
| 006 | [Encrypted store: SQLCipher, migrations, DPAPI key wrapping](../issues/006-encrypted-store.md)                            | M    | 002, 092      |
| 007 | [Identity: recovery phrase, key derivation, onboarding](../issues/007-identity-and-onboarding.md)                         | M    | 006, 004, 008 |
| 008 | [Typed IPC with tauri-specta, freshness gate, and fake adapter](../issues/008-typed-ipc.md)                               | M    | 004           |
| 009 | [Tray, autostart, single instance, redacted logging](../issues/009-tray-autostart-logging.md)                             | S    | 004           |
| 010 | [Settings framework: device-local and shared settings](../issues/010-settings-framework.md)                               | S    | 006, 008      |
| 011 | [Domain event bus and UI subscription stores](../issues/011-event-bus.md)                                                 | S    | 002, 008      |
| 092 | [Unsafe and FFI policy gate (ADR 0012)](../issues/092-unsafe-and-ffi-policy.md)                                           | S    | 001           |

### M1 Dictation

| #   | Issue                                                                                                     | Size | Depends on    |
| --- | --------------------------------------------------------------------------------------------------------- | ---- | ------------- |
| 012 | [Global hotkeys with key up/down (push-to-talk, toggle) and bindings UI](../issues/012-global-hotkeys.md) | M    | 009, 092      |
| 013 | [Audio capture, device selection, level metering](../issues/013-audio-capture.md)                         | M    | 011           |
| 014 | [Voice activity detection (Silero)](../issues/014-vad.md)                                                 | S    | 013           |
| 015 | [infer-host: protocol, supervision, hardware probe](../issues/015-infer-host.md)                          | L    | 002           |
| 016 | [STT engine: whisper.cpp (CPU, Vulkan, CUDA)](../issues/016-stt-whisper.md)                               | M    | 015           |
| 017 | [STT engine: Parakeet via sherpa-onnx (CPU, DirectML)](../issues/017-stt-parakeet.md)                     | M    | 015           |
| 091 | [Cloud speech-to-text (opt-in, own key)](../issues/091-cloud-stt.md)                                      | S    | 016, 047      |
| 018 | [Model manager: catalog, verified resumable downloads, recommendations](../issues/018-model-manager.md)   | M    | 015, 006      |
| 019 | [Dictation session, cleanup rules, dictionary, snippets](../issues/019-dictation-session.md)              | M    | 012, 014, 016 |
| 020 | [Text injection with clipboard restore and target-aware formatting](../issues/020-text-injection.md)      | M    | 019           |
| 021 | [Overlay window and animated mark states](../issues/021-overlay.md)                                       | M    | 004, 011      |
| 022 | [Optional LLM cleanup pass with bounded edits](../issues/022-llm-cleanup.md)                              | S    | 019, 047      |
| 023 | [Translation dictation mode](../issues/023-translation-dictation.md)                                      | S    | 019, 047      |
| 024 | [Dictation history (opt-in, encrypted)](../issues/024-dictation-history.md) (Could: may slip to v0.2)     | S    | 019, 006      |

### M2 Voice out

| #   | Issue                                                                                        | Size | Depends on    |
| --- | -------------------------------------------------------------------------------------------- | ---- | ------------- |
| 025 | [TTS engines: Kokoro and Piper with sentence streaming](../issues/025-tts-engines.md)        | M    | 015, 018      |
| 026 | [Speech queue, priorities, playback, barge-in](../issues/026-speech-queue.md)                | M    | 025, 012      |
| 027 | [Moods: style vectors, prosody fallback, voice picker with previews](../issues/027-moods.md) | M    | 026           |
| 090 | [Windows OneCore voice fallback (zero download)](../issues/090-onecore-voice-fallback.md)    | S    | 026           |
| 028 | [Read selection by hotkey with mood picker](../issues/028-read-selection-hotkey.md)          | M    | 012, 027      |
| 029 | [Browser extension and native messaging host](../issues/029-browser-extension.md)            | L    | 027           |
| 030 | [Reading pill: pause, stop, mood, speed](../issues/030-reading-pill.md)                      | S    | 021, 026, 012 |

### M3 Reminders, scheduler, notifications

| #   | Issue                                                                                          | Size | Depends on |
| --- | ---------------------------------------------------------------------------------------------- | ---- | ---------- |
| 031 | [Scheduler: durable jobs, RRULE, catch-up after sleep, leases](../issues/031-scheduler.md)     | L    | 006, 011   |
| 032 | [Reminders and natural-language parsing](../issues/032-reminders-nl.md)                        | M    | 031, 003   |
| 033 | [Notification delivery policy: quiet hours, focus, batching](../issues/033-delivery-policy.md) | M    | 011        |
| 034 | [Actionable toasts (snooze, done, join)](../issues/034-actionable-toasts.md)                   | M    | 033, 064   |
| 035 | [Spoken alerts](../issues/035-spoken-alerts.md)                                                | S    | 026, 033   |
| 036 | [Microsoft To Do connector](../issues/036-connector-ms-todo.md)                                | M    | 032, 040   |
| 037 | [Google Tasks connector](../issues/037-connector-google-tasks.md)                              | S    | 032, 040   |
| 038 | [CalDAV VTODO connector](../issues/038-connector-caldav-vtodo.md)                              | M    | 032        |
| 039 | [Reminders UI with recurrence editor](../issues/039-reminders-ui.md)                           | M    | 032, 008   |

### M4 Calendar and mail

| #   | Issue                                                                                                 | Size | Depends on                                          |
| --- | ----------------------------------------------------------------------------------------------------- | ---- | --------------------------------------------------- |
| 040 | [Accounts: OAuth loopback + PKCE, credential vault, accounts UI](../issues/040-accounts-and-oauth.md) | L    | 006, 008                                            |
| 041 | [Calendar sources (Graph, Google, CalDAV/iCloud, ICS) and heads-up alerts](../issues/041-calendar.md) | L    | 040, 033                                            |
| 094 | [Travel-time lead for calendar heads-ups (Should)](../issues/094-travel-time-lead.md)                 | S    | 041                                                 |
| 042 | [Mail: IMAP/SMTP (iCloud, Proton Bridge, generic) and POP3](../issues/042-mail-imap-pop3.md)          | L    | 040                                                 |
| 043 | [Mail: Microsoft Graph](../issues/043-mail-graph.md)                                                  | M    | 040                                                 |
| 044 | [Mail: Gmail via IMAP app password, API behind spike](../issues/044-mail-gmail.md)                    | S    | 042, 062                                            |
| 045 | [Mail triage and digests at intervals](../issues/045-mail-triage-digests.md)                          | L    | 042, 047, 033                                       |
| 046 | [Connector leases: one poller per account](../issues/046-connector-leases.md)                         | S    | 031 (cross-device takeover completes with 069, 070) |

### M5 Assistant

| #   | Issue                                                                                          | Size | Depends on    |
| --- | ---------------------------------------------------------------------------------------------- | ---- | ------------- |
| 047 | [Model router: providers, privacy policy, health checks](../issues/047-model-router.md)        | L    | 006, 010, 040 |
| 048 | [Bundled llama-server and default local model](../issues/048-bundled-llama-server.md)          | M    | 015, 018      |
| 049 | [Tool loop, tool registry, effects, confirmations, taint tracking](../issues/049-tool-loop.md) | L    | 047, 011      |
| 050 | [Memory notebook](../issues/050-memory-notebook.md)                                            | M    | 049           |
| 051 | [Assistant hotkey, panel, and voice turn](../issues/051-assistant-voice-turn.md)               | M    | 049, 019, 026 |
| 052 | [Files cleanup tool with undo journal](../issues/052-files-cleanup.md)                         | M    | 049           |
| 093 | [Mail tools: search, draft, reply, archive, send (confirmed)](../issues/093-mail-tools.md)     | M    | 042, 043, 049 |
| 053 | [System tools: apps, timers, volume, media, focus](../issues/053-system-tools.md)              | M    | 049           |
| 054 | [Routines: plain language to structured rules](../issues/054-routines.md)                      | M    | 049, 031      |
| 055 | [MCP client and MCP server](../issues/055-mcp.md)                                              | M    | 049           |
| 056 | [Agent CLI tools: Claude Code and Codex](../issues/056-agent-cli-tools.md)                     | M    | 049           |
| 057 | [Wake word (off by default)](../issues/057-wake-word.md) (Could: may slip to v0.2)             | M    | 061, 013      |

### M6 Today, feeds, projects

| #   | Issue                                                                                    | Size | Depends on                        |
| --- | ---------------------------------------------------------------------------------------- | ---- | --------------------------------- |
| 059 | [Feeds and interest ranking](../issues/059-feeds.md)                                     | L    | 031, 047                          |
| 065 | [Projects: git, CI, agent sessions](../issues/065-projects.md)                           | M    | 031                               |
| 066 | [Today full-screen view at sign-in with gamepad navigation](../issues/066-today-view.md) | L    | 004, 009, 012, 041, 045, 059, 065 |
| 067 | [Spoken morning briefing](../issues/067-spoken-briefing.md)                              | S    | 066, 026                          |

### M7 Sync

| #   | Issue                                                                                     | Size | Depends on    |
| --- | ----------------------------------------------------------------------------------------- | ---- | ------------- |
| 068 | [Device certificates, pairing code, devices UI](../issues/068-devices-and-pairing.md)     | M    | 007           |
| 069 | [Op log and HLC last-writer-wins replication](../issues/069-op-log-replication.md)        | L    | 006, 002, 068 |
| 070 | [Transport: QUIC, mDNS, Tailscale, manual peers](../issues/070-transport-discovery.md)    | L    | 068, 069      |
| 071 | [Presence and single delivery across devices](../issues/071-presence-single-delivery.md)  | M    | 070, 033      |
| 072 | [Device removal, key rotation, phrase rotation](../issues/072-device-removal-rotation.md) | M    | 070           |
| 073 | [Encrypted backup to folder and restore](../issues/073-encrypted-backup.md)               | M    | 069           |

### M8 Release

| #   | Issue                                                                                         | Size | Depends on    |
| --- | --------------------------------------------------------------------------------------------- | ---- | ------------- |
| 074 | [Privacy dashboard and off-device audit log](../issues/074-privacy-dashboard.md)              | M    | 047           |
| 089 | [Windows Hello unlock for the store](../issues/089-windows-hello-unlock.md)                   | M    | 006, 092      |
| 075 | [Installer, infer-host flavors, signing, updater](../issues/075-installer-signing-updater.md) | L    | 015, 029, 005 |
| 076 | [Accessibility pass](../issues/076-accessibility.md)                                          | M    | 066           |
| 077 | [Performance budget bench job](../issues/077-performance-bench.md)                            | M    | 020, 026      |
| 078 | [Security review, SECURITY.md, decoder fuzzing](../issues/078-security-review.md)             | M    | 070, 049      |
| 079 | [User guide and contributor guide](../issues/079-user-and-contributor-docs.md)                | M    |               |

### Spikes (time-boxed, answer a question, produce an ADR or update one)

| #   | Issue                                                                                          | Time box | Needed by                     |
| --- | ---------------------------------------------------------------------------------------------- | -------- | ----------------------------- |
| 060 | [Expressive TTS base model selection](../issues/060-spike-expressive-tts-base.md)              | 1 week   | 080, 081                      |
| 061 | [Wake word engine and custom phrase](../issues/061-spike-wake-word.md)                         | 3 days   | 057 (needs the name from 005) |
| 062 | [Google OAuth verification for Gmail scopes](../issues/062-spike-google-oauth-verification.md) | 2 days   | 044                           |
| 063 | [iCloud Reminders access](../issues/063-spike-icloud-reminders.md)                             | 2 days   | Could                         |
| 064 | [Actionable toasts from a Tauri app](../issues/064-spike-actionable-toasts.md)                 | 2 days   | 034                           |

### ML track

| #   | Issue                                                                                            | Size | Depends on |
| --- | ------------------------------------------------------------------------------------------------ | ---- | ---------- |
| 080 | [Mood TTS dataset: consent, license, recording plan](../issues/080-mood-tts-dataset.md)          | L    | 060        |
| 081 | [Mood TTS fine-tune and evaluation harness](../issues/081-mood-tts-finetune.md)                  | XL   | 080        |
| 082 | [Persona LLM dataset, LoRA fine-tune, evaluation harness](../issues/082-persona-llm-finetune.md) | XL   | 049        |

### Post-MVP (tracked, not scheduled)

| #   | Issue                                                                                           |
| --- | ----------------------------------------------------------------------------------------------- |
| 058 | [Explorer context menu: read aloud and summarize files](../issues/058-explorer-context-menu.md) |
| 083 | [Meeting transcription with on-device diarization](../issues/083-meetings.md)                   |
| 084 | [Self-hostable relay for devices on different networks](../issues/084-relay.md)                 |
| 085 | [macOS and Linux platform adapters](../issues/085-macos-linux.md)                               |
| 086 | [Location and person triggers for reminders](../issues/086-location-person-triggers.md)         |
| 087 | [Echo cancellation and voice barge-in](../issues/087-echo-cancellation.md)                      |
| 088 | [NPU builds: QNN and OpenVINO](../issues/088-npu-builds.md)                                     |
