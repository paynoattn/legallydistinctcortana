# 043. Mail: Microsoft Graph

| Milestone            | Size | Depends on | Labels                                   |
| -------------------- | ---- | ---------- | ---------------------------------------- |
| M4 Calendar and mail | M    | #040       | area/mail, area/connectors, type/feature |

## Story

As a knowledge worker on Outlook.com or Microsoft 365, I want my Outlook inbox connected, so that the assistant can triage it like any other account.

## Context

Microsoft removed basic-auth IMAP for consumer accounts, so Microsoft Graph (`/me/messages`, delta) is the path for Outlook, Live, Hotmail, and M365 ([connectors: mail](../architecture/connectors.md#mail)). Polling uses delta queries, not webhooks, because Graph change notifications need a public endpoint ([connectors: common rules](../architecture/connectors.md#common-rules)). Sending is always a confirmed `External` tool.

## Scope

- In `connector-msgraph`: a `MailSource` and `MailSender` adapter.
  - **Sync:** a delta query on Inbox and the user-chosen folders (`/me/mailFolders/{id}/messages/delta`) with `$select` limited to the envelope fields (id, internetMessageId, conversationId, from, toRecipients, ccRecipients, subject, receivedDateTime, isRead, flag, bodyPreview) and the `Prefer: outlook.body-content-type="text"` header.
  - **Snippet:** `bodyPreview` is short. When triage needs more, fetch `body` as text for the message and keep the first 8 KB. Never fetch attachments.
  - **Delta token lost** (verify the Graph error for an expired delta link, for example `410 Gone` or `syncStateNotFound`): resync the folder from a 14-day window and dedupe by `internetMessageId`.
  - **Actions** for later tools: mark read, move to the well-known `archive` folder, create a draft, and send (`/me/sendMail`, or create the draft then send).
- **Scopes:** `Mail.Read` and `offline_access`. `Mail.ReadWrite` and `Mail.Send` are requested incrementally, only when the user turns on "Let the assistant draft and send".
- **Errors:**
  - Work tenant requires admin consent → a clear message naming the tenant, and that an admin must approve (verify the AADSTS error codes).
  - 401 → refresh, then "Sign in again".
  - 429 / 503 / 504 → honor `Retry-After` and back off with jitter.
  - Mailbox not found (a consumer account without an Outlook mailbox) → "This Microsoft account has no Outlook mailbox".
- Polling runs on the scheduler at the account cadence (default 5 min).

Out of scope: triage and digests (#045), the mail tools themselves (#093), shared mailboxes, and Graph change notifications.

## Acceptance criteria

```gherkin
Feature: Microsoft Graph mail

  Scenario: First sync is bounded and incremental afterwards
    Given a newly connected Outlook account with 20000 messages in Inbox
    When the first sync runs
    Then only messages from the last 14 days are stored
    And a delta link is saved
    When one new message arrives and the next sync runs
    Then exactly one new envelope is stored

  Scenario: No body or attachment download for triage beyond 8 KB
    Given a new message with a 15 MB attachment
    When the sync runs
    Then no attachment endpoint is called
    And at most 8 KB of body text is stored

  Scenario: Work tenant blocks consent
    Given a work account whose tenant requires admin consent
    When I complete sign-in
    Then the account shows "Your organization's admin must approve this app" with the tenant name

  Scenario: Delta link expired
    Given the stored delta link is rejected as expired
    When the sync runs
    Then the folder resyncs from 14 days and no envelope is duplicated

  Scenario: Throttled
    Given Graph answers 429 with Retry-After 60
    When the sync runs
    Then no Graph request is sent for 60 seconds

  Scenario: Send needs the extra scope
    Given the account was connected with Mail.Read only
    When a confirmed send is attempted
    Then the user is asked to grant "send mail" permission first and nothing is sent
```

## Design notes

- **Crates:** `connector-msgraph` (shared with #036 and #041) implements the `mail-application` ports from #042. The shared Graph HTTP client (auth header, throttling, `@odata.nextLink` paging) lives in one module of the crate.
- **arch.toml:** `connector-msgraph` adds `mail-application` and `mail-domain` to its allow list.
- **Data:** reuses `mail_envelopes`, and adds per-folder `delta_link` to `mail_accounts_state`.
- **Privacy:** `DataClass::Mail`. Microsoft is the mailbox provider, so no new exposure. Fetches are recorded in the audit log (#074) as provider "Microsoft Graph", with byte counts.
- **Security:** `bodyPreview` and the body are untrusted and wrapped in `Untrusted<T>` (see #042). Sending requires an `External` confirmation (#049).
- **Testing:** recorded Graph fixtures for delta paging, throttling, and expired delta links, run through the contract test suite shared with `connector-imap`.

## QA procedure

Accounts: QA-MSA (Outlook.com) and QA-M365 (work tenant; ideally one with admin consent required, QA-M365-LOCKED).

1. Add QA-MSA with mail enabled. Observe the consent screen listing only "Read your mail" and "Maintain access". Observe the last 14 days of Inbox after the first sync.
2. Send QA-MSA a message from another account. Within one cadence, observe it in the list, still unread in Outlook on the web.
3. Send a message with a large attachment. Observe (Resource Monitor → Network) no large download during sync.
4. Add QA-M365-LOCKED. Observe the admin-consent message.
5. Turn on "Let the assistant draft and send". Observe a second consent screen for send permission.
6. Disconnect the network for one cadence, then reconnect. Observe the sync catching up with no duplicates.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
