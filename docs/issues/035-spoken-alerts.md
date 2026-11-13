# 035. Spoken alerts

| Milestone    | Size | Depends on | Labels                                 |
| ------------ | ---- | ---------- | -------------------------------------- |
| M3 Reminders | S    | #026, #033 | area/notify, area/speech, type/feature |

## Story

As an accessibility user, I want reminders and meeting heads-ups spoken aloud in a fitting tone, so that I hear them when I am not looking at the screen.

## Context

The vision lists "a spoken heads-up" for calendar alerts and spoken alerts for reminders ([vision](../product/vision.md#proactive)). Voice is one of the delivery channels the notify policy picks ([assistant: proactive engine](../architecture/assistant.md#proactive-engine)). Speech requests go to the queue with `source: Alert`, and alerts preempt replies and read-aloud ([voice-and-speech: requests](../architecture/voice-and-speech.md#requests)).

## Scope

- A `VoiceChannel` adapter for the notify context. It turns a delivered notification into `Speak { text, voice, mood, rate, source: Alert }`.
- Deterministic alert text templates in `notify-domain`. No LLM is involved.
  - Reminder: "It's 7 PM. Take out the trash."
  - Late reminder: "You missed a reminder at 7 PM: take out the trash."
  - Heads-up: "Your meeting Design review starts in 10 minutes. There's a Teams link."
  - Timer: "Your 10 minute timer is done."
- Mood per source, editable: reminders `Neutral`, heads-ups `Alert`, overdue or Critical `Urgent`, timers `Cheerful`.
- An optional chime 300 ms before speech (setting, on by default).
- **When not to speak.** Fall back to the toast only, with the reason recorded in the delivery log, when:
  - a dictation or assistant capture is active (the mic is open)
  - the screen is locked (setting; default: don't speak when locked)
  - the policy says quiet hours, Focus, or presentation (from #033)
  - the default output device is muted
- Barge-in: the stop hotkey or Esc stops the alert immediately, and the toast stays.
- Settings: in Settings → Notifications, per source "Speak this" (off by default for reminders, on per calendar via #041). The page also has voice, mood per source, and chime.

Out of scope: acting on alerts by voice ("snooze"), which waits for the assistant voice turn (#051); echo cancellation (#087); and the spoken morning briefing (#067).

## Acceptance criteria

```gherkin
Feature: Spoken alerts

  Scenario: Reminder is spoken in the configured mood
    Given "Speak reminders" is on with mood Neutral
    When the reminder "take out the trash" is delivered at 19:00
    Then the speech queue receives "It's 7 PM. Take out the trash." with source Alert and mood Neutral
    And a toast is also shown

  Scenario: Alert preempts read-aloud
    Given a long article is being read aloud
    When a heads-up is delivered with "Speak heads-ups" on
    Then read-aloud pauses, the heads-up is spoken in mood Alert, and read-aloud resumes

  Scenario: No speech while dictating
    Given a dictation session is Listening
    When a reminder is delivered with "Speak reminders" on
    Then nothing is spoken
    And the delivery log records "mic in use"

  Scenario: No speech when the screen is locked
    Given the session is locked and "Speak when locked" is off
    When a reminder is delivered
    Then nothing is spoken and the toast is shown

  Scenario: Stop hotkey silences the alert
    Given an alert is speaking
    When I press the stop hotkey
    Then playback stops within 100 ms and the toast remains

  Scenario: Late reminder wording after sleep
    Given a reminder due at 19:00 is delivered at 19:40 after resume
    Then the spoken text is "You missed a reminder at 7 PM: take out the trash."
```

## Design notes

- **Crates:** `notify-domain` holds the templates and mood choice. `notify-application` holds the `VoiceChannel` port. The root wires it to the `speech-application` `Speak` use case, so notify never imports `speech-application` (arch rule: no cross-context application deps).
- **Ports:** the mic-in-use check comes from a `CaptureState` query exposed by voice through an event-fed read model in notify, not a direct call. Locked state comes from `PowerEvents` (lock and unlock).
- **Locale:** times are formatted per the Windows locale (12 h or 24 h). Templates are English for MVP.
- **Privacy:** spoken text can be overheard. Off by default for reminders, and "don't speak when locked" is on by default.
- **Budget:** from delivery decision to first audio, p95 is under 300 ms ([voice-and-speech budgets](../architecture/voice-and-speech.md#budgets)), with the chime played during synthesis.

## QA procedure

1. Settings → Notifications → Reminders: turn on "Speak this" and set the mood to Neutral. Create "in 1 minute water the plants". Observe the chime, then "It's <time>. Water the plants." spoken, plus a toast.
2. Select a long paragraph in Edge and press Win+Alt+R to read it. Create a reminder due during reading. Observe read-aloud pausing for the alert and resuming.
3. Hold the dictation key while a reminder comes due. Observe a toast only.
4. Lock the PC (Win+L) before a reminder is due. Unlock after. Observe that nothing was spoken (ask someone nearby, or check the delivery log) and the toast is present.
5. Let an alert start speaking and press Esc. Observe that it stops at once.

## Definition of done

- [ ] Acceptance scenarios in `features/` pass in `make check`
- [ ] `make mutate SCOPE=changed` at 100%
- [ ] QA procedure in `qa/procedures/` (scripted in `qa/scripts/` where possible) passes
- [ ] Docs updated where behavior or architecture changed
