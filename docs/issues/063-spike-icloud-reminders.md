# 063. Spike: iCloud Reminders access

| Time box | Needed by    | Labels                                      |
| -------- | ------------ | ------------------------------------------- |
| 2 days   | Could (v0.2) | area/connectors, area/reminders, type/spike |

## Question

Is there a clean, supported way for a Windows app to read and write a user's iCloud Reminders (create, complete, and see due dates and recurrence), and if not, what is the least fragile bridge that keeps the user's data private and our terms-of-service exposure at zero?

## Why it matters

Apple users keep reminders in the Reminders app. [connectors](../architecture/connectors.md#reminders-and-tasks) records the risk: reminders in the modern Reminders app are not exposed over CalDAV, and Apple publishes no web API. The MVP ships without iCloud Reminders unless this spike finds a clean path, so the answer decides whether a connector issue exists for v0.2 at all.

## Options to evaluate

| Option                                               | Known (to verify)                                                             | Must verify                                                                                                                                                                                                                                                              |
| ---------------------------------------------------- | ----------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **CalDAV VTODO on `caldav.icloud.com`**              | Works for iCloud Calendar with an app-specific password (#041)                | Whether any reminder lists are still served as VTODO, for which accounts (lists never upgraded?), and whether Apple still allows creating such lists; if yes, #038 already covers it                                                                                     |
| **Shortcuts automation on the user's iPhone or Mac** | Shortcuts can read and create reminders and make HTTP requests or write files | How the Shortcut reaches us: our `local-api` is loopback-only and token-authenticated, so a phone cannot reach it; a file in iCloud Drive that Windows iCloud syncs is an option; scheduling limits for personal automations; how two-way sync and completion would work |
| **macOS helper using EventKit**                      | EventKit is the supported Reminders API on macOS                              | Requires the user to own a Mac that is on and signed in; it would be a `headless` peer on macOS with a small EventKit adapter, so it depends on #085                                                                                                                     |
| **Reverse-engineered iCloud web endpoints**          | Some open-source libraries use the private web API                            | Rejected by default: undocumented, breaks without notice, may violate Apple's terms, and needs the user's Apple ID password plus 2FA handling. Document only.                                                                                                            |
| **Ask the user to move reminders**                   | Microsoft To Do and Google Tasks have APIs                                    | Not a connector; a fallback message                                                                                                                                                                                                                                      |

## Method

1. Using a test Apple ID with an iPhone, create upgraded lists and, if possible, a legacy list. Query `caldav.icloud.com` with an app-specific password via PROPFIND and REPORT; record which collections advertise `VTODO` in `supported-calendar-component-set`.
2. Build a two-action Shortcut: export reminders changed since the last run as JSON to an iCloud Drive folder, and import JSON commands from another file. Run it with iCloud for Windows syncing the folder; measure lag and failure modes.
3. Read Apple's developer documentation for EventKit permissions on current macOS, and sketch the helper's data flow (no implementation).
4. Review Apple's iCloud terms for automated access, and note the conclusion with a dated copy.

## Output

- An update to [connectors.md](../architecture/connectors.md#reminders-and-tasks) through a new ADR (next free number), "iCloud Reminders", with the chosen path or a documented "not supported" decision.
- If a path is chosen, a new connector issue for v0.2 with its own acceptance criteria, and notes on whether it extends [#038](038-connector-caldav-vtodo.md), needs a file-drop adapter in `reminders-infrastructure`, or waits on [#085](085-macos-linux.md).
- If the Shortcut path needs a network listener, a security note for [#078](078-security-review.md): it must not widen `local-api` beyond loopback.
- User-facing text for Settings → Accounts explaining what works for Apple users today.
