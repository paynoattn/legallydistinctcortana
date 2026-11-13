# 044. Mail: Gmail via IMAP app password, API behind spike

| Milestone            | Size | Depends on | Labels                                   |
| -------------------- | ---- | ---------- | ---------------------------------------- |
| M4 Calendar and mail | S    | #042, #062 | area/mail, area/connectors, type/feature |

## Story

As a Gmail user, I want to connect Gmail today with an app password, so that I get mail digests without waiting for Google to verify the project.

## Context

Gmail read scopes are restricted, so a shared OAuth client needs Google's verification and possibly a security assessment. The MVP ships IMAP + app password, plus a "bring your own OAuth client" guide, and spike #062 decides the API path ([connectors: mail](../architecture/connectors.md#mail), [mvp-plan risks](../product/mvp-plan.md#risks)). The Gmail API is a Should, pending #062.

## Scope

- A Gmail preset on top of `connector-imap` (#042): `imap.gmail.com:993` and `smtp.gmail.com:465` or `587`, with the address as the username and an app password.
- **Guided setup:** explain that app passwords need 2-Step Verification, link to the Google account's app-password page, and say that Workspace admins may disable app passwords. Verify the current Google policy and wording when building this.
- **Gmail IMAP specifics:**
  - Sync `INBOX` only by default, never `[Gmail]/All Mail` (it duplicates everything).
  - If the server advertises `X-GM-EXT-1`, use `X-GM-MSGID` for dedupe and `X-GM-THRID` for threading.
  - Labels map to folders as Gmail exposes them. Archive is a move out of INBOX.
- **XOAUTH2:** if #062 recommends it, add XOAUTH2 on IMAP with the user's own OAuth client (scope `https://mail.google.com/`), behind a "Use my own Google OAuth client" option that reuses #040's flow.
- **Gmail API** (`historyId` sync): behind a feature flag only if #062 says the verification path is viable. Otherwise it stays out of scope here and a follow-up issue is created from the spike's ADR.

Out of scope: triage (#045) and Google verification work itself (#062).

## Acceptance criteria

```gherkin
Feature: Gmail via IMAP

  Scenario: Gmail preset with an app password
    Given a Gmail address and a valid app password
    When I add the account with the Gmail preset
    Then the account connects to imap.gmail.com on port 993 with TLS
    And INBOX is synced and "[Gmail]/All Mail" is not

  Scenario: Normal Google password is rejected with guidance
    Given a Gmail account configured with the regular Google password
    When the sync runs and the server rejects the login
    Then the error explains that Gmail needs an app password and 2-Step Verification, with a link

  Scenario: Gmail extensions dedupe a message with two labels
    Given a message labelled both INBOX and "Work", and "Work" is also synced
    When the sync runs
    Then one envelope exists for that X-GM-MSGID

  Scenario: Workspace admin disabled app passwords
    Given the server answers that application-specific passwords are not allowed for the account
    When the sync runs
    Then the error says the Workspace admin has disabled app passwords and suggests "Use my own Google OAuth client"
```

## Design notes

- **Crates:** `connector-imap` gets a `gmail` preset module and `X-GM-EXT-1` handling. `connector-google` is touched only if XOAUTH2 or the API ships, depending on #062.
- **arch.toml:** no new crates. If the API ships, `connector-google` adds `mail-application` and `mail-domain`.
- **Verify:** Google's current rules for app passwords (2SV requirement, Workspace controls, any phase-out notices), and the exact server error text, so the error mapping matches by response code rather than English text where possible.
- **Privacy:** `DataClass::Mail`. Google is the mailbox provider.
- **Testing:** the Dovecot container with a fake `X-GM-EXT-1` capability is not realistic, so use recorded IMAP transcripts for the Gmail-specific responses. Live checks are manual.

## QA procedure

Accounts: QA-GOOG (a personal Gmail with 2-Step Verification), and optionally QA-GWS (a Google Workspace account with app passwords disabled by policy).

1. On QA-GOOG, create an app password in the Google account's security settings.
2. Settings → Accounts → Add → Gmail. Observe the guidance panel. Paste the app password. Observe Connected.
3. Send QA-GOOG two messages. After one sync, observe them in the mail list and still unread in Gmail.
4. Apply a second label to one message and enable that label's folder. Observe no duplicate in the list.
5. Enter the normal Google password instead of the app password. Observe the app-password guidance error.
6. With QA-GWS, observe the "admin has disabled app passwords" message.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed (connectors.md Gmail row reflects the #062 outcome)
