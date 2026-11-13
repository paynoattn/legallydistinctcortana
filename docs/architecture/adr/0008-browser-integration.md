# 0008. Right-click read-aloud in the browser

Status: proposed

## Context

The user wants to highlight text on a web page, right-click, and hear it read aloud in a chosen mood. On Windows, the context menu on a web page belongs to the browser. There is no OS hook for adding items to another app's menu for selected text.

## Decision

- A Manifest V3 extension (Chrome, Edge, Firefox) adds a "Read aloud" submenu with moods for selected text. It talks to the desktop app through **native messaging**: `native-host.exe`, then a named pipe.
- A global hotkey covers every other app through UI Automation.
- An Explorer context-menu command (post-MVP) covers files.

## Consequences

- Users install the extension once, from the Edge and Chrome stores or Firefox AMO. The desktop installer registers the native host for all three browsers.
- No localhost HTTP port is exposed. The native host accepts only our extension ids.
