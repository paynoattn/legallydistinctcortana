# Samara

**Say it:** SAM-uh-ruh, /ˈsæm.ər.ə/ (the botanical pronunciation). Wake phrase: "Hey Samara".

## Meaning and story

A samara is a winged seed, like the maple key that spins down like a helicopter. Your identity in this assistant is a seed too: 24 words that let it travel to your other machines, and nowhere else. The samara carries one seed exactly as far as it needs to go, which is how sync between your devices should work.

## Personality

Light, optimistic, outdoorsy. Samara sounds like someone who just came in from a walk: plain words, a little weather in the briefing, never in a rush. It is the most hopeful of the five, and the one that puts the 24-word seed at the center.

## Voice

- **Reminder:** "You're at work now. This is the one about the parking permit."
- **Morning briefing opener:** "Good morning. Clear skies until four, a light day, and your laptop already has last night's notes."
- **Before deleting files:** "Ready to clear 214 files from Downloads into the recycle bin. Nothing is gone until you empty it. Shall I?"

## Palette

| Name | Hex | On light surfaces | On dark surfaces |
|---|---|---|---|
| Canopy | `#1E4A35` | Primary text, wordmark | Deep panels |
| Key | `#D59B48` | Mark wing, warm highlights (not text) | Mark wing, accent text |
| Kernel | `#7A4A1E` | Seed and leading-edge vein, secondary text (7.4:1 on white) | Seed only (low contrast on `#202020`) |
| Sap | `#5F9B3A` | Success, "synced" state, buttons with white text at large sizes | Success fills, focus rings |
| Sky | `#BFE0F0` | App-icon plate, selection tint | Accent text on Canopy |
| Husk | `#F3E6CF` | Highlight panels | Primary text and wordmark |

## Typography

SIL Open Font License, bundled.

- **Alegreya** (Juan Pablo del Peral, Huerta Tipográfica): wordmark (Medium Italic, lowercase, outlined to paths in `lockup.svg`) and display. It is calligraphic, organic and literary.
- **Alegreya Sans**: UI and body, designed as Alegreya's companion. The pair keeps Samara gentle but readable.

## Mark and motion

A single dry maple key: a dark seed, a papery tan wing with a thick leading-edge vein and fine cross-veins. The first drafts used a green wing, which read as a vegetable at small sizes. The dry autumn key is the form that actually flies, and it reads as a winged seed. The tray icon keeps the wing, leading vein and seed.

| State | Motion |
|---|---|
| Idle | Drifts: a slow rotation of ±4° around the seed, 5 s. |
| Listening | The wing lifts (−14°) as if catching wind, and the veins brighten with your input level. Keystrokes give a small flutter. |
| Thinking | Autorotation like a falling maple key: a continuous spin around the seed (1.1 s per turn) with a faint two-step trail. |
| Speaking | The wing flutters ±3–7°, following the speech envelope. |
| Done / error | Done: the spin settles and the key "lands" (a small drop and stop). Error: one wobble. |
| Reduced motion | No spin. A static lift plus an opacity pulse. |

## Files

`mark.svg` (48 × 48), `lockup.svg`, `lockup-dark.svg`, `app-icon.svg` (256 × 256, a sky plate), `tray-16.svg`.

## Name risk (quick web search, not legal clearance)

**Medium.** In August 2026 Self Storage Manager launched **SAMARA**, an AI agent operators query by voice or text ([PRWeb](https://www.prweb.com/releases/self-storage-manager-introduces-samara-its-new-ai-agent-for-enterprise-self-storage-operations-302861730.html)). There is also a healthcare AI platform ([samarawell.com](https://www.samarawell.com/)) and a music app. Two names sit close by sound: **Samsara** (a fleet company with an AI assistant) and an open-source voice-control project also called Samsara ([GitHub](https://github.com/Morne-Ingstar/Samsara)).

Wake-word notes: Samara is a common given name and a Russian city. Households with a Samara, or anyone saying "Samantha", may wake the assistant by mistake. Pronunciation is also split (SAM-uh-ruh or suh-MAR-uh), which makes the wake model harder to train.

## Strengths

- The most direct link to the product's identity model: the seed phrase becomes the brand.
- The spin is a beautiful, natural "thinking" animation.
- An autumn palette (tan, forest green, sky) that looks nothing like other assistants.

## Weaknesses

- The weakest small-size mark of the five. Below 24 px it is "a seed or feather" rather than clearly a maple key.
- The given-name problem makes it a weaker wake word than the others.
- Medium name risk, including an AI voice agent with the same name.
