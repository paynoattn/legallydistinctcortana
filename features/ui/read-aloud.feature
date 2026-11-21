@read-aloud
Feature: Read-aloud voice
  As someone who listens to selected text
  I want to pick the voice it is read in from the reader's voice button
  So that every reading sounds the way I chose

  Scenario: The voice picker offers the server voices in the preferred language
    Given the text "Hello world" is selected
    And I open the voice picker
    Then the language is "American English (en_US)"
    And the voice is "Server default"
    And the voices are listed in order "Server default, amy (medium), libritts (high)"
    And there is no speaker choice

  Scenario: A picked voice and speaker are remembered after a reload
    Given the text "Hello world" is selected
    And I open the voice picker
    When I pick the voice "libritts (high)"
    And I pick the speaker "p240"
    And I reload the window
    And I open the voice picker
    Then the voice is "libritts (high)"
    And the speaker is "p240"
    And the language is "American English (en_US)"

  Scenario: The default speaker is remembered after a reload
    Given the text "Hello world" is selected
    And I open the voice picker
    When I pick the voice "libritts (high)"
    And I pick the speaker "p240"
    And I pick the speaker "Default speaker"
    And I reload the window
    And I open the voice picker
    Then the voice is "libritts (high)"
    And the speaker is "Default speaker"

  Scenario: Switching the language picks that language's first voice
    Given the text "Hello world" is selected
    And I open the voice picker
    When I pick the language "German (Germany) (de_DE)"
    Then the voice is "thorsten (medium)"
    And the voices are listed in order "Server default, thorsten (medium)"
    And there is no speaker choice

  Scenario: A picked language is remembered after a reload
    Given the text "Hello world" is selected
    And I open the voice picker
    When I pick the language "German (Germany) (de_DE)"
    And I reload the window
    And I open the voice picker
    Then the language is "German (Germany) (de_DE)"
    And the voice is "thorsten (medium)"

  Scenario: Choosing the server default clears a remembered voice
    Given the text "Hello world" is selected
    And I open the voice picker
    When I pick the voice "amy (medium)"
    And I pick the voice "Server default"
    And I reload the window
    And I open the voice picker
    Then the voice is "Server default"
