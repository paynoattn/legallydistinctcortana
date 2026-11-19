@overlay
Feature: Push-to-talk indicator
  As someone dictating into any app
  I want a small light in the corner that follows my voice
  So that I know when the app is listening, thinking, and done

  Background:
    Given the indicator window is open

  Scenario: The indicator follows a dictation
    Then the indicator is hidden
    When the core sends the "armed" state
    Then the indicator is "armed"
    When the core sends the "listening" state
    Then the indicator is "listening"
    When the core sends the "thinking" state
    Then the indicator is "thinking"
    When the core sends the "inserted" state
    Then the indicator is "inserted"
    And the check badge is shown
    When the core sends the "hidden" state
    Then the indicator is hidden

  Scenario: Listening follows the voice level
    Given the core sends the "listening" state
    And the core sends the level 0
    When I note the size of the core
    And the core sends the level 1
    Then the core is larger than noted

  Scenario: An error shows its reason and a fix
    When the core reports the error "Microphone not found." with the fix "sound_settings"
    Then the indicator shows the reason "Microphone not found."
    And it offers the fix "Choose a mic"
    When I click "Choose a mic"
    Then the core is asked to run "overlay_fix"

  Scenario: Clicking the rest of the error dismisses it
    When the core reports the error "Settings could not be read." with the fix "settings"
    Then it offers the fix "Open settings"
    When I click "Settings could not be read."
    Then the core is asked to run "overlay_dismiss"

  Scenario: An error without a fix offers no link
    When the core reports the error "Transcription failed." with no fix
    Then the indicator shows the reason "Transcription failed."
    And it offers no fix

  Scenario: Reduced motion leaves no transform animation
    When the core sends the "listening" state with reduced motion
    And the core sends the level 1
    Then nothing in the indicator moves
    When the core sends the "thinking" state with reduced motion
    Then nothing in the indicator moves
    When the core sends the "inserted" state with reduced motion
    Then nothing in the indicator moves
