# Lampion

**Say it:** LAM-pee-on, /ˈlæm.pi.ən/. Wake phrase: "Hey Lampion".

## Meaning and story

A lampion is a small paper lantern, the kind children carry on lantern walks on November evenings. Lampion is that light for your desktop: a small glow in the corner that follows you and brightens when you speak. It runs on your own machine, so the light is yours and nobody else holds the switch.

## Personality

Warm, gentle, unhurried. Lampion is a companion that keeps you company in the evening, not a butler. It speaks briefly, never cheerleads, and stays quiet when nothing matters. When it does speak up, it sounds like someone who has already done the sorting for you.

## Voice

- **Reminder:** "It's Thursday, seven o'clock. The trash goes out tonight."
- **Morning briefing opener:** "Morning. It's a quiet one: a meeting at ten, and two emails that want you."
- **Before deleting files:** "I'll move 214 files from Downloads to the recycle bin, mostly old installers. You can still bring them back. Go ahead?"

## Palette

| Name | Hex | On light surfaces | On dark surfaces |
|---|---|---|---|
| Dusk | `#1F2347` | Primary text, mark keyline, wordmark | App-icon plate, deep panels |
| Glow | `#FFB81C` | Mark body only (needs the Dusk keyline) | Mark body, focus rings, live indicators |
| Wick | `#D9820B` | Lantern caps, large icons (not text: 2.9:1 on white) | Lantern caps, secondary accents |
| Paper | `#FFF1C7` | Lantern core, warm highlight panels | Wordmark and headings on Dusk |
| Plum | `#6E3A7D` | Links and accent text (8:1 on white) | Avoid as text; use Glow instead |
| Mist | `#C8CBE3` | Dividers, disabled controls | Secondary text on Dusk or `#202020` |

Rule of thumb: Glow is the light, so it appears where something is live (listening, a reminder firing). Everything else stays Dusk, Paper and neutral.

## Typography

All fonts are SIL Open Font License, so they can be bundled.

- **Young Serif** (Bryan Kirschen and Noirblancrouge): wordmark, display, Today-screen headings. A soft, heavy old-style serif with the warmth of a lantern label. The wordmark is Young Serif Regular, lowercase, +1% tracking, outlined to paths in `lockup.svg`.
- **Atkinson Hyperlegible Next** (Braille Institute): UI and body text. It is built for low-vision readers, which matters for an assistant whose personas include accessibility users.

## Mark and motion

The mark is a hanging chōchin-style paper lantern: hanger, cap, a pleated amber body with a pale core, and a bottom cap. Dusk keylines carry the shape on light surfaces. On dark surfaces the amber body carries it, so the lantern looks lit. The tray icon drops the hanger and pleats and keeps the cap, body, core and cap.

| State | Motion |
|---|---|
| Idle | Hangs still. The core breathes slowly (opacity 85–100%, 4 s loop). Roughly every 20 s it makes one tiny sway (±2°), as if a draft passed. |
| Listening | The core grows and brightens with your input level (scale 1.0–1.35 mapped to RMS, about 60 ms smoothing). A warm halo fades in behind the lantern. Each keystroke in the assistant box gives a small flicker (+5% brightness, 120 ms decay). |
| Thinking | Swings like a lantern carried on a walk: a pendulum rotation of ±8° around the hanger point, 1.4 s period, ease-in-out. The core dims to 70%. |
| Speaking | The core pulses with the speech envelope. Halo rings expand outward at the start of each phrase. |
| Done / error | Done: one bright pulse. Error: the core dims and the lantern gives a short ±3° shake. |
| Reduced motion | No sway or rings. States are shown by brightness and halo only. |

Everything animates with transform and opacity, so the overlay stays cheap and inside the brief's under-1%-CPU idle budget.

## Files

- `mark.svg`: symbol, 48 × 48.
- `lockup.svg` / `lockup-dark.svg`: symbol and wordmark for light and dark surfaces.
- `app-icon.svg`: 256 × 256 Windows 11-style plate.
- `tray-16.svg`: hand-simplified 16 × 16.

## Name risk (quick web search, not legal clearance)

**Low.** No AI assistant, voice product or well-known software uses the name. Small unrelated projects do: an iOS gratitude-journal app ([App Store](https://apps.apple.com/eg/app/lampion/id6748689047)), a QLC+ light-show tool ([GitHub](https://github.com/arukosakai/lampion)), a macOS local web-stack manager ([GitHub](https://github.com/webfalter/Lampion)), and a spending tracker ([lampion.money](https://lampion.money/)). A trademark search in classes 9 and 42 is still needed before launch.

Wake-word notes: three syllables with distinct consonants (l, m, p, n), and not a word people say day to day. "Lamp" is a partial overlap ("turn off the lamp"), but the wake model needs the full three syllables.

## Strengths

- The clearest mark at 16–24 px: a lantern stays a lantern at tray size, on both light and dark taskbars.
- The motion explains itself: a light that brightens when you speak needs no tutorial.
- The story is the product: a small light of your own, lit locally, that keeps you company and stays quiet otherwise.
- Warm without looking like the common AI-default warm palette (cream background with a clay accent). Indigo and amber is a dusk scene.

## Weaknesses

- English speakers may not know the word. Some will read it as "lamp-ion".
- The paper-lantern motif leans East Asian and Northern European festive. It needs care so it doesn't turn into a holiday decoration.
- On dark surfaces the Dusk keyline disappears and the mark relies on amber alone. That's fine at 16 px, but less crisp than a single-color mark.
