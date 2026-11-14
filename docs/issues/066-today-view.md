# 066. Today full-screen view at sign-in with gamepad navigation

| Milestone | Size | Depends on                               | Labels                            |
| --------- | ---- | ---------------------------------------- | --------------------------------- |
| M6 Today  | L    | #004, #009, #012, #041, #045, #059, #065 | area/today, area/ui, type/feature |

## Story

As a knowledge worker or developer, I want to sign in to Windows and land on a full-screen Today view, like Xbox full-screen mode, that shows my schedule, the mail that needs me, news for my interests, and the state of my projects, and that I can drive with keyboard, mouse, touch, or a gamepad, so that I know what my day looks like before I open anything, and I can drop to the desktop with Esc or B.

## Context

MVP done-criterion 8 in the [MVP plan](../product/mvp-plan.md#what-done-means-for-the-mvp). Behavior: [os-integration: Today mode](../architecture/os-integration.md#today-mode-full-screen-at-sign-in). Content and layout: [ui: Today view content](../architecture/ui.md#today-view-content). Data: `briefing-application` builds a `TodaySnapshot` from the calendar ([#041](041-calendar.md)), mail triage ([#045](045-mail-triage-digests.md)), feeds ([#059](059-feeds.md)), projects ([#065](065-projects.md)), and reminders read models.

## Scope

- `today` window (entry `today.html` → `src/today-main.tsx`, from [#004](004-multi-window-shell.md)): borderless full screen on the primary monitor, topmost only until the first user input, then normal z-order.
- Launch paths: autostart with `--today` when "Show Today at sign-in" is on; tray menu "Today"; hotkey Ctrl+Alt+T; the assistant ("show my day").
- Sections in order: **Now** (greeting, time, next event with countdown and Join), **Schedule** (timeline of events and timed reminders, conflicts highlighted), **Needs you** (mail needing a reply with one-line summaries; reminders due today), **For you** (3–7 news items, each with source and reason), **Projects**, and **Start my day**.
- Item actions: Join (opens the meeting link), Open (mail web link or default mail app; feed item in the browser; CI run in the browser; repo folder in Explorer), Done and Snooze for reminders, thumbs up/down on news (feeds interest weights).
- Navigation: keyboard (arrows, Tab, Enter, Esc), mouse, touch (tap, swipe-scroll), and gamepad via the Gamepad API (D-pad and left stick move focus, A activates, B closes, LB/RB jump between sections).
- Close to the tray with Esc, B, Win+D, or "Start my day". The core keeps running.
- Snapshot cache: the last snapshot is saved encrypted at sign-out or shutdown and painted at sign-in, then refreshed live from domain events.
- Setting "Show Today at sign-in" (device-local), offered during onboarding.

Out of scope: "Read it to me" speech ([#067](067-spoken-briefing.md)), weather (Could, v0.2), screen-reader sign-off ([#076](076-accessibility.md)), editing events or replying to mail from Today.

## Acceptance criteria

```gherkin
Feature: Today view

  Scenario: Today opens at sign-in from the cached snapshot
    Given "Show Today at sign-in" is on and a snapshot was cached at sign-out
    When the app starts with "--today"
    Then the Today window covers the primary monitor within 1 second of window creation
    And it shows the cached snapshot, then replaces it with fresh data without moving the focused item

  Scenario: Sections show today's data
    Given a meeting "Design review" at 10:00 with a Teams link, two mails triaged as NeedsReply, five ranked feed items, and a project with failing CI
    When Today is open at 09:40
    Then "Now" shows "Design review in 20 min" with a Join button
    And "Needs you" lists both mails with one-line summaries
    And "For you" lists five items with a reason such as "because you follow: local LLMs"
    And "Projects" shows the project with "CI failed"

  Scenario: Navigate with a gamepad and close with B
    Given a gamepad is connected and Today is open
    When I press D-pad down twice and A on a feed item
    Then that item opens in the default browser
    When I press B
    Then the Today window hides, the desktop is visible, and the tray icon remains

  Scenario: Close with Esc and reopen with the hotkey
    Given Today is open
    When I press Esc
    Then the Today window hides
    When I press Ctrl+Alt+T
    Then Today is shown again with keyboard focus on the first item of "Now"

  Scenario: Topmost only until the first input
    Given Today opened at sign-in
    When I press any key or move the mouse, then Alt+Tab to another window
    Then the other window appears in front of Today

  Scenario: Nothing connected yet
    Given no calendar, mail, or feeds are connected
    When I open Today
    Then each empty section shows a one-line explanation and a "Connect" action that opens the matching settings page

  Scenario: A source is failing
    Given the calendar account's token was revoked 2 hours ago
    When I open Today
    Then "Schedule" shows the last known events marked "Updated 2 h ago" with a "Fix" action
    And the other sections are unaffected

  Scenario: Remote content is not loaded
    Given a feed item whose summary contains an image tag and a link
    When it is shown on Today
    Then it renders as plain text with the link's domain shown
    And the Today webview makes no network request
```

## Design notes

- **Rust:** `briefing-application` gains `TodaySnapshotQuery` (ports to read models of calendar, mail, feeds, projects, reminders; built in ≤ 150 ms) and `SnapshotCache` (store-backed, encrypted with the profile). `desktop` adds the `today` window config, `--today` argument handling, full-screen placement on the primary monitor, and "drop topmost on first input". Ctrl+Alt+T is registered through `platform-ports::HotkeySource` ([#012](012-global-hotkeys.md)); the `--today` autostart flag is set through the `Autostart` port ([#009](009-tray-autostart-logging.md)).
- **Commands (tauri-specta):** `today_snapshot`, `today_open_item(item_ref)`, `today_feedback(item_id, up|down)`, `today_close`. `today_open_item` resolves the URL or path in Rust from the item id, accepts only `https`, `http`, `msteams`, `zoommtg`, and folder paths of watched projects, and opens it with the OS handler. The webview never receives a raw URL it can navigate to.
- **Capabilities:** the `today` window gets only those commands plus reminder Done/Snooze; no settings, file system, or shell permissions (`crates/desktop/capabilities/` is protected: needs a human's approval).
- **UI:** `src/today/domain` holds pure view models, the section order, and a spatial focus navigator (given focusable rectangles and a direction, return the next target; property-tested). `src/today/application` holds the snapshot store and a `GamepadInput` port. `src/today/infrastructure` polls `navigator.getGamepads()` from a `requestAnimationFrame` loop inside a subscription source (no `useEffect`), with stick deadzone 0.5, first repeat after 400 ms, then every 150 ms. Components read stores with `useSyncExternalStore`. Imports from other features only through their `index.ts` (`src/projects/index.ts`, `src/feeds/index.ts`).
- **Bundle budget:** 80 KB gzip JS for the `today` entry ([ui](../architecture/ui.md#windows-and-entry-points)).
- **Privacy:** the snapshot holds `Calendar`, `Mail` (summaries only, no bodies), `Feeds`, and `Projects` data. Nothing on Today calls a network service; the strict CSP blocks remote images, and feed and mail text is rendered as text ([assistant: untrusted content](../architecture/assistant.md#untrusted-content-and-prompt-injection)).
- **Motion and scale:** respects `prefers-reduced-motion` and the in-app setting; layouts at 100%, 150%, and 200% scale and from 1280×720 to 3840×2160; `forced-colors` honored.
- **Default:** os-integration.md does not say whether "Show Today at sign-in" defaults to on. This issue preselects it in onboarding to meet done-criterion 8; confirm with the lead.

## QA procedure

1. With calendar, mail, feeds, and one project connected, turn on Settings → "Show Today at sign-in". Sign out of Windows and sign back in. Observe Today full screen on the primary monitor with data from before sign-out, then a refresh.
2. Check each section against the sources: next meeting and countdown, timeline with an overlapping pair highlighted, mails needing reply, 3–7 news items with reasons, project state.
3. Connect an Xbox controller (USB or Bluetooth). Move with the D-pad and the left stick, switch sections with LB/RB, press A on a news item (browser opens), press B (Today hides, tray icon remains).
4. Press Ctrl+Alt+T; Today reappears with focus visible on the first item. Press Esc; it hides. Reopen and press Win+D; it hides.
5. Reopen, click "Start my day"; it hides. Reopen from the tray menu.
6. On a touch screen (or the Windows touch simulator), tap a reminder's Done; observe it disappear from "Needs you" and from the Reminders page.
7. Disconnect the network, revoke nothing, and reopen Today; confirm it still renders. In the privacy dashboard confirm no off-device entries came from Today.
8. Set Windows scale to 200% and turn on a contrast theme; confirm nothing is clipped and focus stays visible.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] `today` entry within its 80 KB gzip budget in `make qa`
- [ ] Docs updated where behavior or architecture changed
