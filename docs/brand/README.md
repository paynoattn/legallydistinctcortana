# Brand concepts

Five candidate identities for the assistant (see [`../product/vision.md`](../product/vision.md)). Each one has a name you say as a wake word, a mark that doubles as the animated bottom-right indicator, an app icon, a 16 px tray icon, a palette, OFL fonts and a voice.

Open [`index.html`](index.html) for the comparison board (it loads fonts from Google Fonts). [`brand-board.png`](brand-board.png) is a screenshot of it.

## Summary

| Concept | Idea | Personality | Wake word | 16 px legibility | Name risk* |
|---|---|---|---|---|---|
| [**Lampion**](lampion/) | A paper lantern: a small light that stays beside you, lit by your own machine | Warm, gentle, unhurried | "Hey Lampion", 3 syllables, distinct | Excellent | Low |
| [**Avocet**](avocet/) | A wading bird that sweeps the shallows and keeps only what matters (triage) | Calm, observant, economical | "Hey Avocet", 3 syllables; "avo-" is close to avocado | Good (reads as a water bird) | Low–medium |
| [**Jerboa**](jerboa/) | A big-eared desert hopper that hears you first | Playful, quick, affectionate | "Hey Jerboa", 3 syllables, very distinct | Good (but reads as rabbit or mouse) | Medium |
| [**Samara**](samara/) | The winged maple seed: your 24-word seed carries you to your own machines | Light, optimistic, outdoorsy | "Hey Samara", but a common given name | Fair (a winged seed or feather) | Medium |
| [**Pilcrow**](pilcrow/) | The paragraph mark ¶: an editor that turns speech into clean text | Dry, literate, exact | "Hey Pilcrow", 2 crisp syllables | Excellent | Medium–high |

\*Name risk is from quick web searches in November 2026 for existing software and AI products with the same name. **It is not legal clearance.** Before committing, run a trademark search (USPTO, EUIPO, WIPO Global Brand Database; Nice classes 9 and 42) and check domains and package names. Sources are in each concept's README.

The concepts differ in kind, not only in color. There's a light (Lampion), a watchful creature (Avocet), a companion pet (Jerboa), a natural object (Samara) and a typographic glyph (Pilcrow). Warmth runs from Jerboa and Lampion down to Pilcrow and Avocet. Energy runs from Jerboa down to Avocet.

## Recommendation: Lampion

1. **The mark works where it matters most.** It is a lantern at 16 px in the tray and at 24–48 px in the corner, on both light and dark Windows 11 surfaces. It needed no rework after the small-size review. Avocet, Jerboa and Samara all did.
2. **The motion explains itself.** The indicator's job is to react to typing and voice. A lantern whose light brightens as you speak, sways while it thinks and pulses as it talks needs no legend, and it animates with transform and opacity only, which keeps the overlay cheap.
3. **The story is the product.** "A small light of your own" carries local-first ("lit by your own machine"), home and hearth, and "proactive, not chatty": a lantern is quiet company that you notice when you need it.
4. **Lowest name risk of the five, and a good wake word.** No assistant or AI product uses the name. "Lampion" has three distinct syllables and isn't a word people say all day.
5. **Distinctive without leaning on AI-default styling.** Dusk indigo and amber is an evening scene, not the cream-and-clay look, and nothing like a blue ring.

Watch-outs: many English speakers won't know the word (an advantage for a wake word, a small cost for recall), and the lantern motif needs restraint so it doesn't read as a holiday decoration.

## Runner-up: Avocet

Its story matches the product's core promise most closely: it sifts mail, feeds and calendars and brings back three things. It also has a single-color mark that is easy to animate and theme, and low name risk (the main existing user is SLB's oil-and-gas operations software). It comes second because at tray size it reads as "a water bird" rather than an avocet, its calm can tip into cool, and "avo-" is a minor false-wake risk. Choose Avocet if the brand should lead with competence over companionship.

## Names considered and rejected

All checked by web search in November 2026:

| Name | Why not |
|---|---|
| Tomte | Already a private, local AI assistant for Mac ([tomteapp.com](https://tomteapp.com/)), plus an open-source Windows 11 app with whisper.cpp and MCP ([GitHub](https://github.com/xditya/tomte-windows)). High risk. |
| Vesper | A private on-device AI assistant ([get-vesper.com](https://get-vesper.com/)) and several "Vesper" screen assistants. High risk. |
| Kairo | A Windows voice assistant whose wake word is "Kairo" ([Microsoft Store](https://apps.microsoft.com/detail/9pdgzp1023h0)). High risk. |
| Hestia | Dozens of local-first AI assistants on GitHub already use the name. High risk. |
| Ingle | A local AI app pitched as "your AI, kept at home" ([GitHub](https://github.com/DemiInfinity/ingle)). High risk. |
| Marmot | An AI notes app whose assistant is "Ask the Burrow" ([marmotvox.com](https://marmotvox.com/)), and an MCP data catalog. Medium–high risk. |
| Tessera | A local AI coding-session workspace in the dev-tool space we hand off to ([GitHub](https://github.com/horang-labs/tessera)), plus several AI firms. Medium–high risk. |
| Nightjar | An open-source, end-to-end-encrypted P2P workspace in our exact audience ([night-jar.co](https://night-jar.co/)), and a popular game-streaming client. Medium risk. |
| Coati, Candela, Tindra, Gnomon, Quipu, Potoo, Numbat, Vernier | Each already in use by several AI or developer products. Tindra also sounds like "Tinder" as a wake word, Gnomon has a silent G, and Quipu raises cultural-appropriation concerns. |

## Files

```
docs/brand/
  README.md          this summary
  index.html         comparison board (references the SVGs by relative path, inlines the state frames)
  brand-board.png    1600 px wide screenshot of index.html
  <concept>/
    README.md        name, story, voice, palette, type, motion, risk, strengths and weaknesses
    mark.svg         symbol, 48 × 48 viewBox
    lockup.svg       symbol + wordmark for light surfaces (wordmark outlined to paths)
    lockup-dark.svg  the same for dark surfaces
    app-icon.svg     256 × 256 Windows 11-style icon
    tray-16.svg      hand-simplified 16 × 16 tray icon
```

All SVGs are hand-written vector files with no raster images and no live text. Every font named here is SIL Open Font License and available from Google Fonts.
