# 051. Assistant hotkey, panel, and voice turn

| Milestone    | Size | Depends on       | Labels                                            |
| ------------ | ---- | ---------------- | ------------------------------------------------- |
| M5 Assistant | M    | #049, #019, #026 | area/assistant, area/voice, area/ui, type/feature |

## Story

As a knowledge worker, I want to hold a hotkey, ask a question out loud, and hear a short spoken answer while the text appears in a small panel near my cursor, so that I can use the assistant without leaving what I am doing.

## Context

The assistant hotkey (default Win+Alt+Space, [os-integration](../architecture/os-integration.md)) starts a dictation session in `assistant` mode, so speech goes to the assistant instead of the cursor ([voice-and-speech: dictation pipeline](../architecture/voice-and-speech.md#dictation-pipeline)). The reply appears in the `panel` window (40 KB budget, [ui: windows](../architecture/ui.md#windows-and-entry-points)) and is spoken with the persona model's short replies, in a mood chosen from an allowed set ([assistant: tool loop](../architecture/assistant.md#conversation-and-the-tool-loop-assistant-context)). Replies preempt read-aloud, and alerts preempt replies ([voice-and-speech: requests](../architecture/voice-and-speech.md#requests)).

## Scope

- **Hotkey:** hold Win+Alt+Space to talk (push-to-talk), or tap it to toggle with VAD auto-stop. The overlay shows Listening and then Thinking (#021).
- **Turn:**
  - The final transcript goes to the `TurnRunner` (#049) with task `voice-turn`.
  - Streamed text goes to the panel. Sentences go to the speech queue (`source: AssistantReply`) as they complete, so speech starts before the full reply.
  - The reply mood comes from the model's allowed set, default Neutral.
- **Panel window** (`panel` entry, `src/assistant/ui/panel/`):
  - Pre-created hidden, and shown near the cursor without stealing focus from the user's app until clicked.
  - Contents: the transcript of what you said (editable, "Fix and resend"), the streaming answer, tool activity lines ("Created reminder…"), confirmation chips that open the `confirm` window, and memory proposals (#050).
  - A text input for typing instead of speaking.
  - Esc closes it. Click-away closes it after the reply finishes.
- **Barge-in:** pressing the hotkey while a reply is speaking stops playback at once and starts listening. The stop hotkey or Esc stops speech and keeps the panel.
- **Spoken reply rules:**
  - Speak only when the turn started by voice (setting: "Always speak replies").
  - Never speak while a dictation session is active.
  - Long replies speak the first 2 sentences, then say "the rest is in the panel".
- **Failure paths:**
  - No speech detected → "I didn't catch that", with no model call.
  - No model available → the panel shows "No local model is running", with a link to Settings → Models. Deterministic commands still work.
  - STT failure → the overlay Error state.

Out of scope: the wake word (#057), voice barge-in without the hotkey (#087), and a conversation history browser.

## Acceptance criteria

```gherkin
Feature: Assistant voice turn

  Scenario: Spoken question, spoken short answer
    Given the persona or default local model is ready
    When I hold the assistant hotkey, say "what's next on my calendar", and release
    Then the panel shows my words and a streaming answer
    And the first sentence of the answer starts playing before the full answer has finished generating

  Scenario: Panel does not steal focus
    Given Notepad is focused with the caret in a document
    When I ask a question with the assistant hotkey
    Then the panel is visible and Notepad keeps keyboard focus

  Scenario: Barge-in with the hotkey
    Given the assistant is speaking a reply
    When I press the assistant hotkey
    Then playback stops within 100 ms and the overlay shows Listening

  Scenario: Typed turn is not spoken by default
    When I type "what time is it" in the panel and press Enter
    Then the answer appears in the panel and nothing is spoken

  Scenario: Silence produces no model call
    When I hold the hotkey for 2 seconds without speaking and release
    Then the panel shows "I didn't catch that" and no router call was made

  Scenario: No model available
    Given no chat provider is healthy
    When I ask "summarize my day"
    Then the panel says "No local model is running" with a link to Settings → Models

  Scenario: Action from voice goes through confirmations
    Given the dev External test tool "net.post" from #049 is enabled
    When I say "post hello to example.test"
    Then a confirmation chip appears and nothing is sent until I approve in the confirm window
```

## Design notes

- **Crates:** `voice-application` (assistant mode publishes `AssistantUtterance` instead of injecting text), `assistant-application` (subscribes and runs the turn), and `speech-application` (sentence streaming). The root wires the cross-context handlers. No application crate imports another context's application crate.
- **UI:** `src/assistant/` with `domain/` (turn view model, sentence splitter for display), `application/` (`PanelStore` fed by `TurnProgress` events), `infrastructure/` (Tauri and fake adapters), and `ui/panel/`. The `panel-main.tsx` composition root stays within the 40 KB gzip budget.
- **Window:** the panel uses `focusable: true` but is shown with `SW_SHOWNOACTIVATE`. Focus moves only on click or when the user presses Tab into it. Implement in `platform-windows` next to the overlay code.
- **Budgets** (proposed; confirm in #077): hotkey → panel visible ≤ 100 ms; release → first audio p95 ≤ 1.5 s on GPU with the bundled model.
- **Privacy:** audio stays on device (`DataClass::Voice`). The transcript carries the classes of whatever context the turn assembles.

## QA procedure

Setup: the bundled model ready (#048), a calendar connected (#041), and speakers or a headset.

1. Click into Notepad. Hold Win+Alt+Space and say "what's next on my calendar". Release. Observe the overlay going Listening → Thinking, the panel near the cursor, and a spoken answer starting quickly.
2. Type in Notepad right after the panel appears. Observe the keys going to Notepad.
3. Ask a long question. While it speaks, tap Win+Alt+Space. Observe speech stopping at once and listening starting.
4. Click the panel's text box, type "what time is it", and press Enter. Observe a text answer only.
5. Hold the hotkey silently for 2 s. Observe "I didn't catch that".
6. Unload the model and quit Ollama. Ask something. Observe the "No local model is running" link.
7. Enable the dev `net.post` tool and say "post hello to example.test". Observe a confirmation chip and the confirm window, and nothing sent on Deny. (Repeat with "clean up my download files" once #052 lands.)

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
