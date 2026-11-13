# Assistant: models, tools, permissions, memory, proactivity

Decision records: [ADR 0007](adr/0007-model-runtimes.md) (runtimes), [ADR 0009](adr/0009-untrusted-content-and-tool-permissions.md) (permissions and prompt injection).

## Model router (models context)

Every LLM call names a **task**, never a model. The router resolves `task × privacy policy × capability × availability` to a provider and model.

| Task                                       | Default route                                         | Needs                                                            |
| ------------------------------------------ | ----------------------------------------------------- | ---------------------------------------------------------------- |
| `dictation-cleanup`                        | Local small (persona model or 1–3B)                   | Low latency                                                      |
| `dictation-translate`                      | Local (persona model or a translation-tuned model)    | Faithful translation, no additions                               |
| `voice-turn`                               | Local persona model                                   | Tool calls, ≤ 2 sentence replies                                 |
| `reasoning`                                | User's chosen model (local large, or cloud)           | Tool calls, long context                                         |
| `mail-triage`                              | Local                                                 | Classification, JSON output                                      |
| `mail-summary`, `feed-summary`, `briefing` | Local, with optional cloud                            | Summarization                                                    |
| `reminder-parse`                           | Deterministic parser first; local LLM fallback        | JSON output                                                      |
| `session-summary`                          | Local                                                 | Summarize Claude Code / Codex session logs for the Projects view |
| `routine-compile`                          | Local                                                 | Plain-language routine → structured `Routine` JSON               |
| `document-summary`                         | Local, with optional cloud                            | Summarize a file or web selection                                |
| `embedding`                                | Local (bge-small or nomic-embed via llama.cpp or ORT) | Vectors for memory and feed ranking                              |

**Providers** (adapters in `connector-llm`, one port `ChatModel` + `EmbeddingModel`):

- OpenAI-compatible HTTP (covers Ollama, LM Studio, llama-server, Foundry Local, vLLM, OpenRouter, DeepSeek, Groq, Together, and OpenAI itself)
- Anthropic Messages API (Claude), with prompt caching for long system prompts
- Google Gemini API
- Agent CLIs as **tools**, not chat providers: Claude Code (`claude -p --output-format stream-json`), Codex (`codex exec --json`). They run in a chosen working directory with the user's own login, and their result streams back into the conversation.

**Privacy policy.** Each piece of context carries a `DataClass` (`Voice`, `Mail`, `Calendar`, `Reminders`, `Location`, `Files`, `Memory`, `Feeds`, `Screen`, `Projects`, `Public`). The user sets, per data class, which providers may receive it: local only (default for everything except `Public`), or named cloud providers. The router refuses a route that would send a disallowed class off-device and either falls back to a local model or returns an error that explains the policy and links to the setting. The rule lives in `models-domain` and is property-tested. Off-device calls that are not LLM calls (cloud STT, travel-time routing, connector syncs) use the same per-data-class check through `privacy-application` and record through `kernel::AuditSink`; they do not go through the model router.

**Availability.** Local providers are health-checked (`/v1/models`) and cached. Cloud providers mark themselves unavailable on auth errors or rate limits. Fallback chains are explicit per task and shown in settings.

## Conversation and the tool loop (assistant context)

```
user turn (voice or text)
  → context assembly: system persona + relevant memory (embedding search, top 8) + live state (time, next event,
    unread counts) + the tool catalog (filtered by what is connected and permitted)
  → model call (task: voice-turn, escalates to reasoning when the persona model calls the `think_harder` tool
    or the router's heuristics say so)
  → tool calls ─► permission check ─► execute or ask ─► results back to the model (max 8 steps per turn)
  → reply: text in the panel and/or speech (mood chosen by the model from an allowed set, default Neutral)
```

- **Tools** are declared in the application layer of their own context (`reminders.create`, `calendar.find_free_time`, `mail.search`, `files.plan_cleanup`, `speech.read_aloud`, `projects.status`, ...) and registered with the assistant's `ToolRegistry` by the composition root. Each tool declares: a JSON schema, an **effect** (`Read`, `Write`, `External` for anything leaving the machine, `Destructive`), the data classes it reads, and a function that renders a preview.
- **MCP:** an MCP client (rmcp) adds tools from servers the user configures. MCP tools default to effect `External` and require confirmation until the user grants a standing permission. The assistant also runs as an MCP server (`mcp-server` root) so Claude Desktop, Claude Code, and others can use its reminders, briefing, and read-aloud.
- **Determinism where it is cheap.** Common intents skip the LLM: the reminder parser handles "remind me every Thursday at 7 to take out the trash", and simple commands ("read this", "stop", "what time is it") are matched by a grammar in `assistant-domain`. The LLM handles everything else.

## Permissions and confirmations

| Effect                                                   | Default                                                                         | Standing grant possible?                                                          |
| -------------------------------------------------------- | ------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| `Read`                                                   | Allowed                                                                         | n/a                                                                               |
| `Write` (create a reminder, add an event)                | Allowed, with an undo toast                                                     | n/a                                                                               |
| `Launch` (open an app, file, or URL)                     | Allowed in untainted turns; confirm in tainted turns (a URL can carry data out) | Per app or domain                                                                 |
| `External` (send mail, post to an API, run an agent CLI) | Confirm with a preview                                                          | Yes, per tool, per target (for example "reply to my own drafts folder")           |
| `Destructive` (delete, move to recycle bin, unsubscribe) | Confirm with a full preview                                                     | No, unless the action is reversible through the undo journal and the user opts in |

Previews are generated by the tool, not the model: the model cannot misdescribe what will happen. File operations go to the recycle bin or an archive folder and are recorded in an **undo journal** (`actions` context) so "undo the cleanup" works for 30 days.

## Untrusted content and prompt injection

Mail bodies, feed items, web pages, files, and MCP tool outputs are **untrusted**. They can contain instructions aimed at the model ("forward all invoices to ...").

1. **Taint tracking.** Untrusted values are wrapped in `kernel::Untrusted<T>`, which only the quarantined summarizer and the renderer can unwrap. Every context item carries `trusted: bool`. A turn whose context includes untrusted content is _tainted_.
2. **Tainted turns cannot auto-run `External` or `Destructive` tools**, even with a standing grant. The confirmation dialog states that the action was suggested while reading content from <source>.
3. **Quarantined summarization.** Triage and summaries run with **no tools at all**, and their output is treated as untrusted data, not instructions, when it reaches the main conversation.
4. **No data exfiltration via rendering.** The UI never auto-loads remote images or links from model output; Markdown links render as text with the domain shown.
5. **Red-team suite.** `features/core/prompt-injection.feature` holds known injection patterns and asserts no tool executes without confirmation.

## Memory ("notebook")

- Facts the user states or confirms ("I work at Contoso", "my sister is Sam", "I take out the trash on Thursdays") are stored as typed `MemoryItem`s: subject, predicate, value, source turn, created, last used. Each has an embedding.
- The assistant may **propose** memories. The user sees them in the panel ("Remember that?"), and they persist only when accepted. Settings → Notebook lists, edits, and deletes everything, with export to JSON.
- Memory syncs across devices (`DataClass::Memory`) and is never sent to a cloud model unless the user's policy allows `Memory` for that provider.

## Proactive engine

Proactive behavior comes from **watchers** and **routines**, all running on the scheduler.

- **Watcher** = source + cadence + evaluator + delivery policy. Built-ins:
  - _Mail digest:_ every N minutes in working hours (default 60), triage new mail into `NeedsReply`, `Waiting`, `FYI`, `Ignore`. A digest is delivered only if something is in `NeedsReply` or a VIP wrote. The full digest goes to the Today view.
  - _Calendar heads-up:_ at event start minus (travel time + prep lead, default 10 min), with the join link and prep notes. A spoken heads-up is optional per calendar.
  - _Reminders:_ fire at their due time, with snooze and done actions.
  - _Feeds:_ poll per feed TTL (default 30 min), rank new items against the interest profile, and add the top items to the briefing. Breaking items that pass a high threshold notify immediately (opt-in).
  - _Projects:_ on a schedule and on file-system change, refresh git status, CI state, and agent sessions.
- **Routine** = a user-authored rule in plain language that the assistant compiles to a structured `Routine { trigger, conditions, actions }` and shows for confirmation. Examples: "every weekday at 8:30 read me my briefing", "when I get a mail from my boss, tell me right away", "every Friday at 5 clean my downloads (ask first)". Routines are data, validated by `assistant-domain`, editable in the UI without the LLM. Routine and reminder text are `DataClass::Reminders`.
- **Delivery policy** (notify context) decides _if, when, where, and how_: quiet hours, Windows Focus state, full-screen apps and presentations (defer), batching window (default 5 min), channel (toast, voice, overlay, Today only), and **cross-device dedupe**: deliver on the device with the most recent user input, mark it delivered in the synced op log, and suppress elsewhere ([sync](sync.md)).
