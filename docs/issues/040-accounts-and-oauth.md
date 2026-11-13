# 040. Accounts: OAuth loopback + PKCE, credential vault, accounts UI

| Milestone            | Size | Depends on | Labels                                       |
| -------------------- | ---- | ---------- | -------------------------------------------- |
| M4 Calendar and mail | L    | #006, #008 | area/connectors, area/security, type/feature |

## Story

As a knowledge worker, I want to connect my Microsoft, Google, iCloud, and IMAP accounts once, with my passwords and tokens encrypted on my machine, so that mail, calendar, and reminders work without handing my credentials to anyone else.

## Context

Connectors authenticate with OAuth 2.0 for installed apps per RFC 8252: system browser, loopback redirect on `127.0.0.1:<random port>`, PKCE, and no client secret in the binary. Otherwise they use passwords and app passwords. Credentials live in the encrypted store, and OAuth tokens are device-local by default, with opt-in sync per account ([connectors: common rules](../architecture/connectors.md#common-rules)). Secrets are additionally sealed with a per-secret key derived from the store key ([security: storage encryption](../architecture/security.md#storage-encryption)).

## Scope

- **Accounts context** ([overview](../architecture/overview.md#bounded-contexts-and-crates)):
  - `Account { id, kind, display_name, address, capabilities: {mail, calendar, tasks}, endpoints, status }`.
  - `kind` is one of `Microsoft`, `Google`, `ICloud`, `Imap`, `Pop3`, `ProtonBridge`, `CalDav`, `IcsUrl`.
  - `status` is one of `Connected`, `NeedsSignIn`, `Error(provider_message)`, `Disabled`.
- **OAuth flow:**
  - Open the system browser with an authorization URL carrying PKCE (S256), `state`, and the scopes for the enabled capabilities only.
  - Listen once on `127.0.0.1` on a random port, for a 5-minute timeout.
  - Check `state`, exchange the code, and store the refresh and access tokens.
  - Show a "You can close this tab" page served from the listener.
- **Providers:**
  - Microsoft identity platform: the `common` authority, for personal and work accounts, using the project's public-client registration.
  - Google: a desktop OAuth client. Google issues a non-confidential "client secret" for desktop clients; shipping it is accepted practice ([connectors: common rules](../architecture/connectors.md#common-rules)). Verify the token endpoint still requires it.
  - "Bring your own client ID" for both, per [connectors: Gmail risk](../architecture/connectors.md#mail).
- **Token service:** `access_token(account, scope)` refreshes when the token is near expiry and serializes concurrent refreshes per account. On `invalid_grant` it sets `NeedsSignIn` and publishes `AccountNeedsSignIn`.
- **Credential vault:** `CredentialVault::{put, get, delete}` for refresh tokens, access tokens, passwords, app passwords, and API keys (#047 reuses it). Each secret is sealed with an HKDF-derived per-secret key. Secrets never cross IPC: the UI sees `has_secret: bool` only.
- **Sync:** account definitions (kind, address, capabilities, endpoints) sync. Secrets do not, unless "Sync this account's sign-in to my other devices" is on. Another device shows a synced account as "Sign in on this device", or as "Polled by <device>" once #046 lands.
- **Accounts UI** (`src/accounts/`):
  - An add-account wizard with tiles: Microsoft, Google, iCloud, Proton (Bridge), IMAP/SMTP, POP3, CalDAV, and ICS URL.
  - A per-account page: capability toggles, status with the provider's message and a fix action, last sync, and Remove.
  - Remove deletes secrets and the account's cached data, and revokes the Google token through the revoke endpoint (Microsoft: link to account settings).
- iCloud guidance: a step-by-step panel to create an app-specific password at account.apple.com.

Out of scope: the protocols themselves (#036–#038, #041–#044), connector leases (#046), and the off-device audit log UI (#074).

## Acceptance criteria

```gherkin
Feature: Accounts and OAuth

  Scenario: Microsoft sign-in with PKCE on a loopback port
    When I add a Microsoft account with calendar enabled
    Then the system browser opens an authorize URL with code_challenge_method "S256" and a random state
    And the redirect URI is "http://127.0.0.1:<port>/" with a port that was free
    When the browser returns the code with the same state
    Then the account status is Connected
    And the refresh token is stored sealed in the vault

  Scenario: State mismatch is rejected
    When the loopback listener receives a code with a different state
    Then no token exchange happens and the wizard shows "Sign-in was not completed"

  Scenario: User abandons the browser
    When no redirect arrives within 5 minutes
    Then the listener closes and the wizard offers "Try again"

  Scenario: Refresh token revoked
    Given a Connected Google account
    When the token endpoint answers invalid_grant
    Then the account status is NeedsSignIn
    And Settings → Accounts shows "Sign in again" for it

  Scenario: Secrets never reach the webview
    Given an IMAP account with an app password
    When the UI requests the account details
    Then the response contains has_secret true and no password field

  Scenario: Synced account without synced credentials
    Given an account added on device "desktop" with credential sync off
    When device "laptop" receives it through sync
    Then "laptop" lists the account as "Sign in on this device"

  Scenario: Remove account
    When I remove a Google account
    Then its secrets and cached mail, calendar, and tasks are deleted
    And the token revoke endpoint was called
```

## Design notes

- **Accounts context:** `accounts-domain`, `accounts-application` (ports: `CredentialVault`, `OAuthBrowser`, `LoopbackListener`, `TokenEndpoint`), and `accounts-infrastructure` (vault on `store`, a loopback listener on `127.0.0.1` only, and the token client on `reqwest` + rustls). Connector crates get tokens through an `accounts-application` port. Opening the browser goes through a `platform-ports` `Launcher` (ShellExecute in `platform-windows`).
- **arch.toml:** `accounts-domain` allows `kernel`. `accounts-application` allows `accounts-domain` and `kernel`. `accounts-infrastructure` allows both plus `store`. Every `connector-*` that needs tokens adds `accounts-application`.
- **Verify:** whether Google's token endpoint still requires the desktop client's non-confidential secret with PKCE (if so, ship it in config, never a confidential one), and whether Microsoft accepts `http://127.0.0.1` with any port for public clients, or only `http://localhost`. Record the results in connectors.md.
- **Migrations:** `accounts` (synced) and `vault_secrets` (`account_id`, `kind`, `nonce`, `ciphertext`; device-local unless credential sync is on).
- **Security:** tokens and passwords are never logged (redaction tests). The loopback listener accepts one request and checks the `Host` header. Use `secrecy` and `zeroize` for secrets in memory. Changes here trigger the security.md review (CODEOWNERS).
- **Privacy:** sign-in sends the account address to the provider, which is expected. No data class applies to credentials, and the vault is never exported in plain text.

## QA procedure

Accounts: QA-MSA (Microsoft personal), QA-M365 (work tenant), QA-GOOG (Google), QA-ICLOUD (Apple ID with an app-specific password), QA-IMAP (any IMAP host).

1. Settings → Accounts → Add → Microsoft with QA-MSA, calendar only. Observe Edge or your default browser opening, consent showing only calendar permissions, and "You can close this tab". Observe the account Connected.
2. Repeat with QA-GOOG. Observe the scopes requested match the toggles.
3. Add QA-ICLOUD with an app-specific password. Observe the guidance panel and Connected.
4. Open `%APPDATA%\<app>\profile.db` in a hex viewer. Observe no plain-text password or token (the file is SQLCipher-encrypted).
5. Revoke the app at myaccount.google.com. Trigger a sync. Observe "Sign in again".
6. Start adding QA-MSA and close the browser tab. Wait 5 minutes. Observe "Try again".
7. Remove QA-GOOG. Observe that its data disappears from calendar and tasks views.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed (connectors.md auth notes; security.md reviewed)
