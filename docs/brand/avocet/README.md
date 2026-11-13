# Avocet

**Say it:** AV-uh-set, /ˈæv.ə.sɛt/. Wake phrase: "Hey Avocet".

## Meaning and story

The avocet is a wading bird that feeds by sweeping its upturned bill side to side through the shallows and keeping only what it wants. That is the assistant's job with mail, feeds and calendars: it sifts the flood overnight and brings you the three things that matter. It is calm, it watches, and it is never in a hurry to talk.

## Personality

Calm, observant, economical. Avocet is the colleague who reads everything and says one sentence. It is precise rather than warm, and quiet rather than cold. It uses numbers instead of adjectives.

## Voice

- **Reminder:** "You asked me to mention the invoice the next time you talked to Sam. Sam is calling now."
- **Morning briefing opener:** "Three things matter this morning. The rest can wait."
- **Before deleting files:** "214 files, 3.8 GB, all older than six months. They go to the recycle bin, not away. Confirm?"

## Palette

| Name | Hex | On light surfaces | On dark surfaces |
|---|---|---|---|
| Pied | `#1C2B33` | Primary text, wordmark | Deep panels, Today-screen background |
| Estuary | `#2E7D7F` | Mark, primary buttons, links (4.8:1 on white) | Mark (3.4:1 on `#202020`, fine for graphics) |
| Tide | `#6FC2BE` | Water lines, selection tint (not text) | Accent text, focus rings, links |
| Salt | `#F4F7F5` | Page background, the mark's eye and wing stripe | Primary text and wordmark |
| Silt | `#CDBB98` | Quiet dividers, "can wait" priority | Muted labels |

Estuary is a mid-tone picked to read on both Windows 11 Mica light (`#F3F3F3`) and dark (`#202020`) without a keyline.

## Typography

SIL Open Font License, bundled.

- **Hanken Grotesk** (Alfredo Marco Pradil), one family for everything. The wordmark is weight 430, lowercase, +4% tracking, outlined to paths in `lockup.svg`. UI uses 400 and 600. A plain, slightly wide grotesk keeps the voice precise and leaves the bird as the only expressive element.

## Mark and motion

A swimming avocet in profile: a low body with a pale wing stripe, an upright neck, and the long upturned bill. It is one teal silhouette, so it reads on light and dark without changes. The tray icon drops the stripe and keeps the body, neck, head and bill.

| State | Motion |
|---|---|
| Idle | Floats on a slow swell (±1 px, 4 s). Blinks every 6–10 s. |
| Listening | Lifts its head and stretches its neck toward you (+2 px), the eye widens, and a waterline appears under it that wobbles with your input level. Keystrokes send a tiny ripple. |
| Thinking | The feeding sweep: the bill scythes side to side (±9° around the body, 1.6 s period) and leaves a small wake. |
| Speaking | Ripples spread outward from the body at each phrase, with amplitude following the speech envelope. |
| Done / error | Done: one settled bob. Error: a quick head shake. |
| Reduced motion | No sweep. Ripples become a single static ring that fades. |

## Files

`mark.svg` (48 × 48), `lockup.svg`, `lockup-dark.svg`, `app-icon.svg` (256 × 256, a salt plate with a tide line), `tray-16.svg`.

## Name risk (quick web search, not legal clearance)

**Low to medium.** No AI assistant, voice product or consumer app uses the name. The main existing user is SLB's Avocet production-operations software for oil and gas ([SLB](https://www.software.slb.com/products/avocet)). It is enterprise software in a different market, but it is software, so it would come up in a class 9/42 trademark search. A closed ad-tech platform also used the name ([Crunchbase](https://www.crunchbase.com/organization/avocet)).

Wake-word notes: three syllables and a rare word. The opening "avo-" overlaps "avocado", which is a slight false-wake risk in kitchens.

## Strengths

- Its story fits the product's core promise (proactive, not chatty) more closely than any other concept: sift, rank, summarize.
- Lowest name risk among assistant-adjacent names, and the name is easy to spell once heard.
- Single-color mark: the simplest to animate, theme and print.
- The sweep motion is unusual and recognizable. No other assistant moves like this.

## Weaknesses

- At 16–24 px the bird reads more as "a duck" than "an avocet". The upturned bill is the only distinguishing feature.
- Calm and precise can tip into cool. It is the least cozy option for people who want a companion.
- Few people know the bird, so the story needs one line of explanation.
