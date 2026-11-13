# 0009. Tool permissions and untrusted content

Status: proposed

## Decision

- Every tool declares an effect: `Read`, `Write`, `Launch`, `External`, or `Destructive`. Read and Write run immediately; Write shows an undo. `Launch` (open an app, file, or URL) runs immediately only in untainted turns, because a URL can carry data out. External and Destructive need confirmation with a preview the tool generates.
- Content from mail, feeds, web pages, files, and MCP tools is **untrusted**. A turn that includes untrusted content is _tainted_. A tainted turn cannot auto-run External or Destructive tools, even with a standing grant.
- Summarization and triage of untrusted content run with no tools.
- Destructive file operations are reversible through an undo journal (recycle bin or archive folder).

Details: [assistant](../assistant.md).

## Consequences

- More confirmations when acting on mail. That friction is intended.
- A prompt-injection acceptance suite (`features/core/prompt-injection.feature`) is part of `make check`.
