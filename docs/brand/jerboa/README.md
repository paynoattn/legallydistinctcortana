# Jerboa

**Say it:** jer-BOH-uh, /dʒərˈboʊ.ə/. Wake phrase: "Hey Jerboa".

## Meaning and story

The long-eared jerboa is a tiny desert hopper whose ears are bigger than its head. It hears everything, and it moves in quick, happy bounds. For a voice assistant whose first job is listening, it is a companion that hears you first and bounces off to get things done.

## Personality

Playful, quick, affectionate. Jerboa is the familiar that sits on your desk: enthusiastic but brief, uses exclamation marks sparingly, and never makes jokes about your data. It is the most character-driven concept, aimed at people who want the assistant to feel like a pet more than a tool.

## Voice

- **Reminder:** "Trash night! Bins out before you settle in."
- **Morning briefing opener:** "Good morning! Your inbox behaved overnight. Just two need you."
- **Before deleting files:** "Found 214 old files in Downloads. They go to the recycle bin, not the void. Want me to toss them?"

## Palette

| Name | Hex | On light surfaces | On dark surfaces |
|---|---|---|---|
| Burrow | `#3A2619` | Primary text, wordmark, mark keyline | Deep panels |
| Dune | `#F2B45A` | Mark fur (with Burrow keyline), highlights | Mark fur, accent text, focus rings |
| Ear | `#FF8FA3` | Listening state, small accents (not text) | Listening state, badges |
| Oasis | `#1F8A70` | Buttons, large text, "done" (4.3:1 on white, so not for small text) | App-icon plate, success fills |
| Moon | `#FFF3DC` | Highlight panels, the eye glints | Primary text and wordmark |

## Typography

SIL Open Font License, bundled.

- **Fredoka** (Milena Brandão): wordmark and display. The wordmark is weight 600, lowercase, outlined to paths in `lockup.svg`. Rounded terminals match the round ears.
- **Lexend** (Bonnie Shaver-Troup and Thomas Jockin): UI and body. Wide, very readable, and friendly without being childish.

## Mark and motion

A front-facing jerboa head with two large splayed ears, pink inner ears and big dark eyes. Brown keylines carry the shape on light surfaces, and the sand fill carries it on dark ones. The tray icon keeps the ears, head and eyes, and drops the inner ears and nose.

| State | Motion |
|---|---|
| Idle | Sits still with relaxed ears. One ear flicks every 8 s or so. Blinks. |
| Listening | Both ears snap upright and turn toward you. The inner ears flush a deeper pink with your input level. Typing makes an ear twitch on each burst. |
| Thinking | The ears swivel independently, one forward and one back, alternating every 0.8 s. The eyes glance up. |
| Speaking | Small hops in time with the syllables (−4 px with a squash on landing), and a soft shadow underneath scales with each hop. |
| Done / error | Done: one big hop. Error: ears droop for a second. |
| Reduced motion | No hops. The ears change pose between states without tweening. |

## Files

`mark.svg` (48 × 48), `lockup.svg`, `lockup-dark.svg`, `app-icon.svg` (256 × 256, an oasis-green plate), `tray-16.svg`.

## Name risk (quick web search, not legal clearance)

**Medium.** No AI assistant uses the name, but **Jerboa for Lemmy** is a well-known open-source Android client for the Lemmy federated forum ([F-Droid](https://f-droid.org/en/packages/com.jerboa/), [GitHub](https://github.com/LemmyNet/jerboa)). Its audience (FOSS and privacy-minded users) is the same as ours. There is also an older LLM fine-tuning experiment from Jina AI ([GitHub](https://github.com/jina-ai/jerboa)).

Wake-word notes: three syllables with distinctive sounds (dʒ, b) and not an everyday word. A strong wake word.

## Strengths

- The most lovable mark, and big ears explain a voice assistant instantly.
- The richest motion vocabulary: ears give the overlay many readable states.
- A good fit for a free, community-built project that wants a mascot people draw fan art of.

## Weaknesses

- At 16–24 px it reads as "a rabbit" or "a mouse". The jerboa's real signature, its long tufted tail, doesn't survive at tray size.
- A cute mascot can undercut trust when the assistant is asking to send mail or delete files.
- It collides in mindshare with a well-loved FOSS app in our exact audience.
