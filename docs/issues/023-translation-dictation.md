# 023. Translation dictation mode

| Milestone    | Size | Depends on | Labels                                |
| ------------ | ---- | ---------- | ------------------------------------- |
| M1 Dictation | S    | #019, #047 | area/voice, area/models, type/feature |

## Story

As a bilingual user, I want to speak in one language and have the text appear in another, so that I can write to colleagues in their language at the speed of speech.

## Context

[Voice: dictation pipeline](../architecture/voice-and-speech.md#dictation-pipeline) lists a translate mode: speak in language A, paste in language B. [Vision: voice in](../product/vision.md#voice-in-dictation-openwhispr-parity-and-beyond) lists translation dictation. This is a "Should" in the [MVP scope](../product/mvp-plan.md#scope-moscow).

## Scope

- A `Translate { source: Auto | Lang, target: Lang }` dictation mode in `voice-domain`, with its own hotkey binding (`hotkeys.dictate_translate`, unbound by default) or a quick toggle in the tray.
- Two routes, picked by a pure rule:
  1. Target is English: whisper's built-in translate task (#016), no LLM needed.
  2. Any other target: transcribe in the source language, then a routed `dictation-translate` task through the model router (#047), local by default.
- Deterministic cleanup runs on the source text before translation; dictionary replacements also apply. Target formatting (#020) runs after.
- Bounded output: reject a translation more than 3× or less than 0.3× the source length, or one that starts with a reply preamble; on rejection insert the source text and say why.
- Settings (shared): `voice.translate_target` (default: none), `voice.translate_source` (default: auto).
- `src/dictation`: a Translation section with the two language pickers and which route will be used.

Out of scope: translating selected text (assistant), speaking translations aloud (#026), cloud STT translation.

## Acceptance criteria

`features/core/translation-dictation.feature` with fake STT and a fake `ChatModel`:

```gherkin
Feature: Translation dictation

  Scenario: Speak German, paste English through whisper
    Given the translate target is English
    When German speech is dictated in translate mode
    Then whisper's translate task is used and English text is inserted
    And the model router is not called

  Scenario: Speak English, paste Spanish through the router
    Given the translate target is Spanish
    And the model returns "Nos vemos el viernes."
    When "see you on Friday" is dictated in translate mode
    Then "Nos vemos el viernes." is inserted

  Scenario: No model can translate under the privacy policy
    Given the target is Spanish and only a cloud model is configured
    And the Voice data class is local only
    When a translate dictation finishes
    Then the English source text is inserted
    And the overlay says "Translation needs a local model"

  Scenario: Implausible translation is rejected
    Given the model returns text ten times longer than the source
    Then the source text is inserted and the reason is shown

  Scenario: Translate mode without a target
    Given no translate target is set
    When the translate hotkey is pressed
    Then the main window opens the Translation settings with "Choose a target language"
```

## Design notes

- The router's task table in [assistant.md](../architecture/assistant.md#model-router-models-context) has no translation task yet. Add `dictation-translate` (local default, low latency) when #047 lands, and update the doc.
- `arch.toml`: no new crates. The translate route rule lives in `voice-domain`; the model call goes through a `TranslationModel` port in `voice-application`, implemented in `voice-infrastructure` over the router.
- Data class: `DataClass::Voice`. The router enforces the policy.
- The translation call has no tools and treats the source text as data in a delimited block.
- Budget: whisper translate meets the normal 500 ms GPU budget; the LLM route targets 1.5 s on a GPU with a local model.

## QA procedure

1. Set the target to English, bind Ctrl+Alt+T to translate mode, and say a sentence in German into Notepad. English text appears.
2. Install the default local model, set the target to Spanish, and say "see you on Friday at noon". Spanish text appears.
3. Stop the local model and repeat. The English text is inserted and the overlay explains that translation needs a local model.
4. Clear the target and press the translate hotkey. The Translation settings open.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed (assistant.md task table)
