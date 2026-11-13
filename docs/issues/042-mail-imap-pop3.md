# 042. Mail: IMAP/SMTP (iCloud, Proton Bridge, generic) and POP3

| Milestone            | Size | Depends on | Labels                                   |
| -------------------- | ---- | ---------- | ---------------------------------------- |
| M4 Calendar and mail | L    | #040       | area/mail, area/connectors, type/feature |

## Story

As a privacy-conscious user with iCloud, Proton, or my own mail server, I want the assistant to read my inbox over IMAP or POP3 without uploading it anywhere, so that it can tell me which emails need me.

## Context

MVP done-criterion 5 includes "any IMAP account (including Proton through Bridge)" ([mvp-plan](../product/mvp-plan.md#what-done-means-for-the-mvp)). `connector-imap` implements `mail-application::MailSource` ([connectors](../architecture/connectors.md)). The rules: incremental sync with `UIDVALIDITY` + `UIDNEXT` (+ `CONDSTORE`), POP3 by UIDL set, headers plus the first 8 KB of text for triage, full bodies on demand, never attachments, and backoff with `Retry-After` ([connectors: common rules](../architecture/connectors.md#common-rules)). POP3 is a Should and is shown as a limited account.

## Scope

- `mail-domain`:
  - `Envelope { message_id, account, folder, uid, from, to, cc, subject, date, flags, list_unsubscribe, in_reply_to, references, snippet: Untrusted<String> }`.
  - Threading by `References` / `In-Reply-To`.
- `mail-application`: the ports `MailSource` (list new, fetch snippet, fetch body, set flags, move) and `MailSender`, plus a sync use case that runs on the scheduler at the account cadence (default 5 min).
- **IMAP** in `connector-imap`:
  - TLS on 993, or STARTTLS on 143 when required. `LOGIN` with a password or app password; `AUTHENTICATE XOAUTH2` where the account has OAuth (#044 may use it).
  - Sync INBOX plus user-chosen folders.
  - Track `UIDVALIDITY` and `UIDNEXT`, and use `CONDSTORE` / `HIGHESTMODSEQ` for flag changes when advertised. A `UIDVALIDITY` change means a resync of that folder.
  - Fetch with `BODY.PEEK` so nothing is marked read.
  - Snippet: the first 8 KB of the first `text/plain` part. If there is only `text/html`, convert it to text locally.
- **POP3** in `connector-imap`:
  - TLS on 995. `UIDL` to find new messages, `TOP n 0` plus the first lines for the snippet.
  - Never `DELE`. Read and archived state is local only, and the account UI says so.
- **SMTP send** (`MailSender`): submission on 587 STARTTLS or 465 TLS, used only by confirmed `External` tools. The mail tools are #093.
- **Presets:**
  - iCloud: `imap.mail.me.com:993`, `smtp.mail.me.com:587`, app-specific password.
  - Proton Mail Bridge: `127.0.0.1`, using the IMAP and SMTP ports Bridge shows (verify the defaults). STARTTLS with Bridge's self-signed certificate, pinned on first connect after the user confirms its fingerprint.
  - Fastmail, Yahoo, and generic.
- **Proton detection:** a refused connection to the Bridge port reports "Proton Mail Bridge isn't running", with a link explaining that Bridge needs a paid Proton plan.
- **Errors:**
  - Auth failure → "Password rejected" plus provider guidance (iCloud: app-specific password).
  - TLS failure → refuse; there is no "accept invalid cert" switch, except the Bridge pin.
  - Throttling (`[LIMIT]`, server bye) → backoff.
- **Retention:** snippets are kept 30 days for triage and Today. Full bodies are fetched on demand and not cached beyond the session.

Out of scope: Graph mail (#043), Gmail specifics (#044), triage and digests (#045), IMAP IDLE (post-MVP), JMAP (Could), and attachments.

## Acceptance criteria

```gherkin
Feature: IMAP and POP3 mail sources

  Scenario: Incremental IMAP sync fetches only new mail without marking it read
    Given an IMAP account whose INBOX has UIDNEXT 120 from the last sync
    And two new messages arrived
    When the sync runs
    Then envelopes and snippets for UIDs 120 and 121 are stored
    And both messages are still unread on the server

  Scenario: UIDVALIDITY change triggers a folder resync
    Given the stored UIDVALIDITY for INBOX is 7
    When the server reports UIDVALIDITY 8
    Then the folder cache is rebuilt and no duplicates remain

  Scenario: Attachments are never fetched for triage
    Given a new message with a 20 MB PDF attachment
    When the sync runs
    Then at most 8 KB of body text is downloaded for that message

  Scenario: Proton Bridge not running
    Given a Proton Bridge account
    And nothing listens on the Bridge IMAP port
    When the sync runs
    Then Settings → Accounts shows "Proton Mail Bridge isn't running"

  Scenario: Bridge certificate changes after pinning
    Given the Bridge certificate fingerprint was pinned
    When Bridge presents a different certificate
    Then the connection is refused and the user is asked to confirm the new fingerprint

  Scenario: POP3 never deletes server mail
    Given a POP3 account with 3 messages on the server
    When the sync runs twice
    Then 3 envelopes exist locally and no DELE command was sent

  Scenario: iCloud rejects a normal Apple ID password
    Given an iCloud account configured with a non-app-specific password
    When the sync runs
    Then the error explains that iCloud needs an app-specific password, with a link

  Scenario: Offline sync is skipped quietly
    Given the network is disconnected
    When the sync runs
    Then no error notification is shown and the account shows "Offline, last synced at <time>"
```

## Design notes

- **Crates:** create `mail-domain` and `mail-application`, and build `connector-imap` with an async IMAP client (evaluate `async-imap`), `lettre` for SMTP, and a minimal POP3 client (a small command set, written here). TLS is rustls only, with no OpenSSL. MIME parsing via `mail-parser` (verify license). HTML-to-text conversion is local, with remote resources never fetched.
- **arch.toml:** `mail-domain` allows `kernel`. `mail-application` allows `mail-domain` and `kernel`. `connector-imap` allows `mail-application`, `mail-domain`, `accounts-application`, `kernel`, and `store`.
- **Taint:** `kernel::Untrusted<T>` marks every field that comes from the message body or display names. It is what ADR 0009's taint tracking keys on downstream (#045, #049).
- **Migrations:** `mail_accounts_state` (per-folder `uidvalidity`, `uidnext`, `modseq`; POP3 UIDL set) and `mail_envelopes` (snippets; 30-day retention job). Envelopes are device-local. Triage results sync (#045).
- **Privacy:** `DataClass::Mail`. The account host is the user's chosen provider. No mail content goes to any other endpoint.
- **Security:** MIME and HTML parsing take hostile input, so add fuzz targets. Size caps: 8 KB for the snippet and 25 MB for an on-demand body.
- **Testing:** a Dovecot container in CI for IMAP and POP3, plus a fake SMTP sink. The Proton and iCloud steps are manual.

## QA procedure

Accounts: QA-ICLOUD (app-specific password), QA-PROTON (paid plan, Bridge installed), QA-IMAP (any generic host), QA-POP3 (a host offering POP3).

1. Add QA-ICLOUD with an app-specific password. From another device, send it two messages. After one sync, observe both in the mail list, still unread in iCloud Mail on the web.
2. Quit Proton Mail Bridge. Add QA-PROTON. Observe "Proton Mail Bridge isn't running". Start Bridge, accept the certificate fingerprint shown (compare it with Bridge's settings), and observe the sync succeeding.
3. Send QA-IMAP a message with a large attachment. Check the network usage in Task Manager or Resource Monitor during sync: the traffic must be far smaller than the attachment.
4. Add QA-POP3. Observe the "limited account" note, and after two syncs, that the messages are still on the server.
5. Disconnect the network. Observe "Offline" and no error toast.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
