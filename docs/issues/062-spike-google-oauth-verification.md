# 062. Spike: Google OAuth verification for Gmail scopes

| Time box | Needed by | Labels                                 |
| -------- | --------- | -------------------------------------- |
| 2 days   | #044      | area/connectors, area/mail, type/spike |

## Question

Can an open-source, local-only desktop app read Gmail through the Gmail API (or IMAP with XOAUTH2) for any user, and at what cost? Specifically:

1. Which Gmail scopes does triage need (read headers and the first 8 KB of text, archive, later draft and send), and which of them are restricted versus sensitive today?
2. What does Google require for a shared OAuth client to use restricted scopes: brand verification, restricted-scope verification, and an annual third-party security assessment? Does any exemption apply to an app that stores and processes data only on the user's device, with no server of ours?
3. What does "bring your own OAuth client" look like for a single user: project setup, consent screen in Testing versus Production, the test-user cap, and refresh-token lifetime in Testing mode?
4. Is IMAP with an app password still available for personal Gmail and for Workspace accounts, and under what conditions (2-Step Verification, admin policy)?

## Why it matters

Mail triage (#045) is a core MVP feature, and Gmail is the largest provider. [connectors](../architecture/connectors.md#mail) marks Gmail read scopes as restricted and the security-assessment requirement as unconfirmed for a local-only app. #044 ships IMAP + app password in the MVP and keeps the API behind this spike. Calendar (`calendar.readonly`) and Tasks are believed to be sensitive, not restricted, scopes; confirm that too, because #041 and #037 assume it.

## Options to evaluate

| Option                             | Known (to verify)                                                                                              | Must verify                                                                                                                                                                                                                                                                                     |
| ---------------------------------- | -------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Project-verified shared client** | Restricted scopes need Google's verification; the assessment is performed by an approved assessor and recurs   | Whether a local-only data flow changes the assessment requirement; cost and timeline; what the review asks of an open-source app whose client id is public; how the "no client secret in the binary" rule ([RFC 8252](https://www.rfc-editor.org/rfc/rfc8252)) fits Google's installed-app flow |
| **Bring your own OAuth client**    | The user creates a Google Cloud project and a desktop client and pastes the client id into Settings → Accounts | Test-user limits and refresh-token expiry in Testing mode (reported as 7 days; confirm); whether a personal project can be published without verification for its own owner; the "unverified app" warning flow                                                                                  |
| **IMAP + app password**            | Works for personal accounts with 2-Step Verification                                                           | Current Google policy for app passwords; Workspace admin controls that block them; whether Google has announced deprecations                                                                                                                                                                    |
| **IMAP + XOAUTH2**                 | Uses the full mail scope, which is restricted                                                                  | Same verification burden as the API, so likely no advantage                                                                                                                                                                                                                                     |

## Method

1. Read Google's current OAuth policies, the restricted-scope verification FAQ, and the security-assessment pages; save dated copies (PDF) under `docs/research/google-oauth/` in the ADR's PR. Quote, do not paraphrase, the parts that decide the answer.
2. Create a test Google Cloud project with a desktop OAuth client. Run the loopback + PKCE flow from #040 against `gmail.readonly` in Testing mode with one test user; record the consent screens, warnings, and token lifetime over at least two days of the time box (note if the full expiry window cannot be observed).
3. Write a minimal `historyId` incremental fetch with the token to confirm the API shape #044 would need.
4. Confirm app-password IMAP on a fresh personal account with 2-Step Verification, and on a Workspace test tenant with default policy.
5. If possible, ask Google's verification support whether a local-only open-source client still needs the assessment, and attach the reply.

## Output

- A new ADR (next free number), "Gmail access", stating which paths the product supports, the scopes requested, and the evidence (dated policy copies and test results). It marks unverifiable claims as such.
- Changes to [#044](044-mail-gmail.md): whether the API path ships in v0.1, ships as bring-your-own-client only, or waits for verification; and the in-app guide text for app passwords and for creating a client.
- Changes to [#040](040-accounts-and-oauth.md) if the client-id handling changes (user-supplied client ids per provider).
- Confirmation (or correction) of the scope classes assumed by [#041](041-calendar.md) and [#037](037-connector-google-tasks.md).
