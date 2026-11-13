# Issues

Implementation issues for the MVP and beyond. The index, with milestones, sizes, and dependencies, is in [`../product/mvp-plan.md`](../product/mvp-plan.md#issue-index). Each file can be pasted into a GitHub issue as-is: `gh issue create --title "<title>" --body-file docs/issues/<file>.md`.

Every feature issue is a story for the gauntlet (`/gauntlet` in Claude Code or Cursor): the **specifier** turns its acceptance criteria into `features/**.feature` and `qa/procedures/*.md`, then **coder → cleaner → hardener → qa** take it to `make gauntlet` green.

## Template (feature)

```markdown
# NNN. Title

| Milestone    | Size | Depends on | Labels                   |
| ------------ | ---- | ---------- | ------------------------ |
| M1 Dictation | M    | #012, #014 | area/voice, type/feature |

## Story

As a <persona>, I want <capability>, so that <outcome>.

## Context

Why this exists and links to the architecture sections it implements.

## Scope

- What is built.

Out of scope: what is deliberately not built here (and which issue covers it).

## Acceptance criteria

Gherkin scenarios at the system boundary (UI, IPC command, hotkey, OS effect), including unhappy paths.

## Design notes

Crates and UI features touched, ports and adapters, `arch.toml` / dependency-cruiser additions, data and migrations, privacy data classes, security notes, performance budgets.

## QA procedure

Numbered steps a person runs on a real Windows 11 machine, each with what they should observe.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
```

## Template (spike)

```markdown
# NNN. Spike: question

| Time box | Needed by | Labels |
| -------- | --------- | ------ |

## Question

## Why it matters

## Options to evaluate

## Method

## Output

An ADR (new or updated) with the recommendation, evidence, and the follow-up issues it creates or changes.
```
