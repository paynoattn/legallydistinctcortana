# 010. Settings framework: device-local and shared settings

| Milestone      | Size | Depends on | Labels                                 |
| -------------- | ---- | ---------- | -------------------------------------- |
| M0 Foundations | S    | #006, #008 | area/foundation, area/ui, type/feature |

## Status

**MVP slice implemented** on branch `feat/overlay-tray-settings` (2026-11-18): a settings window for the shortcuts, the voice server, the indicator, and start at sign-in, applied without a restart. It differs from the design below:

- `settings-domain` holds one typed `Settings` value with its validation (`Settings::with`, `Field`, `FieldError`), not a registry of definitions. There are no shared settings or scopes yet; everything is device-local.
- `settings-application::SettingsService` opens, puts in effect, and saves the settings through the `SettingsStore` and `SettingsEffects` ports. A save validates, applies what changed, then stores; if a shortcut can't be registered or the file can't be written, everything is put back.
- `settings-infrastructure::JsonFileStore` keeps them in `settings.json` in the app's config directory, not the encrypted store (#006). Environment variables (`VOICE_HOST`, `DICTATE_SHORTCUT`, and so on) give the defaults.
- The window is `index.html?view=settings`. Its commands (`settings_get`, `settings_default`, `settings_save`, `shortcuts_suspend`, `shortcuts_resume`) refuse any other window by label; they are not typed through #008 yet.
- QA: [`qa/procedures/tray-and-settings.md`](../../qa/procedures/tray-and-settings.md).

## Story

As a user with several PCs, I want preferences like quiet hours to follow me while hotkeys and microphones stay per machine, so that each PC behaves the way its hardware needs.

## Context

[Sync: what syncs](../architecture/sync.md#what-syncs) separates shared settings (synced) from device-local ones (hotkeys, per-device hardware). [Security: Tauri hardening](../architecture/security.md#tauri-hardening): only the main window may call settings commands. The overview has no settings context yet; this issue adds it.

## Scope

- `settings-domain`: a typed registry of settings. Each `SettingDef` has a key (`voice.dictation.mode`), a scope (`Device` or `Shared`), a value type (bool, enum, integer range, duration, string with max length, time-of-day range), a default, and a validator. Contexts declare their own definitions; the registry rejects duplicate keys.
- `settings-application`: `GetSettings`, `SetSetting`, `ResetSetting` use cases and a `SettingsRepository` port. Change notifications are published once #011 exists; until then the use case returns the new value.
- `settings-infrastructure`: store-backed repository (tables `settings_device`, `settings_shared`), migration `settings/1`.
- IPC (main window only): `settings_get_all`, `settings_set`, `settings_reset`, typed through #008.
- `src/settings` feature: a page with sections per context, rendered from the definitions (toggle, select, number, time range), inline validation messages, and a "This PC only" badge on device-local settings.
- First real definitions: `app.autostart_mode` (Device: off, tray, today), `app.reduce_motion` (Shared: bool), `privacy.local_only` (Shared: bool, default true; read by #047 and #074).

Out of scope: syncing shared settings (#069), hotkey bindings UI (#012), per-context pages (each feature issue).

## Acceptance criteria

`features/core/settings.feature` and `features/ui/settings.feature`:

```gherkin
Feature: Settings

  Scenario: Defaults apply on first run
    Given a new profile
    When the UI loads settings
    Then "Reduce motion" is off and "Local only" is on

  Scenario: A valid change persists across restart
    When I turn on "Reduce motion"
    And the core restarts
    Then "Reduce motion" is still on

  Scenario: Invalid values are rejected by the core
    When settings_set is called with app.autostart_mode = "sometimes"
    Then the call fails with code "invalid_setting" naming the allowed values
    And the stored value is unchanged

  Scenario: Unknown keys are rejected
    When settings_set is called with key "voice.nonexistent"
    Then the call fails with code "unknown_setting"

  Scenario: Device-local settings are labelled
    When I open Settings
    Then "Start at sign-in" shows the badge "This PC only"
    And "Reduce motion" does not

  Scenario: Only the main window may change settings
    Given the capability files
    Then settings_set is granted to the "main" window only
```

## Design notes

- `arch.toml`: `settings-domain` allow `["kernel"]`; `settings-application` allow `["settings-domain", "kernel"]`; `settings-infrastructure` allow `["settings-application", "settings-domain", "kernel", "store"]`. Standard bans on domain and application.
- Other contexts depend on `settings-domain` to declare definitions (domain-to-domain for shared value types is allowed), never on `settings-application`.
- Values are stored as a tagged, versioned encoding (not free JSON in domain code). A stored value that no longer validates after an upgrade falls back to the default and logs a warning without the value.
- Shared settings are written so #069 can add op-log entries per field later without a data migration: one row per key, with an `updated_hlc` column now.
- No secrets in settings. API keys and passwords go to the sealed vault (#006, #040).

## QA procedure

1. Start with a fresh profile and open Settings. "Local only" is on and "Reduce motion" is off.
2. Turn on "Reduce motion". Quit from the tray and restart. It is still on.
3. Hover "Start at sign-in". It carries the "This PC only" badge.
4. Using DevTools in the overlay window (debug build), call `settings_set`. It is rejected.
5. Navigate the whole page with Tab and Space only. Every control is reachable and focus is visible.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed (add the settings context to `overview.md`)
