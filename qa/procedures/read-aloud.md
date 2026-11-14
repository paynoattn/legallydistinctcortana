# Read selected text aloud in a mood

Preconditions: Windows 11 with speakers or headphones. A Wyoming Piper server is reachable, for example `docker run -p 10200:10200 rhasspy/wyoming-piper --voice en_US-amy-medium` on a machine on your LAN or tailnet. The app runs with `VOICE_HOST=<that machine> make dev` (add `VOICE_TTS_PORT` if the port is not 10200, and `VOICE_TTS_VOICE` to pick a voice). Nothing else uses Ctrl+Alt+R.

1. Open a long article in Edge. Select a paragraph and press Ctrl+Alt+R. → Expect: a small dark window opens next to the mouse pointer, inside the screen, showing the start of the paragraph, its length in characters, and nine moods with Neutral highlighted.
2. Press 2. → Expect: the window shows "Reading in a calm voice." and the paragraph is read slower and softer, starting within about a second. When it ends, the window closes by itself.
3. Copy the word "original" to the clipboard. Select a sentence in Notepad, press Ctrl+Alt+R, and press Escape. Paste with Ctrl+V. → Expect: the window closed, and "original" is pasted, so the clipboard was restored.
4. In File Explorer, copy a file (Ctrl+C). Select a sentence in Edge, press Ctrl+Alt+R, and press Escape. Paste into a folder. → Expect: the file is pasted, so every clipboard format was put back. If the window instead says the clipboard was left untouched, paste still works the same way.
5. Select a paragraph in Word, press Ctrl+Alt+R, move down with the arrow keys to Mad, and press Enter. While it reads, press Ctrl+Alt+R. → Expect: louder, faster speech that stops immediately on the second press.
6. Select a paragraph, press Ctrl+Alt+R, choose Alert, then click into another app while it reads. → Expect: the window hides and the reading continues; Ctrl+Alt+R stops it.
7. Click an empty area of the desktop so nothing is selected and press Ctrl+Alt+R. → Expect: the window says "Select some text first, then press the read-aloud shortcut." with a focused Close button; Escape closes it.
8. Stop the Piper server. Select text, press Ctrl+Alt+R, and choose Neutral. → Expect: the window shows "can't reach the voice server at <host>:<port>: …" in the alert colour and nothing is read.
9. Move the mouse to the bottom-right corner of a second monitor (if you have one), select text there, and press Ctrl+Alt+R. → Expect: the window opens up and to the left of the pointer, fully on that monitor.

Cleanup: restart the Piper server if you stopped it. Nothing is stored by this feature.
