# 003. Replace the starter example with reminders, local-api, and headless skeletons

| Milestone      | Size | Depends on | Labels                        |
| -------------- | ---- | ---------- | ----------------------------- |
| M0 Foundations | M    | #002       | area/foundation, type/feature |

## Story

As a contributor, I want the starter's `tasks` example replaced by real product crates, so that every gate exercises product code and the QA harness boots the real core.

## Context

[Overview: bounded contexts](../architecture/overview.md#bounded-contexts-and-crates) says the starter's `tasks-*`, `http-api`, and `app` crates stay until this issue replaces them with `reminders-*`, `local-api`, and `headless`. `headless` is the always-on core with no UI: a sync peer, a connector poller, and the target of `make qa-rs` and `features/core/`.

## Scope

- Rename and reshape the crates:
  - `tasks-domain` → `reminders-domain`: `Reminder { id, title, due: Option<Timestamp>, done }`, `ReminderTitle::parse` (trimmed, 1–200 characters), `complete()` idempotent. Uses kernel `Id`, `Timestamp`.
  - `tasks-application` → `reminders-application`: `ReminderService` (create, list, complete) with `ReminderRepository`, `kernel::IdSource`, and `kernel::Clock` ports.
  - `tasks-infrastructure` → `reminders-infrastructure`: in-memory repository and a random `IdSource`. SQLCipher persistence arrives with #032 on top of #006.
  - `http-api` → `local-api`: axum router on loopback only. `GET /health` (no auth) and `/v1/reminders` (bearer token).
  - `app` → `headless`: composition root and binary. Reads `HOST`, `PORT`, and `LOCAL_API_TOKEN`.
- Move `features/core/tasks.feature` to `features/core/reminders.feature`, `qa/scripts/core/tasks.sh` to `reminders.sh`, and `qa/procedures/tasks-api.md` to `reminders-api.md`.
- Update the Makefile (`acceptance-rs` targets `headless`), `scripts/qa-rs.sh`, `.cargo/mutants.toml`, `COVERAGE_IGNORE`, and `arch.toml`. All are protected files, so the PR needs human approval.
- Make `qa-rs.sh` work on Windows: stop `headless` through an authenticated `POST /v1/shutdown` instead of SIGTERM, and keep SIGTERM on Unix.

Out of scope: natural-language parsing and recurrence (#032), the reminders UI (#039), the starter UI example in `src/tasks` (removed by #004).

## Acceptance criteria

`features/core/reminders.feature`, run in process against `headless`:

```gherkin
Feature: Reminders over the local API

  Scenario: Create and list a reminder
    Given headless is running with a valid API token
    When I POST "/v1/reminders" with title "Take out the trash"
    Then the response is 201 with an id and done = false
    And GET "/v1/reminders" lists it

  Scenario: Reject a blank title
    When I POST "/v1/reminders" with title "   "
    Then the response is 422 and names the title rule

  Scenario: Completing twice is a no-op
    Given a reminder that is already done
    When I complete it again
    Then the response is 200 and done is still true

  Scenario: Requests without the token are refused
    When I GET "/v1/reminders" without an Authorization header
    Then the response is 401
    And GET "/health" still answers 200

  Scenario: headless refuses a non-loopback bind
    When headless starts with HOST=0.0.0.0
    Then it exits non-zero with "local-api binds to loopback only"

  Scenario: headless refuses to start without a token
    When headless starts with no LOCAL_API_TOKEN
    Then it exits non-zero and says how to set one
```

## Design notes

- `arch.toml` replaces the `tasks-*`, `http-api`, and `app` entries:
  ```toml
  [crates.reminders-domain]
  allow = ["kernel"]
  ban = ["tokio", "reqwest", "rusqlite", "serde_json", "tauri", "windows", "uuid", "axum"]

  [crates.reminders-application]
  allow = ["reminders-domain", "kernel"]
  ban = ["tokio", "reqwest", "rusqlite", "serde_json", "tauri", "windows", "uuid", "axum"]

  [crates.reminders-infrastructure]
  allow = ["reminders-application", "reminders-domain", "kernel"]
  ban = ["tauri", "windows", "axum"]

  [crates.local-api]
  allow = ["reminders-application", "reminders-domain", "kernel"]
  ban = ["rusqlite", "reqwest", "tauri", "windows"]

  [crates.headless]
  allow = ["local-api", "reminders-application", "reminders-domain", "reminders-infrastructure", "kernel"]
  ```
  `no_dependents` becomes `["headless", "desktop", "xtask"]`.
- Token check: constant-time comparison. The token is never logged. `PORT` defaults to 3000 as in the starter; QA passes a free port from `cargo xtask free-port`.
- `local-api` is a QA and diagnostics surface, not a product API. It never binds a non-loopback address, and the desktop app does not start it.
- Data class: reminder titles are user content. No new off-device path is added.

## QA procedure

1. In Git Bash, run `LOCAL_API_TOKEN=dev PORT=3999 cargo run -p headless`. The log says it listens on `127.0.0.1:3999`.
2. Run `curl -s localhost:3999/health`. It prints an OK body.
3. Run `curl -s -H "Authorization: Bearer dev" -d '{"title":"Trash"}' -H "Content-Type: application/json" localhost:3999/v1/reminders`. It returns the new reminder.
4. Repeat without the header. It returns 401.
5. Start with `HOST=0.0.0.0`. It refuses to start.
6. Run `make qa-rs` on Windows. It boots `headless`, runs the scripts, and shuts it down through `/v1/shutdown` with no leftover process in Task Manager.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed (`rust-gates.md` layout section)
