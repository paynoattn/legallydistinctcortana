# 073. Encrypted backup to folder and restore

| Milestone | Size | Depends on | Labels                              |
| --------- | ---- | ---------- | ----------------------------------- |
| M7 Sync   | M    | #069       | area/sync, area/store, type/feature |

## Story

As a person with only one PC, or who worries about losing all of them at once, I want the app to write an encrypted backup to a folder I choose (OneDrive, a USB drive, a NAS) and to restore it with my recovery phrase, so that losing my devices does not mean losing my reminders, routines, memory, and settings.

## Context

[sync: replication protocol (backups)](../architecture/sync.md#replication-protocol) and the backup key in [security: key hierarchy](../architecture/security.md#key-hierarchy). [ADR 0005](../architecture/adr/0005-identity-and-keys.md) promises that setup offers encrypted backups because losing the phrase and every device loses the data. The snapshot reads the synced entities from [#069](069-op-log-replication.md).

## Scope

- Settings → Backup: choose a folder, "Back up now", schedule (off, daily, or weekly at a chosen time; default off), and how many to keep (default 7).
- **Backup file** `<app>-backup-<UTC timestamp>.<ext>`: a header (magic, format version, group fingerprint, created at, source device, version vector) and a body with every synced entity's current state, field history, and tombstones. The content is compressed (`zstd`), then encrypted in chunks with a random file key; the file key is wrapped by the backup key ("backup/v1", derived from the phrase and kept sealed in every device's store, so scheduled backups need no phrase).
- Writes are atomic: write to a temporary name in the same folder, flush, then rename.
- **Restore** from onboarding ("Restore from a backup") or Settings → Backup: pick a file, enter the phrase, preview what it contains (counts per entity, date), confirm. Restored data is imported as ops with their original HLCs, so it merges with existing data by the normal rules and replicates to other devices.
- Failures (folder missing, disk full) go through the delivery policy as a low-priority notification and appear in Settings → Backup.

Out of scope: model files, audio, OAuth tokens and other device-local secrets, device-local settings; cloud backup services (the folder can be inside one).

## Acceptance criteria

```gherkin
Feature: Encrypted folder backup

  Scenario: Back up now
    Given a backup folder "D:\Backups" and a reminder titled "Renew passport"
    When I click "Back up now"
    Then a new backup file appears in "D:\Backups"
    And the file does not contain the bytes of "Renew passport" in any encoding

  Scenario: Restore on a fresh PC
    Given a backup file made on PC A
    When I install on PC B, choose "Restore from a backup", pick the file, and enter my phrase
    Then I see a preview with the number of reminders, routines, and memory items
    And after I confirm, PC B shows the same reminders, routines, memory, and shared settings

  Scenario: Wrong phrase
    When I try to restore with a phrase that did not create the backup
    Then I see "This phrase does not open this backup" and nothing is imported

  Scenario: Damaged file
    Given a backup file truncated by 1 KB
    When I try to restore it
    Then I see "This backup is damaged" and nothing is imported

  Scenario: Newer data is not overwritten
    Given a backup from Monday and a reminder renamed on Tuesday
    When I restore Monday's backup on the same PC
    Then the reminder keeps Tuesday's title
    And reminders deleted since Monday stay deleted

  Scenario: Scheduled backups keep the newest 7
    Given daily backups and 7 existing backup files in the folder
    When the next daily backup runs
    Then the folder holds the 7 newest backups

  Scenario: The folder is unavailable
    Given the backup folder is on a USB drive that is unplugged
    When a scheduled backup runs
    Then I get a low-priority notification "Backup failed: folder not found"
    And the next scheduled run tries again

  Scenario: Backup from a newer version
    Given a backup file with a higher format version than this app understands
    When I try to restore it
    Then I see "This backup was made by a newer version. Update the app to restore it."
```

## Design notes

- **Crates:** `sync-domain` (backup header, retention selection, format versioning; pure), `sync-application` (`CreateBackup`, `InspectBackup`, `RestoreBackup`; ports `BackupSink`, `SnapshotReader`, `OpImporter`, `BackupKeyDeriver`), `sync-infrastructure` (file adapter, `zstd`, chunked AEAD via the `chacha20poly1305` STREAM construction so truncation and reordering are detected), `identity-application` for reading the sealed backup key when backing up and deriving it from the phrase when restoring. The scheduled run is a scheduler job ([#031](031-scheduler.md)).
- **Keys:** every device keeps the backup key sealed in its encrypted store ([security: key hierarchy](../architecture/security.md#key-hierarchy)), so manual and scheduled backups run without the phrase. A restore on a device with no keys needs the phrase, from which the backup key is derived. After a phrase change ([#072](072-device-removal-rotation.md)) new backups use the new key; older files still need the old phrase.
- **Restore semantics:** import as ops through #069's apply path, never by overwriting tables, so LWW and tombstones protect newer data.
- **Budgets:** a profile with 10,000 entities backs up in ≤ 5 s on a mid-range laptop and runs at background priority.
- **Privacy:** the file holds `Mail` (digests and triage results), `Memory`, `Calendar`, `Feeds`, and `Projects` data, all encrypted. Writing to a synced folder (OneDrive) sends only ciphertext; the dashboard ([#074](074-privacy-dashboard.md)) notes the backup location.
- **UI:** `src/settings/` backup page and `src/onboarding/` restore path.

## QA procedure

1. On PC A, set the backup folder to a OneDrive folder and click "Back up now". Observe a new file. Search the file for a reminder title with `findstr /c:"Renew passport"`; expect no match.
2. On a fresh PC B (or VM), install, choose "Restore from a backup", pick the file from OneDrive, enter the phrase. Check the preview counts, confirm, and compare data with A.
3. Repeat with one word of the phrase changed; observe the error and that nothing is imported.
4. Copy the backup, cut 1 KB from its end (`fsutil file seteof` or a hex editor), and try to restore; observe "damaged".
5. On A, rename a reminder, then restore the earlier backup on A; observe the newer title kept.
6. Set the folder to a USB drive, schedule daily at a time 2 minutes from now, and unplug the drive. When the time passes, observe the failure notification and the error in Settings → Backup.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Backup format documented in sync.md (header fields, versioning)
