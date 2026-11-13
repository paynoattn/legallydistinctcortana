# 074. Privacy dashboard and off-device audit log

| Milestone  | Size | Depends on | Labels                                    |
| ---------- | ---- | ---------- | ----------------------------------------- |
| M8 Release | M    | #047       | area/privacy, area/security, type/feature |

## Story

As a privacy-conscious user, I want one page that shows, per kind of data, where it is allowed to go and exactly what was sent off my devices in the last 30 days, with a single switch to keep everything local, so that I can trust the assistant without reading its code.

## Context

MVP done-criterion 10. Promises 1 and 3 in [security: promises](../architecture/security.md#promises-to-the-user) and the dashboard in [security: privacy UX](../architecture/security.md#privacy-ux). The per-data-class policy is enforced by the router ([assistant: privacy policy](../architecture/assistant.md#model-router-models-context), [#047](047-model-router.md)).

## Scope

- **Audit log:** one entry per off-device call: time, destination (provider and host), purpose (router task or connector operation), data classes, bytes sent and received, outcome (sent, failed, blocked by policy). Never content, prompts, subjects, or addresses. Retained 30 days.
- **Sources that write entries** (all count as off-device calls, per [security: privacy UX](../architecture/security.md#privacy-ux)): the models router (cloud LLM, cloud STT and TTS), account connector syncs (mail, calendar, tasks), feeds fetches, GitHub CI calls ([#065](065-projects.md)), update checks ([#075](075-installer-signing-updater.md)), and model downloads.
- **Dashboard** (Settings → Privacy, `main` window): per data class (`Voice`, `Mail`, `Calendar`, `Reminders`, `Location`, `Files`, `Memory`, `Feeds`, `Screen`, `Projects`, `Public`), the allowed destinations and an edit link to the policy; a 30-day summary ("Nothing was sent" or counts per destination); the log as a filterable table; "Your devices" listing sync peers and backup folders separately (end-to-end encrypted, not off-device); export of the log as CSV or JSON.
- **"Local only" switch:** one click sets every data class to local only for model providers and disables cloud model routes. Turning it off restores the previous per-class policy. Account connectors keep working, because they fetch the user's own data from its provider; the page explains this.

Out of scope: per-request content inspection (never stored); network-level enforcement (a firewall); export and "delete everything" flows (settings issue).

## Acceptance criteria

```gherkin
Feature: Privacy dashboard

  Scenario: A fresh install sent nothing
    Given a fresh install with no cloud provider configured and no accounts connected
    When I open Settings → Privacy
    Then every data class shows "This device only"
    And the summary says "Nothing was sent off this device in the last 30 days"

  Scenario: A cloud call is logged without content
    Given Claude is configured and my policy allows Mail for Claude
    When the assistant summarizes an email through Claude
    Then the log shows an entry with destination "Anthropic", purpose "mail-summary", data class "Mail", and byte counts
    And no stored field of the entry contains any text from the email

  Scenario: A blocked call is shown as blocked
    Given my policy keeps Mail on this device and the reasoning task routes to a cloud model
    When I ask the assistant about an email
    Then the answer comes from a local model or explains the policy
    And the log shows one "blocked by policy" entry with zero bytes sent

  Scenario: The local-only switch
    Given cloud routes are configured for reasoning and mail-summary
    When I turn on "Local only"
    Then the next reasoning request uses a local model and no off-device model entry is logged
    When I turn "Local only" off
    Then the previous per-class policy is restored exactly

  Scenario: Account connectors are listed separately
    Given an IMAP account is connected
    When the mail poller runs
    Then the log shows an entry with destination "imap.example.com", purpose "mail fetch", data class "Mail"

  Scenario: Entries expire
    Given a log entry from 31 days ago
    When the daily cleanup runs
    Then the entry is deleted

  Scenario: Only the main window can read the log
    When the overlay or Today window calls the audit-log command
    Then the call is denied by the window's capabilities
```

## Design notes

- **Ownership** ([overview: bounded contexts](../architecture/overview.md#bounded-contexts-and-crates)): adapters report through the `AuditSink` port in `kernel` (trait only, next to `Clock`). The new `privacy` context owns the log: `privacy-domain` (audit entry, retention, summary rules, policy snapshot for the local-only switch; pure) and `privacy-application` (dashboard queries, the local-only switch, which calls the models policy through a port, and the `AuditSink` implementation over a repository port; its store adapter is wired by the composition root). Add both crates to `arch.toml` (protected: needs a human's approval): `privacy-domain` → `kernel`; `privacy-application` → `privacy-domain`, `kernel`.
- **Where entries are written:** at the adapter boundary (`connector-*`, `connector-llm`, the updater, the model downloader), so no off-device call can bypass logging; a contract test per adapter asserts one entry per request.
- **Policy rules** stay in `models-domain` (property-tested). The local-only switch stores the previous policy so it can be restored.
- **Data:** migration adds `audit_log` (device-local, not synced: each device shows its own calls; the dashboard notes that). Retention job on the scheduler.
- **UI:** `src/privacy/{domain,application,infrastructure,ui}`; table virtualized for 30 days of entries; numbers formatted with the user's locale.
- **Security:** commands available only to the `main` window capability set (`crates/desktop/capabilities/` is protected).

## QA procedure

1. Install fresh on a Windows 11 VM, finish onboarding offline, and open Settings → Privacy. Observe "This device only" for every class and "Nothing was sent".
2. Connect an IMAP account. Wait for a poll. Observe an entry for the IMAP host with data class Mail and byte counts.
3. Add a Claude API key, allow Mail for Claude, and ask the assistant to summarize an email. Observe the "Anthropic / mail-summary" entry. Export the log as JSON and search it for words from the email; expect none.
4. Change Mail to local only, ask again, and observe a "blocked by policy" entry and a local answer.
5. Turn on "Local only", ask a reasoning question, and confirm no new model entry. Turn it off and confirm the earlier policy is back.
6. Check that "Your devices" lists the paired PCs and the backup folder.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] security.md updated if the logged call list changes
