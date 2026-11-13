# 0011. Brand-neutral code

Status: accepted

## Decision

The product name is undecided; the concepts are in [`docs/brand/`](../../brand/). Code, crates, and packages use functional names (`desktop`, `reminders-domain`, `infer-host`). The product name, wake phrase, and identifier live in three places only: `crates/desktop/tauri.conf.json`, `src/shared/brand.ts`, and the installer config. "Cortana" must not appear in any shipped string (trademark).

## Consequences

- Renaming is a three-file change plus assets.
- The bundle identifier (`dev.localassistant.desktop`) must be final **before the first public release**. Changing it later moves the app's data folder and breaks autostart and notification registration (issue [#005](../../issues/005-finalize-brand-and-identifier.md)).
