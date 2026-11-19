@settings
Feature: Settings
  As someone who uses the assistant every day
  I want to change its shortcuts, voice server, indicator, and startup
  So that it fits my machine and my habits

  Background:
    Given the settings window is open

  Scenario: The current settings are shown
    Then the "Dictate (hold to talk)" shortcut is "F9"
    And the "Read selection aloud" shortcut is "Ctrl+Alt+R"
    And the "Host" field is "127.0.0.1"
    And the "Speech recognition port" field is "10300"
    And the "Voice port" field is "10200"
    And the "Dictation language" field is "auto"
    And the "Show the listening indicator" switch is on
    And the "Reduce motion" switch is off
    And the "Start at sign-in (hidden in the tray)" switch is off

  Scenario: Recording a new shortcut
    When I change the "Dictate (hold to talk)" shortcut
    Then I am asked to "Press a shortcut, or Esc to cancel"
    When I press "Control"
    Then I am asked to "Press a shortcut, or Esc to cancel"
    When I press "Control+Shift+Space"
    Then the "Dictate (hold to talk)" shortcut is "Ctrl+Shift+Space"
    And the "Change Dictate (hold to talk)" button is focused

  Scenario: Escape cancels recording
    When I change the "Read selection aloud" shortcut
    And I press "Escape"
    Then the "Read selection aloud" shortcut is "Ctrl+Alt+R"
    And the "Change Read selection aloud" button is focused

  Scenario: Reset restores the default shortcut
    When I change the "Dictate (hold to talk)" shortcut
    And I press "F8"
    And I click the "Reset Dictate (hold to talk)" button
    Then the "Dictate (hold to talk)" shortcut is "F9"

  Scenario: A duplicate shortcut is refused
    When I change the "Dictate (hold to talk)" shortcut
    And I press "Control+Alt+KeyR"
    Then the "Change Dictate (hold to talk)" button shows the error "Both shortcuts are the same. Pick a different one."
    And Save is disabled

  Scenario: An invalid port is refused
    When I set the "Speech recognition port" field to "70000"
    Then the "Speech recognition port" field shows the error "Use a whole number from 1 to 65535."
    And Save is disabled

  Scenario: Saving shows Saved
    When I set the "Host" field to "Voice.Local"
    And I turn on "Reduce motion"
    And I click the "Save" button
    Then the save status says "Saved."
    And the "Host" field is "voice.local"
    And the "Reduce motion" switch is on

  Scenario: Revert restores the saved values
    When I set the "Host" field to "elsewhere"
    And I click the "Revert" button
    Then the "Host" field is "127.0.0.1"

  Scenario: The core's field error is shown inline
    When I set the "Dictation language" field to "English"
    And I click the "Save" button
    Then the "Dictation language" field shows the error 'Use "auto" or a language code such as "en".'

  Scenario: The connection status is shown and can be tested
    Then the connection status says "Speech recognition and voice are reachable at 127.0.0.1."
    When I click the "Test connection" button
    Then the connection status says "Speech recognition and voice are reachable at 127.0.0.1."
