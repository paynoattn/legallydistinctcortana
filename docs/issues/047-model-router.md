# 047. Model router: providers, privacy policy, health checks

| Milestone    | Size | Depends on       | Labels                                  |
| ------------ | ---- | ---------------- | --------------------------------------- |
| M5 Assistant | L    | #006, #010, #040 | area/models, area/privacy, type/feature |

## Story

As a privacy-conscious developer, I want every AI call to go through one router that picks local Ollama, LM Studio, or the bundled model by default, and Claude, OpenAI, DeepSeek, or Gemini only where I allowed it per data class, so that I get the best model for each task without my mail ever leaving by accident.

## Context

Every LLM call names a task, never a model. The router resolves task × privacy policy × capability × availability ([assistant: model router](../architecture/assistant.md#model-router-models-context)). Providers are OpenAI-compatible HTTP (Ollama, LM Studio, llama-server, Foundry Local, vLLM, OpenRouter, DeepSeek, Groq, OpenAI), the Anthropic Messages API, and the Gemini API, all in `connector-llm` behind `ChatModel` and `EmbeddingModel` ([ADR 0007](../architecture/adr/0007-model-runtimes.md)). Vision principle 1: cloud is opt-in per feature, and the provider and the data sent are shown before the first call.

## Scope

- **`models-domain`:**
  - `Task` enum: `dictation-cleanup`, `voice-turn`, `reasoning`, `mail-triage`, `mail-summary`, `feed-summary`, `briefing`, `reminder-parse`, `dictation-translate`, `session-summary`, `routine-compile`, `document-summary`, `embedding` (the table in [assistant](../architecture/assistant.md#model-router-models-context)).
  - `ProviderLocation`: `ThisDevice` (loopback), `MyNetwork` (a LAN host the user labelled), or `Cloud`.
  - `PrivacyPolicy`: per `DataClass` (including `Reminders` and `Location`), the set of allowed providers. The default is `ThisDevice` only, for every class except `Public`.
  - `route(task, data_classes, policy, health, capabilities) -> Route | Refusal(reason, setting_link)`. It is pure and property-tested: no route ever sends a class to a provider the policy does not allow, and `MyNetwork` counts as off-device unless the user allowed it.
  - Explicit fallback chains per task (for example voice-turn: persona model on llama-server → Ollama → refuse).
- **`models-application`:** a `Router` service, the `ChatModel` and `EmbeddingModel` ports, a `ProviderHealth` cache, and a `PolicySource` port. Every call that leaves `ThisDevice` reports through `kernel::AuditSink` `{time, provider, task, data_classes, bytes_out, bytes_in}` with no content. The `privacy` context (#074) owns the log, its storage, and the policy storage; until it lands, the root wires a store-backed sink and reads the policy from a shared setting (#010).
- **`connector-llm` adapters:**
  - OpenAI-compatible chat, streaming, and tool calls, plus `/v1/embeddings`. Presets: Ollama (`http://127.0.0.1:11434/v1`), LM Studio (`http://127.0.0.1:1234/v1`), the bundled llama-server (#048), OpenAI, DeepSeek, OpenRouter, Groq, and a custom base URL. Verify each base URL and its tool-call support.
  - Anthropic Messages API with streaming, tool use, and prompt caching for the long system prompt.
  - Gemini API with streaming and function calling.
- **Health:**
  - Local providers are probed with `GET /v1/models` (cached 60 s, re-probed on failure).
  - Cloud providers mark themselves unavailable on 401 (key invalid) or 429 (until `Retry-After`).
- **First-use disclosure:** the first time a task would use a cloud provider, the call is held and the `confirm` window shows the provider, the task, and the data classes. "Allow" updates the policy; "Keep local" refuses.
- **Settings UI** (`src/models/`):
  - A provider list with health dots and "Add provider" (API keys go to the vault from #040).
  - A routing table: task → chain.
  - A privacy matrix: data class × provider checkboxes, with a "Local only" master switch.

Out of scope: the bundled llama-server and the default model download (#048), the tool loop (#049), and the dashboard UI (#074). Agent CLIs are tools, not providers (#056).

## Acceptance criteria

```gherkin
Feature: Model router

  Scenario: Mail stays local by default
    Given Claude and a healthy local Ollama are configured
    And the privacy policy is the default
    When mail-summary is requested with data class Mail
    Then the route is the local Ollama model
    And no off-device audit record is written

  Scenario: Disallowed class with no local fallback is refused with an explanation
    Given only a Claude provider is configured
    When mail-triage is requested with data class Mail
    Then the request is refused with "Mail is set to local only" and a link to Settings → Privacy

  Scenario: Allowed cloud route is audited
    Given the policy allows class Public for DeepSeek
    When feed-summary for public feed items is routed to DeepSeek
    Then an audit record with provider "DeepSeek", task "feed-summary", class Public, and byte counts is written
    And the record contains no prompt text

  Scenario: First cloud use asks first
    Given the user picked Claude for reasoning but has never used it
    When a reasoning request with class Memory arrives
    Then the confirm window shows provider "Anthropic", task "reasoning", and class "Memory"
    And no request is sent until I choose Allow

  Scenario: Local provider down falls back along the chain
    Given voice-turn's chain is llama-server then Ollama
    And llama-server's /v1/models probe fails
    When voice-turn is requested
    Then the request goes to Ollama

  Scenario: Rate-limited cloud provider is skipped until Retry-After
    Given OpenAI answered 429 with Retry-After 120 for reasoning
    When reasoning is requested 30 seconds later
    Then the next provider in the chain is used

  Scenario: LAN Ollama counts as off-device
    Given an Ollama provider at a LAN address labelled MyNetwork
    And class Mail is ThisDevice only
    When mail-summary is requested
    Then that provider is not used for it
```

## Design notes

- **Crates:** create `models-domain` and `models-application`, and build `connector-llm` (on `reqwest` + rustls, with SSE streaming). `models-infrastructure` holds the policy and provider config storage and the audit table; #018's catalog also lives in the models context.
- **arch.toml:** `models-domain` allows `kernel`. `models-application` allows `models-domain` and `kernel`. `connector-llm` allows `models-application`, `models-domain`, `kernel`, and `accounts-application` (API keys).
- **API keys** are stored in the credential vault from #040.
- **Migrations:** `models_providers` (synced, without keys), and, until #074 moves them into the privacy context, the policy as a synced shared setting and a device-local audit table (30-day retention).
- **Security:** only loopback providers count as `ThisDevice`. A non-loopback URL can never be labelled `ThisDevice`. Prompts and responses are never logged. API keys are never sent to the webview.
- **Testing:** property tests for `route`, and contract tests per adapter against recorded SSE streams (OpenAI, Anthropic, Gemini formats). A local Ollama is not required in CI.

## QA procedure

Setup: Ollama installed with a small model pulled. A QA Anthropic API key (QA-ANTHROPIC) and a QA DeepSeek key (QA-DEEPSEEK).

1. Settings → Models: observe Ollama detected with a green dot. Add QA-ANTHROPIC and QA-DEEPSEEK keys. Observe that the keys are masked and never shown again.
2. Settings → Privacy: observe every class set to local only.
3. Ask the assistant (or use the dev "route test" button) for a mail summary. Observe in the routing log that Ollama was used.
4. Set reasoning to Claude and ask a reasoning question. Observe the disclosure dialog naming Anthropic and the data classes. Choose Keep local and observe the local answer. Repeat and choose Allow, and observe the Claude answer plus an audit record.
5. Quit Ollama (tray → Quit). Observe a red dot within 60 s and the fallback in use.
6. Enter an invalid DeepSeek key. Observe "Key rejected", and the provider skipped.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
