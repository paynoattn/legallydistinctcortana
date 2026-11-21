# Lampion artboards

Live HTML artboards for the places Lampion meets the user. Each opens directly from disk (no server or build) and loads its fonts from Google Fonts. Open [`index.html`](index.html) first.

| File | What it is |
| --- | --- |
| `01-overlay.html` | A Windows 11 desktop at 1920 × 1080 with the bottom-right voice indicator, live. |
| `01-overlay-storyboard.html` → `01-overlay-states.png` | The seven overlay states, each a 2× crop of the corner, with timing. |
| `02-read-aloud-context-menu.html` → `02-read-aloud-context-menu.png` | An Edge-like browser with the Chromium context menu, the mood submenu, the reading pill and Web Speech playback. |
| `03-today.html` → `03-today.png` | The full-screen Today view, with keyboard, mouse and gamepad navigation. |
| `04-focus-reader.html` → `04-focus-reader.png` | The focus reader: the read-aloud shortcut opens a window that lights each word as Piper speaks it. |
| `tokens.css` | Shared tokens, plus the CSS for the animated mark. |
| `mark.js` | The animated mark (state, level, rings, pulse, flicker, frozen poses for screenshots), shared by all three. |

Query strings for deterministic screenshots:

- `01-overlay.html?state=hidden|armed|listening|thinking|inserted|speaking|error`, plus `&mood=calm|alert|angry|…`, `&bounds=1` to outline the 160 × 160 window, and `&reduce=1` for reduced motion.
- `02-read-aloud-context-menu.html?static=1` (menu open) or `?state=reading&mood=calm`.
- `03-today.html?static=1` freezes the clock at 8:42 AM and the lantern's breathing.
- `04-focus-reader.html?static=1` freezes on sentence 3 with "step" lit; `&reduce=1` turns off the gliding light and smooth scroll.

```sh
CHROME="/c/Program Files/Google/Chrome/Application/chrome.exe"
"$CHROME" --headless=new --disable-gpu --hide-scrollbars --virtual-time-budget=5000 \
  --window-size=1920,1080 --screenshot="D:\\…\\03-today.png" \
  "file:///D:/www/foss-stuff/cortana/docs/design/artboards/03-today.html?static=1"
# storyboard: --window-size=1920,500 --force-device-scale-factor=2 on 01-overlay-storyboard.html
```

## Token plan

**Color.** The six brand colors, with one rule taken from the brand README: Glow means "live".

| Token | Hex | Role here |
| --- | --- | --- |
| Dusk | `#1F2347` | Today's ground; text on light; the reading pill and error pill |
| Glow | `#FFB81C` | Only live things: the lantern's light, Join (a meeting about to start), CI running, "needs your input", focus rings, the word being read |
| Wick | `#D9820B` | Lantern caps (in the mark only) |
| Paper | `#FFF1C7` | Headings and primary text on Dusk (13.4:1); the sentence being read on a web page |
| Plum | `#6E3A7D` | Accent text and focus rings on light surfaces (8.2:1 on white) |
| Mist | `#C8CBE3` | Secondary text on Dusk (9.4:1) |

Derived and used sparingly: Mist-dim `#A3A8D0` for tertiary text (6.5:1 on Dusk), Ember `#FF8A7A` for failures and conflicts on Dusk (6.6:1), Dusk-raise `#272B55` for pressed surfaces, and nine mood tints (RGB triplets) that only ever color the halo and rings. Contrast was computed rather than eyeballed. Every text pair is at least AA, and the smallest text on Dusk (Mist-dim, 16 px) is 6.5:1.

**Type.** Young Serif for display only (the greeting, section headings, the countdown, storyboard captions). Atkinson Hyperlegible Next for everything you read in Lampion's own UI. Segoe UI Variable for anything that belongs to Windows or the browser: the taskbar, title bars and the context menu. Lampion doesn't restyle the OS. The article uses Georgia, because it belongs to a third-party publication. Scale: 16 / 18 / 20 / 22 / 32 / 44 / 72 px on Today. The glance tier (greeting, countdown, schedule titles) is 22 px and up; the detail tier (sources, reasons, agent labels) is 16–20 px.

**Layout.**

```
Today (1920 × 1080, left-aligned, three columns, one light source)

 │  Thursday, 12 November. 6°C …       Needs you              For you
 │  Morning, Robin.                    Priya Raman  Outlook   Running 8B models …
 │  A full one. Standup soon …         …                      Hacker News  Because …
(L) Standup in 18 min      [ Join ]    Due today              …
 ┃  The rest of today                  ○ Return the library…
 ●  9:00 AM  Standup                                          
┌●  10:30 AM Architecture review       Projects
└●  11:00 AM 1:1 with Priya ⚠          Project   Uncommitted  CI   PRs  Last agent session
 ○  12:15 PM Call the pharmacy         lampion   3 files      ◌    2    Claude Code, finished …
 ●  …                                  …
 ○  7:00 PM  Take out the trash
 •  [Start my day] [Read it to me]  Built on this PC …      (A) Select (B) Desktop (Y) … LB RB
```

The overlay and read-aloud artboards are real Windows 11 scenes. The Lampion parts are small: a 64 px lantern in a 160 × 160 window 16 px above the taskbar, and a 540 px reading pill to its left.

**Principles.**

1. One light source. On Today the only bold thing is the lantern hanging at now, with its light pooling over the greeting and the next event. Everything else is flat Dusk with type and spacing: no cards, no shadows, no gradients except that light.
2. Glow means live. If it is yellow, it is happening now or wants you now.
3. Respect whose surface it is. On Windows and in the browser, Lampion matches the OS chrome and only owns its mark, its pill and its words.
4. Motion answers something. The lantern moves only in response to voice, keys or thinking. Reduced motion keeps the light (brightness and halo) and drops every transform.

## Review against generic defaults (before building)

- **First idea for Today: a 3 × 2 grid of rounded cards with a gradient header.** That is the SaaS-card kit from the skill's list of tells. It became one continuous scene: the day as a string, the lantern at now, and sections set as type on the ground. Projects became a real table because those rows are meant to be compared.
- **First idea for buttons: Glow on every primary action.** That would have spent the brand's one signal on chrome. Glow now appears only on Join and on things that are live. Start my day is Paper and Read it to me is outlined.
- **Labels.** No all-caps eyebrows, middle-dot meta strings or arrows on links. Meta lines are written as sentences ("Teams, 15 min", "Because you follow Rust").
- **Monospace for branch names** would have been the "monospace data label" tell. Branches use the UI face with a small branch glyph.
- **Numbers.** "Standup in 18 min" is a sentence in the serif, not a big-number tile.

## What changed after the screenshot critique

1. **Today, pass 1.** The reminder rows rendered as default white buttons, the Projects table ran into the footer, and the "Built on this PC" stamp collided with the For you heading. I reset the button styles, merged the table's columns, shortened the agent summaries to fit one line ("Added the focus navigator and 14 tests"), and moved the stamp into the footer.
2. **Today, pass 2.** The narrower left column wrapped "Morning, Robin." onto two lines and pushed the schedule into the footer. I set the column to 650 px, stopped the greeting from wrapping, and now position the lantern and its light from the countdown's real position in JS, so it can't drift. Morning times gained "AM" to match "2:00 PM". "3 uncommitted" wrapped, so the column is now "Uncommitted" with "3 files". The boiler summary was reworded so it doesn't break mid-time.
3. **Today, pass 3.** The light felt weak, so the pool is now an ellipse that reaches the greeting and Join, and the string from the lantern down to the next event glows. That is the light running to what's next.
4. **Read aloud.** The programmatic focus drew a black outline on "Read aloud with Lampion". Chromium shows a grey fill on hover, so the static scene now uses the hover state and adds a pointer on Calm, which makes the screenshot read as an action in progress.
5. **Storyboard.** "Overlay window, 160 × 160" was repeated on all seven frames, so it now appears only in the live artboard. Thinking and Error looked like Armed in a still frame, so they now show a faint copy at the other end of the swing or shake (a storyboard convention, not runtime behavior).
6. **Bugs found by driving the pages** (keyboard events, a stubbed `navigator.getGamepads`, the strip buttons): dictated text was inserted twice; the speed label read "1.×" after cycling back to 1.0; the menu's arrow-key handler could throw when the event target was the window. All fixed.

Checked:
- Every state and transition in the overlay demo.
- Space tap types a space; Space hold dictates; a hold under 400 ms is cancelled as too short.
- Reduced motion leaves no transform animation (only core opacity and the halo).
- In the read-aloud page: the menu, the submenu and keyboard navigation; the no-selection menu; mood, speed, pause, stop and Esc; and the sentence and word highlights.
- On Today: spatial navigation with arrows and D-pad, stick repeat (400 ms, then 150 ms), LB/RB, A, B (closes settings first, then goes to the desktop), Y, and Menu.

## Focus reader

The read-aloud shortcut opens a reader instead of reading blind. It is a proposal that changes the MVP in issue 028 (the shortcut opens the mood window) and sits beside the reading pill of issue 030 (reading without a window).

**Flow.** Select text, press the shortcut. The reader opens over the work (860 × 660 logical px, centred on the monitor the selection is on) and starts at once in the last mood and voice. A two-step flow (pick a mood, then read) costs a key press on every read, so the mood moves into the control bar, where 1 to 9 still pick one. Esc stops, closes, and returns focus to the app the text came from.

**What is lit.**

| Thing | Color | Contrast on Dusk |
| --- | --- | --- |
| The word being spoken | Dusk on a Glow light that glides from word to word (130 ms) and jumps on a new line | 8.7:1 |
| The sentence being spoken | Paper | 13.4:1 |
| Sentences ahead | Mist-dim | 6.5:1 |
| Sentences read | `#9095C2` | 5.2:1 |

Only the spoken word and the progress line are Glow, and the line turns Mist-dim when reading finishes. Ideally the corner lantern stays hidden while the reader is open, so the 30 px lantern in the title bar is the one light on screen; issue 095 leaves the overlay unchanged for now.

**Following.** The text scrolls to keep the lit line about a third of the way down. Scrolling with the wheel or the scrollbar stops following and shows "Follow the voice". Clicking a word reads from the start of its sentence, not mid-sentence, because a cut into a synthesized clip starts mid-syllable.

**Timing without timestamps.** Wyoming Piper returns audio only (`audio-start`, `audio-chunk`, `audio-stop`), with no word or phoneme times. Speech is already one clip per sentence, so:

1. The core reports `{ index, startsInMs, speechMs }` as each sentence is queued on the speakers: `startsInMs` is the audio already queued ahead of it, and `speechMs` excludes the mood's trailing pause.
2. The UI spreads `speech_ms` over the sentence's words by weight: estimated syllables (vowel groups, minus a silent final e), digits at 1.4 each, plus 1.3 after a comma, semicolon, colon or dash, plus 0.25 per word.
3. Each new sentence snaps the light back to the true clock, so drift never carries over. Within a sentence the error is a word or so on long numbers and abbreviations.

If a later Piper or engine (#025) reports phoneme alignments, the same event can carry real word start times and step 2 drops out. In the artboard, speed and mood apply from the next sentence. The product restarts the current sentence instead (issue 095), because a reading can start from any sentence; speed waits for rate support in the synthesizer.

**Bundle.** It is the existing `read-aloud` window restyled, with no new dependencies: spans per word, one absolutely positioned light, and a `useSyncExternalStore` store fed by `requestAnimationFrame` and the sentence events. Long selections (the 20,000-character cap) need the words of far-off paragraphs to stay plain text until they are near, to keep the DOM small.

**Accessibility.** The text column is a normal document, so a screen reader can read it, and the live highlight is not announced (`aria-live="off"`). The status line is the live region. Text size goes from 20 to 40 px with Ctrl and plus or minus. Reduced motion keeps the light and the colors but jumps instead of gliding and scrolls instantly.

## Accessibility notes

- Focus on Today is a 3 px Glow ring plus a soft outer ring and a faint fill, on everything focusable. Under `forced-colors` it uses the system Highlight color. The ring follows the mouse, as in a console shell, so the pointer and the controller never disagree.
- The overlay is `aria-hidden` and never focusable, matching the spec, and the demo's state readout is an `aria-live` region standing in for the main window's mirror. The error pill is the only clickable state, and its fix link is keyboard reachable.
- `prefers-reduced-motion` is honored on all three pages, and each has an explicit toggle (the strip, `?reduce=1`, or the Today settings sheet).
- Text contrast is AA or better everywhere. Ember on Dusk is 6.6:1, and failures always pair color with a glyph and a word ("Failed, 2 tests").

## Open design questions

1. **Idle overlay: hidden or resting?** The overlay spec says Hidden at idle; the brand motion table describes an idle lantern that breathes. The artboard follows the spec, sends the "keystroke flicker" to the tray lantern, and offers "Resting lantern at idle" as a toggle. A permanent corner light is more charming, but it competes with "proactive, not chatty".
2. **Ten-foot scale for the detail tier.** The glance tier on Today (22–72 px) holds up across a room; the 16–18 px sources and reasons do not. Proposal: a "TV distance" setting that scales the detail tier by 1.25 and drops For you to three items. Needs testing on a real TV and a handheld (a 7-inch 1080p panel is the opposite problem).
3. **Reminders appear twice on Today** (timed ones in the schedule, all of them in Due today). The spec asks for both. It's honest but redundant. One option is to keep untimed reminders only in Due today, and show timed ones there only once they're within the hour.
4. **The Mad and Deadpan moods.** The vision lists "sarcastic"; the speech spec lists Mad and Deadpan. The menu follows the speech spec. "Mad" reads oddly next to "Angry" in a menu; "Furious" may be clearer.
5. **The reading pill's position.** It sits left of the lantern above the taskbar. With a vertical taskbar or a second monitor it needs a rule, probably "the corner of the monitor the selection is on".
6. **The Today light in the morning.** Dusk is an evening scene, and Today opens at sign-in, usually in the morning. A dawn variant (a lighter Dusk, a cooler pool) could follow the clock. Worth trying before calling the palette final.
7. **Mood tints on light wallpapers.** Calm (lavender) and Deadpan (grey) halos are faint on pale wallpapers. The mark stays legible thanks to its Dusk keyline, but the mood is hard to see. A thin ring in the tint would carry it.
8. **Focus reader or pill by default?** The focus reader suits an article you want to follow; the pill suits a paragraph you want to hear while you keep working. One setting ("When I read aloud: open the reader / just read") or a modifier (Shift for the other one) both work. The reader also needs a rule for a second monitor and for a full-screen game, where it should probably fall back to the pill.
