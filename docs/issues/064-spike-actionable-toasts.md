# 064. Spike: Actionable toasts from a Tauri app

| Time box | Needed by | Labels                                       |
| -------- | --------- | -------------------------------------------- |
| 2 days   | #034      | area/os-integration, area/notify, type/spike |

## Question

How does an unpackaged Tauri 2 (Win32) app installed per user by NSIS show Windows toasts with buttons (Snooze 10 min, Done, Join) and receive the button press reliably, including when the app was restarted after the toast was shown and when the toast is clicked from Action Center hours later?

## Why it matters

Reminders without working Snooze and Done fail the MVP's done-criterion 4. [os-integration](../architecture/os-integration.md) plans Windows App SDK app notifications with a COM activation handler, an AppUserModelID, and a Start-menu shortcut, and flags the Tauri-compatible path as unconfirmed. The mitigation in the [MVP plan](../product/mvp-plan.md#risks) is a toast that opens an in-app reminder card. #034 needs a decision before it can be specified.

## Options to evaluate

| Option                                                       | Known (to verify)                                                                                                                                       | Must verify                                                                                                                                                                                                                                                                                                  |
| ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Windows App SDK `AppNotificationManager`**                 | Supports buttons and arguments; documented for unpackaged apps in recent SDK versions                                                                   | Minimum SDK version for unpackaged apps; whether it needs the Windows App Runtime installed (framework dependency) or can be self-contained, and the installer impact; how to call it from Rust (the `windows` crate's WinRT projection, or a small C++/C# shim); activation when the process is not running |
| **Classic WinRT `ToastNotificationManager` + COM activator** | Requires an AUMID on a Start-menu shortcut with a `ToastActivatorCLSID` property, and an `INotificationActivationCallback` COM server; works from Win32 | Registering the COM class per user (HKCU) from NSIS; implementing the callback with the `windows` crate; behavior when the app is not running (Windows launches it with `-ToastActivated`?)                                                                                                                  |
| **Protocol activation**                                      | Buttons with `activationType="protocol"` open `<app>://reminder/<id>/snooze`; the installer already registers `<app>://`                                | Whether it flashes a window; that `tauri-plugin-single-instance` forwards the URI to the running instance; whether protocol buttons work from Action Center after a reboot; security of handling a URI any app can invoke (ids must be opaque and actions idempotent)                                        |
| **`tauri-plugin-notification`**                              | Simple toasts                                                                                                                                           | Whether its Windows backend supports buttons and background activation at all; likely insufficient                                                                                                                                                                                                           |
| **Fallback: in-app reminder card**                           | Clicking the toast body opens a small window with the same actions                                                                                      | Only needs body activation, which every option supports                                                                                                                                                                                                                                                      |

## Method

1. Build a throwaway Tauri 2 app (outside the repo, or in a scratch branch) installed per user through an NSIS script that creates the Start-menu shortcut and AUMID.
2. Implement options 1–3 behind one Rust function `show(reminder_id, buttons)` and one activation callback that logs `(reminder_id, action)`.
3. Test matrix on Windows 11 23H2 and 24H2 (x64; ARM64 if a device is available): app running and focused; running in tray; exited; after reboot; Focus (Do not disturb) on, then clicked from Action Center; multiple monitors.
4. For each cell record: toast shown, buttons shown, callback received, latency, any window flash or console window.
5. Check that the toast `tag`/`group` lets #071 remove a toast on the other devices' copies (`ToastNotificationHistory.Remove` or the App SDK equivalent).

## Output

- An update to [os-integration.md](../architecture/os-integration.md) through a new ADR (next free number), "Actionable notifications on Windows", naming the API, the activation path, installer registration (AUMID, shortcut property, COM class or protocol), and the test matrix.
- Changes to [#034](034-actionable-toasts.md) (the `Notifier` port shape in `platform-ports` and its `platform-windows` adapter) and to [#075](075-installer-signing-updater.md) (registry entries, any Windows App Runtime dependency).
- Confirmation that removing a delivered toast by tag works, for [#071](071-presence-single-delivery.md).
- If no option works when the app is not running, #034 ships the in-app card fallback and records the gap.
