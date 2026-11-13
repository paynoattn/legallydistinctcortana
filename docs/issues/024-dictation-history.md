# 024. Dictation history (opt-in, encrypted)

| Milestone    | Size | Depends on | Labels                                               |
| ------------ | ---- | ---------- | ---------------------------------------------------- |
| M1 Dictation | S    | #019, #006 | area/voice, area/store, priority/could, type/feature |

## Story

As a writer, I want to turn on a history of what I dictated so I can recover text that went into the wrong window, knowing it is encrypted and deleted on a schedule I choose.

## Context

[Voice: dictation pipeline](../architecture/voice-and-speech.md#dictation-pipeline): history is off by default; when on, transcripts are stored encrypted, and audio is optional and kept 7 days by default. [Overview: data and persistence](../architecture/overview.md#data-and-persistence): audio is never written to disk unless the user turns on history. [Sync: what syncs](../architecture/sync.md#what-syncs): dictation history is device-local, with opt-in sync. Priority: **Could**. It is scheduled in M1 but may slip to v0.2 without blocking the milestone.

## Scope

- `voice-domain`: `HistoryEntry { id, at, text, raw_text, target_app, engine, duration_ms, audio: Option<AudioRef> }` and a `RetentionPolicy` (transcripts: 7, 30, or 90 days, or forever; audio: off, 1 day, or 7 days).
- `voice-application`: a subscriber to `DictationFinished` that stores entries only when history is on; `ListHistory`, `SearchHistory`, `DeleteEntry`, `DeleteAll`, and a daily retention sweep.
- `voice-infrastructure`: store tables `voice_history` and `voice_history_audio` (16-bit PCM blob, about 32 KB per second) with migration `voice/2`.
- Settings: `voice.history.enabled` (Device, off), `voice.history.retention_days`, `voice.history.keep_audio` (Device, off), `voice.history.sync` (Shared, off; takes effect once #069 lands).
- `src/dictation` → History page: list with time, app, and text; search; copy; play audio when kept; delete one; delete all (with confirmation).
- Turning history off asks whether to delete existing entries.

Out of scope: syncing history (#069 carries it if the user opts in), export (privacy export in #074).

## Acceptance criteria

`features/core/dictation-history.feature` and `features/ui/dictation-history.feature`:

```gherkin
Feature: Dictation history

  Scenario: Off by default
    Given a new profile
    When three dictations finish
    Then the history table is empty

  Scenario: Entries are stored when on
    Given history is on without audio
    When "Send the report Friday" is dictated into Outlook
    Then History lists it with the time and "Outlook"
    And no audio row exists

  Scenario: Audio is kept only when chosen and expires
    Given history keeps audio for 1 day
    When a dictation finishes and 25 hours pass
    Then the entry's text remains and its audio is deleted

  Scenario: Retention removes old transcripts
    Given retention is 7 days
    When the daily sweep runs on day 8
    Then entries older than 7 days are gone

  Scenario: Delete all
    When I choose "Delete all history" and confirm
    Then the list is empty and the store has no history rows

  Scenario: Search
    Given entries "budget review" and "lunch order"
    When I search "budget"
    Then only "budget review" is listed

  Scenario: Turning history off offers deletion
    Given history has 10 entries
    When I turn history off
    Then I am asked "Delete the 10 saved dictations?"
```

## Design notes

- Data class: `DataClass::Voice`. Stored only in SQLCipher; audio blobs are inside the encrypted database, never as files.
- Deletion: `DELETE` plus `PRAGMA secure_delete` for the history tables (verify the SQLCipher behavior) and a periodic `VACUUM` after large deletes, so deleted text does not linger in free pages.
- The history subscriber receives `DictationFinished` in process; nothing about history goes to the overlay.
- Search is `LIKE` over the decrypted database (SQLCipher decrypts pages transparently); full-text search can come later.
- Storage cap: warn at 500 MB of audio and offer to shorten retention.

## QA procedure

1. With a new profile, dictate three sentences. History is empty and says it is off.
2. Turn history on without audio. Dictate into Outlook and Notepad. Both appear with the right app names.
3. Turn on "Keep audio for 1 day", dictate, and play the entry. Your voice plays.
4. Search for a word from one entry. Only that entry shows.
5. Choose "Delete all" and confirm. The list is empty.
6. Turn history off with entries present. The deletion prompt appears.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
