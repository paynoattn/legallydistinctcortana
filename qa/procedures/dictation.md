# Push-to-talk dictation

Preconditions: Windows 11 with a microphone. A Wyoming Whisper server is reachable, for example `docker run -p 10300:10300 rhasspy/wyoming-whisper --model base-int8 --language en` on a machine on your LAN or tailnet. The app runs with `VOICE_HOST=<that machine> make dev` (add `VOICE_STT_PORT` if the port is not 10300, and `VOICE_STT_LANGUAGE` to change the language, or `auto` to let the server detect it). Nothing else uses F9; stop `voice.py ptt` if it is running, or every dictation is typed twice. Run with `RUST_LOG=info` to see the log lines quoted below.

1. Open Notepad. Hold F9, say "The quarterly numbers are in.", and release. → Expect: within about a second of release, "The quarterly numbers are in. " (with a trailing space) is typed at the cursor. The log says "dictation finished outcome=typed 30 characters" and never contains the words themselves.
2. While holding F9, look at the microphone icon in the taskbar's system tray. Release F9. → Expect: the icon shows the app is using the microphone only while F9 is held.
3. Copy the word "original" to the clipboard. Dictate "hello there" into Notepad, then press Ctrl+V. → Expect: "hello there " is typed, and the paste inserts "original", so the clipboard was restored.
4. In File Explorer, copy a file (Ctrl+C). Dictate a sentence into Edge's address bar, then paste into a folder. → Expect: the sentence is typed and the file is pasted, so every clipboard format was put back.
5. Tap F9 quickly without speaking. → Expect: nothing is typed, and the log says "the key was only tapped, so nothing was transcribed".
6. Hold F9 for two seconds in silence and release. → Expect: nothing is typed, and the log says "no words were heard" (or Whisper's guess for the room noise).
7. Hold F9, say a long sentence, release, and press F9 again straight away while it is still being transcribed. → Expect: the first sentence is typed once; the log says the second press was ignored ("still typing the last dictation").
8. Dictate into VS Code, Teams, and a browser text area. → Expect: the text lands at the cursor in each.
9. Stop the Whisper server, then dictate. → Expect: nothing is typed, and the log says "dictation failed error=can't reach the speech recognition server at <host>:<port>: …". Start the server again and dictate. → Expect: it works without restarting the app.
10. Unplug or disable the microphone (Settings → System → Sound → Input), then press F9. → Expect: the log says "dictation could not start error=can't record from the microphone: …" and the app keeps running. Re-enable it and dictate. → Expect: it works.
11. Quit the app, start it with `DICTATE_SHORTCUT=ctrl+alt+d`, and dictate with Ctrl+Alt+D held. → Expect: it works, and F9 does nothing in the app.

Cleanup: restart the Whisper server if you stopped it, and re-enable the microphone. Dictation stores nothing.
