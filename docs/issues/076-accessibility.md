# 076. Accessibility pass

| Milestone  | Size | Depends on | Labels                                    |
| ---------- | ---- | ---------- | ----------------------------------------- |
| M8 Release | M    | #066       | area/ui, area/accessibility, type/feature |

## Story

As a user who relies on a screen reader, keyboard-only input, high contrast, or large text, I want every window of the assistant to work with the tools I use, so that the app that reads text aloud for others is also usable by me.

## Context

The accessibility persona in the [vision](../product/vision.md#who-it-is-for) and the release sign-off in the M8 exit demo. Requirements: [ui: accessibility](../architecture/ui.md#accessibility). jsx-a11y strict is already a lint gate; this issue adds automated checks in acceptance tests, fixes what they and manual testing find, and records the screen-reader sign-off. Target: WCAG 2.2 level AA for every window.

## Scope

- **Automated:** an axe-core check (`@axe-core/playwright`) in the UI acceptance run for every entry (`main` routes, `overlay`, `today`, `panel`, `confirm`) in light, dark, and `forced-colors` modes. Serious or critical violations fail `make check`.
- **Keyboard:** every action reachable by keyboard; visible focus everywhere; logical tab order; `confirm` traps focus and returns it on close; Esc closes dialogs, the panel, and Today.
- **Screen readers:** Narrator and NVDA pass on onboarding (including the recovery phrase sheet), reminders, the assistant panel and confirm dialog, Devices, Privacy, and Today.
- **Live announcements:** overlay state changes (Listening, Thinking, Inserted, Error) and spoken-alert text are announced through an ARIA live region in the `main` window, because the overlay is not focusable.
- **Scale and contrast:** layouts at 100%, 150%, and 200% scale and with Windows text size at 200%; contrast ratio ≥ 4.5:1 for text in both schemes; `forced-colors` support with system colors.
- **Motion:** `prefers-reduced-motion` and the in-app "Reduce motion" switch to opacity-only transitions (overlay mark, Today).
- **Gamepad and touch** on Today, re-verified with Narrator running.

Out of scope: localization (separate issue); voice control of the whole UI (the assistant covers common commands).

## Acceptance criteria

```gherkin
Feature: Accessibility

  Scenario Outline: No serious axe violations
    Given the "<entry>" window is rendered with the fake IPC adapter in "<scheme>" mode
    When axe-core analyzes the page
    Then there are no serious or critical violations

    Examples:
      | entry   | scheme        |
      | main    | light         |
      | main    | forced-colors |
      | today   | dark          |
      | confirm | light         |
      | panel   | dark          |
      | overlay | light         |

  Scenario: Keyboard-only onboarding
    Given a fresh install
    When I complete onboarding using only Tab, Shift+Tab, Space, Enter, and arrow keys
    Then every step is reachable and focus is visible on every focused control

  Scenario: The confirm dialog traps focus
    Given the assistant asks to confirm moving 42 files to the recycle bin
    When I press Tab repeatedly
    Then focus cycles within the dialog
    And Esc cancels the action and returns focus to where it was

  Scenario: Overlay states are announced
    Given Narrator is running and the main window is open
    When I dictate a sentence
    Then Narrator announces "Listening", then "Inserted"

  Scenario: Large text does not clip
    Given Windows text size is 200% at a 1366×768 resolution
    When I open Settings → Reminders and Today
    Then no text is clipped and no horizontal scrolling is needed in the main column

  Scenario: Reduced motion
    Given "Reduce motion" is on
    When the overlay goes from Armed to Listening
    Then only opacity changes, with no scaling or movement
```

## Design notes

- **UI:** shared primitives in `src/shared/design/` (Button, Dialog, List, Toast, Mark) get the fixes so features inherit them; a `LiveAnnouncer` in `src/shared/` fed by the overlay state store through IPC events; a11y helpers stay in `domain`/`application` layers where they are logic (for example announcement text).
- **Tooling:** `@axe-core/playwright` as a dev dependency (check its license against the project's npm license policy); a cucumber-js step "the page has no serious accessibility violations" reused across features. `package.json` is protected: adding a dependency needs a human's approval.
- **Rust side:** toasts are OS-rendered and read by Narrator; verify toast text and button labels are meaningful. The tray menu uses OS menus.
- **Today:** the spatial navigator from [#066](066-today-view.md) must keep DOM focus in sync so screen readers follow gamepad moves.
- **Sign-off:** a short report in `qa/procedures/accessibility.md` with tool versions (Narrator build, NVDA version) and results, attached to the release.

## QA procedure

1. Turn on Narrator (Ctrl+Win+Enter). Complete onboarding with the keyboard only; confirm each control is announced with name and role and the recovery phrase sheet is read word by word.
2. Create a recurring reminder with the recurrence editor using only the keyboard and Narrator.
3. Ask the assistant to clean up downloads; in the confirm dialog, confirm focus stays inside and Esc cancels.
4. Repeat steps 1–3 with NVDA.
5. Dictate a sentence with the main window open and confirm the announcements.
6. Turn on a contrast theme (Settings → Accessibility → Contrast themes) and check every window.
7. Set text size to 200% and display scale to 150%; check Settings pages and Today for clipping.
8. Turn on Windows "Animation effects: off" and confirm reduced motion in the overlay and Today.
9. On Today, navigate with a gamepad while Narrator runs; confirm each focused item is announced.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes, with the sign-off report
- [ ] ui.md updated where behavior changed
