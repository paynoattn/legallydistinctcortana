# Corner indicator

Preconditions: Windows 11 with a microphone. A Wyoming Whisper server and a Wyoming Piper server are reachable, as in [dictation](dictation.md) and [read-aloud](read-aloud.md). The app runs with `make dev` (or an installed build), and the voice server in Settings points at those servers. In Settings, "Show the listening indicator" is on and "Reduce motion" is off. Run with `RUST_LOG=info` to see the log lines quoted below.

1. Open Notepad and click into it. Hold F9. → Expect: the lantern appears in the bottom-right corner of the screen, inside the work area (not over the taskbar), within a blink of the key going down. Notepad's caret keeps blinking, so Notepad still has focus.
2. Keep holding F9 and speak, then stay quiet. → Expect: the lantern brightens and grows a little while you speak and settles when you stop.
3. Release F9. → Expect: the lantern shows that it is thinking until the text is typed, then shows a check for under a second, then fades out. Notepad keeps focus throughout, and the text lands at its cursor.
4. Tap F9 quickly without speaking. → Expect: the lantern appears and fades out without a check, and nothing is typed.
5. Move the mouse to a second monitor, preferably one at a different scale, and dictate. → Expect: the lantern appears in that monitor's bottom-right corner, at the same physical size as on the first. Move the taskbar to the left or top, then dictate. → Expect: the lantern still sits 16 px inside the bottom-right of the work area.
6. Select a paragraph in Notepad and press Ctrl+Alt+R. → Expect: the lantern shows that it is speaking while the text is read and fades out when the reading ends or is stopped with Ctrl+Alt+R again.
7. While a reading plays, hold F9. → Expect: the dictation takes over the lantern (listening, then thinking), and it does not jump back to speaking while you dictate.
8. Disable the microphone (Settings → System → Sound → Input → your microphone → Don't allow), then press F9. → Expect: the lantern shows "Can't use the microphone." with a "Choose a mic" link, and the mouse can click it (every other state lets clicks through to the window underneath). Click "Choose a mic". → Expect: Windows' sound settings open and the error goes away. Allow the microphone again.
9. In Settings, set the host to a machine that does not run the servers (for example `192.0.2.1`), save, then dictate a sentence. → Expect: after release the lantern shows "Can't reach speech recognition at 192.0.2.1:10300." with "Open settings". Click "Open settings". → Expect: the settings window opens and the error goes away. Restore the host.
10. Cause the error from step 9 again and click the reason itself, not the link. → Expect: the error goes away and nothing opens. Cause it once more and leave it alone. → Expect: it fades out by itself after about 6 seconds.
11. With the error from step 9 on screen, click a window underneath the indicator's 160 by 160 px square, outside the pill. → Expect (known limitation): the click does not reach that window until the error is gone, since the whole square takes clicks while an error is shown. Once the error fades, click the same spot. → Expect: the click reaches that window.
12. In Settings, turn on "Reduce motion" and save, then dictate. → Expect: the lantern only fades between states; nothing grows, sways, or pulses. Turn it off again.
13. In Settings, turn off "Show the listening indicator" and save, then dictate. → Expect: no lantern appears, and dictation still types the text. Turn it back on.
14. Hold and release F9 twenty times in a row in Notepad, quickly. → Expect: Notepad keeps focus the whole time and the lantern ends hidden.

Cleanup: restore the voice server host, allow the microphone, and set "Reduce motion" and "Show the listening indicator" back to how you found them.
