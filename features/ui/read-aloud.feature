@read-aloud
Feature: Read aloud
  As someone reading on screen
  I want selected text read to me in a mood I pick
  So that I can listen instead of reading

  Scenario: The selection and the moods are shown
    Given the text "  Hello   brave   new world " is selected
    Then I see the preview "Hello brave new world"
    And I see "21 characters"
    And the moods are listed in order "Neutral, Calm, Cheerful, Alert, Urgent, Angry, Mad, Whisper, Deadpan"
    And the "Neutral" mood is focused

  Scenario: Choosing a mood by click starts reading
    Given the text "Hello world" is selected
    When I choose the "Calm" mood
    Then the status says "Reading in a calm voice."

  Scenario: Pressing a number key chooses that mood
    Given the text "Hello world" is selected
    When I press "8"
    Then the status says "Reading in a whisper voice."

  Scenario: Stopping a reading
    Given the text "Hello world" is selected
    And I choose the "Urgent" mood
    When I press the Stop button
    Then I see "Stopped."
    And there is no Stop button

  Scenario: Nothing selected shows guidance and Close returns to waiting
    Given the text "   " is selected
    Then I see the alert "Select some text first, then press the read-aloud shortcut."
    And the Close button is focused
    When I press the Close button
    Then I see "Select text in any app, then press the read-aloud shortcut."
    And there are no moods

  Scenario: Arrow keys move focus through the moods
    Given the text "Hello world" is selected
    When I press "ArrowDown"
    Then the "Calm" mood is focused
    When I press "ArrowUp"
    And I press "ArrowUp"
    Then the "Deadpan" mood is focused
    When I press "Home"
    Then the "Neutral" mood is focused
    When I press "End"
    Then the "Deadpan" mood is focused

  Scenario: The voice picker offers the server voices in the preferred language
    Given the text "Hello world" is selected
    Then the language is "American English (en_US)"
    And the voice is "Server default"
    And the voices are listed in order "Server default, amy (medium), libritts (high)"
    And there is no speaker choice

  Scenario: A picked voice and speaker are remembered after a reload
    Given the text "Hello world" is selected
    When I pick the voice "libritts (high)"
    And I pick the speaker "p240"
    And I reload the window
    Then the voice is "libritts (high)"
    And the speaker is "p240"
    And the language is "American English (en_US)"

  Scenario: The default speaker is remembered after a reload
    Given the text "Hello world" is selected
    When I pick the voice "libritts (high)"
    And I pick the speaker "p240"
    And I pick the speaker "Default speaker"
    And I reload the window
    Then the voice is "libritts (high)"
    And the speaker is "Default speaker"

  Scenario: Switching the language picks that language's first voice
    Given the text "Hello world" is selected
    When I pick the language "German (Germany) (de_DE)"
    Then the voice is "thorsten (medium)"
    And the voices are listed in order "Server default, thorsten (medium)"
    And there is no speaker choice

  Scenario: A picked language is remembered after a reload
    Given the text "Hello world" is selected
    When I pick the language "German (Germany) (de_DE)"
    And I reload the window
    Then the language is "German (Germany) (de_DE)"
    And the voice is "thorsten (medium)"

  Scenario: Choosing a mood after picking a voice still starts reading
    Given the text "Hello world" is selected
    When I pick the voice "amy (medium)"
    And I choose the "Calm" mood
    Then the status says "Reading in a calm voice."

  Scenario: Choosing the server default clears a remembered voice
    Given the text "Hello world" is selected
    When I pick the voice "amy (medium)"
    And I pick the voice "Server default"
    And I reload the window
    Then the voice is "Server default"
