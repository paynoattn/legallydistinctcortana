# Pilcrow

**Say it:** PIL-kroh, /ˈpɪl.kroʊ/. Wake phrase: "Hey Pilcrow".

## Meaning and story

The pilcrow (¶) is the paragraph mark: the sign that a new thought starts here. Medieval scribes drew it in red or blue to show where an idea began. Pilcrow is an editor that sits at your cursor, turns what you say into clean, punctuated text, and marks what is worth your attention.

## Personality

Dry, literate, exact. Pilcrow writes like a good copy editor: short sentences, no fluff, the occasional deadpan aside. Writers, students and developers will like it most. People who want warmth may find it a little bookish.

## Voice

- **Reminder:** "A note from last Tuesday's you: renew the passport."
- **Morning briefing opener:** "Good morning. Today's draft: one meeting, three emails worth your time, and a CI run that failed overnight."
- **Before deleting files:** "214 files in Downloads, mostly installers. They'll sit in the recycle bin until you say otherwise. Proceed?"

## Palette

The palette comes from the editor's desk: brown-black iron-gall ink, and the non-photo blue pencil copy editors used because cameras couldn't see it.

| Name | Hex | On light surfaces | On dark surfaces |
|---|---|---|---|
| Iron gall | `#2B2420` | Primary text, wordmark | Deep panels |
| Blueline | `#2F7FC1` | Mark, primary buttons, large text (4.3:1 on white; use Iron gall for small links) | Mark (3.6:1 on `#202020`, fine for graphics) |
| Non-photo | `#9ED8F0` | Ruled lines, selection tint, listening fill (not text) | Accent text, links, focus rings |
| Margin | `#EEF2F7` | Page and panel background | Primary text and wordmark |
| Graphite | `#6A6F7A` | Secondary text (5:1 on white) | Dividers |

## Typography

SIL Open Font License, bundled.

- **Libre Caslon Text** (Pablo Impallari): wordmark (Regular, capitalized, outlined to paths in `lockup.svg`) and display. Caslon is the classic book face.
- **Libre Franklin** (Impallari Type): UI and body. A Franklin Gothic revival, the traditional companion to Caslon in American printing.

## Mark and motion

A geometric pilcrow: a solid bowl and two stems in Blueline, one color on every surface. It is the crispest small-size mark of the five because it is built from rectangles and one half-circle. The tray icon is redrawn on the pixel grid with 2 px stems and a 2 px counter.

| State | Motion |
|---|---|
| Idle | Steady. No idle animation: an editor waits. |
| Listening | The glyph lightens to Non-photo blue and fills with Blueline "ink" from the bottom up to your voice level. |
| Thinking | The right stem blinks like a text caret at the Windows caret rate (530 ms on and off). While you type it holds solid, as a caret does. |
| Speaking | Short lines of "text" set out to the right of the glyph, one per phrase, then collapse back when it finishes. |
| Done / error | Done: the ink settles to full. Error: the bowl drains. |
| Reduced motion | No blinking. The fill level changes in steps. |

## Files

`mark.svg` (48 × 48), `lockup.svg`, `lockup-dark.svg`, `app-icon.svg` (256 × 256, a ruled-paper plate), `tray-16.svg` (pixel-snapped).

## Name risk (quick web search, not legal clearance)

**Medium to high.** Several AI writing products already use the name. **Pilcrow** ([trypilcrow.com](https://www.trypilcrow.com/)) is a desktop Markdown app with private on-device AI, the closest to us. **Pilcrow: Spell Check Keyboard** is an on-device iOS keyboard ([App Store](https://apps.apple.com/us/app/pilcrow-spell-check-keyboard/id6809833441)). There is also an open-source AI writing skill suite ([GitHub](https://github.com/SamGalanakis/pilcrow)), a Shopify support tool ([pilcrow.ai](https://pilcrow.ai/)) and a reporting product ([pilcrowreg.com](https://pilcrowreg.com/)).

Wake-word notes: two crisp syllables (p, l, k, r) and a rare word. An excellent wake word.

## Strengths

- The most legible mark at 16 px, with a single color that works on every surface.
- Strong ties to dictation and writing, the OpenWhispr half of the product.
- The caret-blink and ink-fill motions feel native on Windows.

## Weaknesses

- The highest name risk of the five, in the closest product category (private, on-device writing AI).
- The ¶ glyph already means "show formatting marks" in Word, so people may read the icon as a toolbar button rather than an assistant.
- It undersells the proactive half of the product (mail, calendar, briefings).
