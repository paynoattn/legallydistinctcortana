# 065. Projects: git, CI, agent sessions

| Milestone | Size | Depends on | Labels                      |
| --------- | ---- | ---------- | --------------------------- |
| M6 Today  | M    | #031       | area/projects, type/feature |

## Story

As a developer with many repositories and several AI coding agents, I want the assistant to know the state of each project (branch, uncommitted work, CI, open pull requests, and what my last Claude Code or Codex session did), so that I can see or ask "what is the state of my projects?" without opening a terminal in each one.

## Context

The developer persona's first "wow" in the [vision](../product/vision.md#who-it-is-for). The projects context is listed in [overview](../architecture/overview.md#bounded-contexts-and-crates); data sources are in [connectors: AI apps and agents](../architecture/connectors.md#ai-apps-and-agents); the watcher cadence is in [assistant: proactive engine](../architecture/assistant.md#proactive-engine). The Today view ([#066](066-today-view.md)) renders this read model.

## Scope

- Settings → Projects: add a repository folder (folder picker), name it, remove it. Optional GitHub token for CI and pull requests.
- Per project: current branch, ahead/behind its upstream, count of uncommitted changes, last commit (subject, author, age), latest CI run for the branch (status, workflow, age, URL), count of open pull requests, and the last agent session.
- Agent sessions: read metadata (title or first prompt, last activity, status) from Claude Code and Codex local session folders, and a one-line local summary of what the session did.
- Refresh on a scheduler job (default every 5 minutes, configurable 1–60) and on changes under `.git/` (HEAD, refs, index), debounced 2 s.
- Assistant tool `projects.status` (effect `Read`) returning the read model.

Out of scope: GitLab, Azure DevOps, and other CI providers (follow-up); writing to repositories; starting agent sessions (that is [#056](056-agent-cli-tools.md)); live state of clones that exist only on another device (see design notes).

## Acceptance criteria

```gherkin
Feature: Project state

  Scenario: Add a git repository
    Given a folder "C:\src\demo" that is a git repository on branch "main" with 2 modified files
    When I add it in Settings → Projects
    Then the project "demo" shows branch "main", "2 uncommitted changes", and its last commit

  Scenario: Reject a folder that is not a repository
    When I add the folder "C:\Users\me\Pictures"
    Then I see "This folder is not a git repository" and no project is added

  Scenario: A commit refreshes the state
    Given the project "demo" is watched
    When a commit is made in "C:\src\demo"
    Then within 5 seconds the project shows the new last commit and "0 uncommitted changes"

  Scenario: CI status with a read-only GitHub token
    Given the project's remote is a GitHub repository and a fine-grained read-only token is saved
    And the latest workflow run on "main" failed
    Then the project shows "CI failed" with the workflow name and a link to the run

  Scenario: Rejected token
    Given the saved GitHub token is revoked
    When the projects watcher refreshes
    Then the project shows "CI: token rejected" and Settings → Accounts offers "Replace token"
    And git state still refreshes

  Scenario: Last agent session is summarized locally
    Given a Claude Code session for "C:\src\demo" ended 20 minutes ago
    And my privacy policy allows the Projects data class on this device only
    Then the project shows the session title, "finished 20 min ago", and a one-line summary
    And the off-device audit log has no entry for the summary

  Scenario: Unreadable session file
    Given a Codex session file for the project contains malformed lines
    Then the project still shows its git state and the session shows "Could not read the last session"
    And the log entry names the file but contains none of its content

  Scenario: Ask the assistant
    When I ask "what is the state of my projects?"
    Then the assistant calls "projects.status" without asking for confirmation
    And answers from the same data the Projects view shows
```

## Design notes

- **Crates:** `projects-domain` (Project, CloneLocation, GitState, CiState, AgentSession, staleness rules, session-status heuristics; pure), `projects-application` (use cases `AddProject`, `RefreshProject`, `ProjectsOverview`; ports `GitReader`, `CiProvider`, `AgentSessionSource`, `SessionSummarizer`, `ProjectRepository`), `projects-infrastructure` (adapters below and store tables). Add all three to `arch.toml` (protected: needs a human's approval) with the standard rules: domain → `kernel`; application → `projects-domain`, `kernel`; infrastructure → both plus `store`.
- **Git:** `gix` (gitoxide, MIT/Apache-2.0) read-only. `git2` is excluded because libgit2's license is not on the `deny.toml` allow list. If `gix` lacks a needed status feature, fall back to `git status --porcelain=v2 --branch` spawned by Rust with a fixed argument shape and the repository as working directory, never through a shell.
- **File watching:** the `notify` crate on `<repo>/.git` only, not the worktree, to keep idle CPU under budget. The worktree is rescanned on the scheduled job.
- **CI and pull requests:** GitHub REST via `reqwest`: latest run for the branch from `/repos/{owner}/{repo}/actions/runs`, open pull requests from `/repos/{owner}/{repo}/pulls`. Conditional requests with `ETag`, `Retry-After` honored. The token is stored in the credential vault from [#040](040-accounts-and-oauth.md), never in plain config. These are off-device calls and are recorded in the audit log ([#074](074-privacy-dashboard.md)) with data class `Projects`.
- **Agent sessions:** Claude Code keeps JSONL sessions under `%USERPROFILE%\.claude\projects\`, Codex under `%USERPROFILE%\.codex\sessions\`. Both formats are undocumented: verify the paths and fields at implementation time, parse defensively (skip unknown lines), and pin fixtures for contract tests. Only the last 64 KB of a session is read for the summary.
- **Summaries:** session content is untrusted (it contains web pages, tool output). Summaries run quarantined with no tools ([ADR 0009](../architecture/adr/0009-untrusted-content-and-tool-permissions.md)) through the models router with `DataClass::Projects`, local by default. The router task is `session-summary` (local default, [assistant: model router](../architecture/assistant.md#model-router-models-context)).
- **Data and sync:** migration in `projects-infrastructure` adds `projects` (synced: id, name, remote URL), `project_clones` (device-local: project id, path), `project_state` and `agent_sessions` (device-local caches). The project list syncs ([sync: what syncs](../architecture/sync.md#what-syncs)); state is computed by each device for its own clones. Another device's project without a local clone shows "on <device name>" with no live state.
- **UI:** `src/projects/{domain,application,infrastructure,ui}`; settings screen in the `main` window; `src/projects/index.ts` exports the read-only project card used by `src/today/`.
- **Budget:** one refresh of a 10k-file repository ≤ 300 ms on an SSD; the watcher adds no measurable idle CPU.

## QA procedure

1. Clone any public GitHub repository to `C:\src\demo`. In Settings → Projects, add it. Observe branch, "0 uncommitted changes", and the last commit.
2. Edit two files. Within the refresh interval (or on "Refresh"), observe "2 uncommitted changes". Commit them; within 5 s observe the new last commit.
3. Add `C:\Users\<you>\Pictures`. Observe the "not a git repository" message.
4. Create a fine-grained GitHub token with read-only Actions and Pull requests access to the repository and save it. Observe the CI status and open PR count; click the CI link and confirm it opens the run in the browser.
5. Revoke the token on GitHub, click Refresh, and observe "token rejected" and the fix action in Settings → Accounts.
6. Run a short Claude Code session in `C:\src\demo` (for example "list the files"). Within 5 minutes observe the session title, "finished", and a one-line summary. Open the privacy dashboard and confirm no off-device entry for it.
7. Ask the assistant "what is the state of my projects?" and confirm the answer matches the view with no confirmation dialog.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed (connectors.md)
