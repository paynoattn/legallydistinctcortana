# 091. Cloud speech-to-text (opt-in, own key)

| Milestone    | Size | Depends on | Labels                                               |
| ------------ | ---- | ---------- | ---------------------------------------------------- |
| M1 Dictation | S    | #016, #047 | area/voice, area/models, area/security, type/feature |

## Story

As a user on an old laptop with no GPU, I want to opt in to a cloud transcription service with my own API key, so that dictation is fast and accurate, knowing exactly which provider hears my voice.

## Context

[Voice: STT engines](../architecture/voice-and-speech.md#speech-to-text-engines-in-infer-host): cloud STT (OpenAI, Groq, Deepgram, any OpenAI-compatible `/audio/transcriptions`) with the user's key; the privacy router must allow `DataClass::Voice` to leave the device. [Security: privacy UX](../architecture/security.md#privacy-ux): every off-device call is recorded through the `AuditSink` port in `kernel`. [Vision principle 1](../product/vision.md#principles): cloud is opt-in per feature, with the provider and the data sent shown before the first call.

## Scope

- A `speech-to-text` task in the model router (#047), local by default. Cloud providers are eligible only if the user's policy allows `Voice` for that provider.
- `voice-infrastructure`: a `RoutedSpeechToText` adapter implementing `SpeechToText`: it asks the router for a route and calls either `infer-client` (local) or the cloud adapter.
- Cloud adapters in `connector-llm`: OpenAI-compatible `POST /v1/audio/transcriptions` (covers OpenAI and Groq; verify Groq's base URL and model names) and Deepgram's pre-recorded API (verify the endpoint and auth header). Audio is sent as 16 kHz mono 16-bit WAV with the language and dictionary prompt when the API supports one.
- API keys reuse the provider configuration and sealed key storage from #047; no new secret store.
- First-use disclosure: before the first cloud transcription, a dialog names the provider, says that dictated audio will be sent, and requires an explicit "Send my voice to <provider>".
- Every cloud call records `AuditSink::record { provider, task: "speech-to-text", data_classes: [Voice], bytes_sent, bytes_received }`, never content.
- Fallback: on network failure, auth failure, or rate limit, use the local engine if one is installed, otherwise fail with a clear error.
- `src/dictation` settings: an engine choice "Cloud (<provider>)", visible only when an eligible provider exists, with a "Sends audio to <provider>" badge.

Out of scope: cloud TTS, streaming cloud STT, the privacy dashboard UI (#074).

## Acceptance criteria

`features/core/cloud-stt.feature` with a recorded HTTP fixture for each provider and a fake `AuditSink`:

```gherkin
Feature: Cloud speech-to-text

  Scenario: Voice is local-only by default
    Given an OpenAI key is configured for chat
    And the Voice data class has no allowed providers
    When a dictation finishes
    Then the local engine transcribes it and no HTTP request is made

  Scenario: First use asks for consent
    Given Voice is allowed for OpenAI and the cloud engine is selected
    When the first cloud dictation finishes
    Then the disclosure names OpenAI and nothing is sent until I accept

  Scenario: Transcribe in the cloud and record the call
    Given consent was given
    When a 10 s dictation finishes
    Then a WAV is posted to /v1/audio/transcriptions and the text is inserted
    And the audit log has one entry with provider "openai", task "speech-to-text", class Voice, and the byte count

  Scenario: Invalid key
    Given the provider answers 401
    When a dictation finishes
    Then the local engine is used if installed
    And Settings shows "OpenAI rejected the API key" with a fix action

  Scenario: Offline with no local model
    Given there is no network and no local STT model
    When a dictation finishes
    Then the session fails with "stt_unavailable" and the overlay says "Offline and no local model installed"

  Scenario: Policy changed after consent
    Given cloud dictation was working
    When I switch "Local only" on
    Then the next dictation is transcribed locally and no request is made
```

## Design notes

- `arch.toml`: no new crates. `voice-infrastructure` gains the routed adapter and may depend on `models-domain` for route types; it never calls `connector-llm` directly, only through the router's port. `connector-llm` stays the only crate here with `reqwest` for these providers.
- Privacy: `DataClass::Voice` leaves the device only under an explicit policy grant plus first-use consent. The router enforces the policy; this issue must not add a bypass.
- Security: HTTPS only; keys sealed in the store (#006) and never logged; request bodies never logged; responses parsed into typed structs.
- Provider limits (maximum upload size, accepted formats, rate limits): verify per provider and split or refuse long recordings accordingly.
- Budget: no local budget applies, but show provider latency in the debug timings.

## QA procedure

1. Configure an OpenAI-compatible provider with your own key in model settings. Dictate. It is transcribed locally and the privacy dashboard (or audit log table) shows no call.
2. Allow Voice for that provider and choose "Cloud" as the dictation engine. Dictate. The consent dialog names the provider; accept. Text appears.
3. Check the audit log. One entry with the provider, task, class Voice, and bytes.
4. Replace the key with a wrong one and dictate. The local engine is used and Settings shows the key error.
5. Unplug the network with no local model installed and dictate. The offline error appears.
6. Switch "Local only" on and dictate. No request is made (check with the audit log).

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed (assistant.md task table: `speech-to-text`)
