# Product vision

> Working name: **the assistant**. The brand is undecided (see [`docs/brand/`](../brand/)). The code is brand-neutral on purpose; the product name lives in `crates/desktop/tauri.conf.json` and `src/shared/brand.ts` only.

## One line

A free, open-source, local-first desktop assistant you talk to and that talks back. It types what you say, reads what you select, watches your mail, calendar, reminders, and feeds, and tells you what matters before you have to ask. Nothing leaves your machines unless you say so.

## Why

- **OpenWhispr** proved people want push-to-talk dictation with local models. It is Electron, its assistant is a one-shot command box, and its sync, team spaces, and notes run through a hosted cloud.
- **Windows Copilot** is a chat window bolted to the taskbar. It does not know your inbox, your calendar, or your projects, and it sends everything to Microsoft.
- **Cortana** (2014–2023) had the right idea: a proactive assistant with a notebook of what it knows about you, reminders by time, place, and person, and a daily briefing. It died because it was a funnel for Bing and M365, not because the idea was wrong.

We build the assistant Cortana should have become: proactive, useful every day, and private by construction.

## Principles

1. **Local first, local by default.** Speech, language models, text-to-speech, storage, and scheduling run on the user's hardware (CPU, GPU, or NPU). Cloud models are opt-in per feature, with the provider and the data sent shown before the first call.
2. **Encrypted at rest, end-to-end in transit.** The local store is encrypted with a key derived from a 24-word seed. Devices sync peer-to-peer over LAN or Tailscale with keys only the user's devices hold. No server of ours ever exists in the data path.
3. **Proactive, not chatty.** The assistant earns attention: it batches, ranks, and summarizes, and it stays quiet when nothing matters. Every proactive behavior has a visible rule the user can edit or turn off.
4. **Actions are explicit and reversible.** Anything that changes the world (sends mail, deletes files, creates events) shows a preview and needs a confirmation, unless the user has granted a standing permission for that exact action. File operations go to the recycle bin, never `rm`.
5. **Fast.** Dictation text lands in under 500 ms after release on a mid-range GPU. The overlay appears in under 50 ms. The app idles under 1% CPU and 300 MB RAM with models unloaded.
6. **Open and extensible.** Connectors, tools, and voices are plugins with a published contract. The assistant speaks MCP both ways: it can use MCP servers, and it exposes itself as one.
7. **Free.** No paid tier, no telemetry, no account. Optional crash reports are opt-in, local-file first, and sent only when the user clicks send.

## Who it is for

| Persona                    | Need                                            | First "wow"                                                                                     |
| -------------------------- | ----------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| **Knowledge worker**       | Drowning in mail, meetings, and tabs            | Morning briefing on the Today screen: 3 emails that need them, next meeting with its prep notes |
| **Developer**              | Many machines, many projects, many AI tools     | "What is the state of my projects?" answered from git, CI, and their Claude/Codex sessions      |
| **Writer / student**       | Fast, accurate dictation without a subscription | Hold a key, speak, and polished text appears at the cursor, offline                             |
| **Accessibility user**     | Hands-free control, text read aloud             | Select any text, right-click, "Read aloud: calm", and hear it in a natural voice                |
| **Privacy-conscious user** | Assistant features without surveillance         | Everything works with the network cable unplugged                                               |

## Capabilities

### Voice in (dictation, OpenWhispr parity and beyond)

- Push-to-talk and toggle hotkeys, global, with a recording overlay in the bottom-right corner that reacts to your voice.
- Local speech-to-text: whisper.cpp (CUDA, Vulkan, DirectML), NVIDIA Parakeet via sherpa-onnx, with NPU execution through ONNX Runtime (QNN for Snapdragon X, OpenVINO for Intel Core Ultra, Vitis AI for Ryzen AI).
- Optional cloud STT with the user's own key.
- AI cleanup pass (punctuation, filler removal, formatting by target app), custom dictionary, voice snippets ("insert my address").
- Translation dictation (speak in one language, paste in another).
- Paste at cursor, with clipboard restore.

### Voice out (text-to-speech with moods)

- Read selected text aloud from anywhere: hotkey, tray, or the Windows right-click menu ("Read aloud", with a mood submenu).
- Moods: neutral, calm, cheerful, alert, urgent, angry, mad (theatrical fury), whisper, deadpan. Mood is a style input to an expressive local TTS model, not a pitch filter.
- Local TTS on GPU/NPU (Kokoro and Piper baseline, expressive fine-tuned model for moods), with a voice picker and per-mood previews.
- Spoken replies from the assistant, interruptible by voice or key.

### Assistant (the brain)

- Wake word ("Hey <name>", on-device, off by default) and a push-to-talk assistant hotkey.
- Conversation with tools: the model plans, calls tools, and asks before acting.
- Model router: local (llama.cpp, Ollama, LM Studio) or cloud (Claude, OpenAI/Codex, DeepSeek, Gemini, OpenRouter) per task, with a per-task privacy level.
- A fine-tuned local "persona" model for voice replies: short, spoken-style answers in the assistant's voice. Big reasoning goes to the routed model.
- Memory ("notebook" in Cortana terms): facts the user told it, viewable and deletable.

### Proactive

- Reminders by time and recurrence ("every Thursday at 7 pm, take out the trash"), by event ("when I get to work"), and by person ("next time I talk to Sam").
- Calendar alerts with travel and prep lead time, joining links, and a spoken heads-up.
- Mail triage and summaries at user-set intervals: what needs a reply, what is waiting on others, what to ignore.
- Feeds (RSS, Atom, JSON Feed, newsletters by mail) filtered against the user's interests, delivered in the briefing.
- Daily briefing on the full-screen **Today** view at startup: schedule, priority mail, news for your interests, and your projects' state.

### Actions (tools)

- Files: "clean up my downloads" (preview, group by type and age, move to recycle bin or archive folders).
- Reminders and events: create, move, complete across providers.
- Mail: draft, reply, archive (send always confirms).
- System: open apps, set focus mode, volume, media, timers.
- Projects: read git status, CI state, and AI-session summaries.
- Hand off to AI coding agents (Claude Code, Codex) and return their result.

### Connectors

- **Mail:** Gmail (OAuth, API), Outlook/Live/M365 (Microsoft Graph), IMAP/POP3/SMTP (any provider), Proton Mail (via Proton Mail Bridge IMAP), Fastmail (JMAP).
- **Calendar:** Google, Microsoft Graph, iCloud (CalDAV), any CalDAV, local ICS.
- **Reminders/tasks:** Microsoft To Do, iCloud Reminders (CalDAV VTODO), Google Tasks, Todoist, local.
- **Feeds:** RSS/Atom/JSON Feed, Hacker News, Reddit, GitHub releases, newsletters by mail label.
- **AI:** Ollama, LM Studio, llama.cpp server, Claude API, OpenAI API, DeepSeek API, Gemini, OpenRouter, any OpenAI-compatible endpoint; Claude Code and Codex CLIs as agent tools; MCP servers.

### Across your devices

- One identity per person: a 24-word BIP-39 seed creates it; entering the seed on another machine joins it.
- Devices find each other on the LAN (mDNS) or over Tailscale, and sync with end-to-end encryption.
- Reminders fire once across devices: on the device you are using.
- "Read this on my laptop", "send this to my desktop".

## Non-goals (for now)

- Mobile apps (a companion is plausible later through the same sync protocol).
- A hosted cloud, accounts, team spaces.
- Smart-home control (MCP servers can add it).
- Meeting recording and diarization (OpenWhispr does this; it is post-MVP here, see the roadmap).

## Platforms

Windows 11 first (x64 and ARM64, because ARM64 laptops have the NPUs). macOS and Linux follow. Tauri keeps that door open; OS integration sits behind ports so each platform is an adapter.
