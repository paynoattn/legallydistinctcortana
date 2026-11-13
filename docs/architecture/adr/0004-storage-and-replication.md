# 0004. Storage and replication

Status: proposed

## Context

Data must be encrypted at rest, queryable with SQL, and replicated between devices that are often offline, without a server.

## Decision

- One **SQLCipher** database per profile, accessed through `rusqlite` from the `store` crate. Each context owns its tables and migrations.
- Every synced mutation also appends a **signed, content-key-encrypted op** in the same transaction. Ops are field-level and ordered by a **hybrid logical clock**. They merge **last-writer-wins per field**; deletes are tombstones and sets are add-wins. Replication exchanges ops by version vector.
- Rich-text documents (post-MVP) use a CRDT library (Loro or Automerge) carried as op payloads.

## Consequences

- Simple, auditable merge semantics that fit reminders, settings, and memory.
- Concurrent edits to the same field resolve to one winner. The overwritten value stays visible in the item's history for 30 days.
- Ops grow until compaction, and compaction needs acknowledgement from every device. A long-offline device could block it, so after 90 days and a warning that device is dropped from the vector.

## Alternatives

- **cr-sqlite:** elegant, but its maintenance is uncertain and it sits below our encryption layer.
- **Full CRDT documents for everything:** heavier, and harder to query.
- **Event sourcing for every context:** more power than the product needs, and slow cold starts.
