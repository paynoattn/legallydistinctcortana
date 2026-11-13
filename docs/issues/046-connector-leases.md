# 046. Connector leases: one poller per account

| Milestone            | Size | Depends on                                             | Labels                                   |
| -------------------- | ---- | ------------------------------------------------------ | ---------------------------------------- |
| M4 Calendar and mail | S    | #031 (cross-device takeover completes with #069, #070) | area/connectors, area/sync, type/feature |

## Story

As a user with a desktop and a laptop, I want each mail and calendar account polled by only one of my devices, so that my mail is not fetched and summarized four times and my provider does not throttle me.

## Context

An account is polled by the device holding its lease in the synced store, renewed every cadence and taken over after 2× cadence of silence. Other devices receive digests and state through sync ([connectors: common rules](../architecture/connectors.md#common-rules)). Connector leases are among the synced entities ([sync: replication protocol](../architecture/sync.md#replication-protocol)). The generic lease primitive comes from the scheduler (#031). This issue applies it per account.

## Scope

- `ConnectorLease { account_id, holder_device, renewed_at (HLC), cadence }`, using the #031 lease with `ttl = 2 × cadence`.
- **Eligibility:** a device may take a lease only if it has working credentials for the account (#040: device-local tokens). Preference order when several are eligible:
  1. a pinned device
  2. a `headless` node
  3. a device on AC power
  4. the most recently active device
- Every account sync job (#036–#038, #041–#045) runs as a scheduler singleton keyed by account id, and the lease holder renews on each run.
- **Takeover:** a non-holder that sees no renewal for 2× cadence takes the lease and polls. Split brain (two partitioned devices both polling) is tolerated: results dedupe by message id or event uid, and when they reconnect, last-writer-wins by HLC leaves one holder.
- **Release:** on graceful shutdown, sleep, or credential loss, the holder writes a release so another device takes over at once.
- **UI:** each account page in `src/accounts/` shows "Polled by: <device name>, <n> min ago", with a "Poll from this device" pin.

Out of scope: replicating the lease between devices (#069, #070). Until those land, the lease has local semantics: it is always held by this device, and cross-device takeover completes with them. Deciding which device notifies is single delivery (#071), which never consults leases ([sync: presence and single delivery](../architecture/sync.md#presence-and-single-delivery)).

## Acceptance criteria

```gherkin
Feature: One poller per account

  Scenario: Only the lease holder polls
    Given devices "desktop" and "laptop" both have credentials for account "work-mail"
    And "desktop" holds the lease with cadence 5 minutes
    When both devices tick for 30 minutes
    Then only "desktop" calls the mail provider

  Scenario: Takeover after silence
    Given "desktop" holds the lease and goes to sleep at 10:00 without releasing
    When "laptop" ticks at 10:11
    Then "laptop" holds the lease and polls

  Scenario: Graceful release on sleep
    Given "desktop" holds the lease
    When "desktop" receives a sleep notification
    Then it writes a release
    And "laptop" polls at its next tick

  Scenario: Device without credentials never takes the lease
    Given "laptop" lists the account as "Sign in on this device"
    And the holder has been silent for 1 hour
    Then "laptop" does not take the lease
    And the account page shows "No device can poll this account"

  Scenario: Split brain resolves to one holder
    Given "desktop" and "laptop" both took the lease while partitioned
    When they sync again
    Then the holder with the later HLC keeps the lease and the other stops polling
    And no duplicate digest items exist

  Scenario: Pin to this device
    When I choose "Poll from this device" on "laptop"
    Then "laptop" takes the lease at its next tick and "desktop" stops
```

## Design notes

- **Crates:** the lease use case for connectors lives in `scheduler-application` (singleton jobs keyed by account). Eligibility (credentials present) is answered by an `accounts-application` port, wired in the root.
- **arch.toml:** no new crates. `scheduler-application` must not depend on `accounts-application`; the eligibility port is defined in the scheduler and implemented by an adapter in the root.
- **Migrations:** none beyond `scheduler_leases` from #031. Lease rows are synced ops once #069 lands.
- **Dependency note:** the issue ships in M4 with local semantics. The cross-device acceptance scenarios need #069 and #070 to run against real devices. Until then they run in `features/core/connector-leases.feature` with two in-process cores sharing a fake op-log transport.
- **Privacy:** leases hold device ids and timestamps only.

## QA procedure

Run after #070 lands. Use two Windows 11 PCs in the same sync group on one LAN, both signed in to QA-MSA.

1. On PC A, open Settings → Accounts → QA-MSA. Observe "Polled by: PC A".
2. On PC B, observe the same holder. Use Resource Monitor on PC B to confirm no connections to `graph.microsoft.com` during two cadences.
3. Put PC A to sleep. Within one cadence on PC B, observe "Polled by: PC B".
4. Wake PC A. Observe that PC B keeps the lease (no flapping) and PC A shows "Polled by: PC B".
5. On PC A, click "Poll from this device". Observe that the holder moves to PC A within one cadence.
6. Sign out of QA-MSA on PC B only, then sleep PC A for 3 cadences. Observe "No device can poll this account" on PC B.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
