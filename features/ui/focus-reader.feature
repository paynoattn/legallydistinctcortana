@read-aloud
Feature: Focus reader
  As someone who reads along while listening
  I want the read-aloud shortcut to open a reader that lights each word as the voice says it
  So that I can follow the text without losing my place

  Scenario: Reading starts when the selection arrives
    Given the last mood was "Calm"
    When the text "Good morning. Two emails need you." is selected
    Then reading starts from sentence 1 in the "Calm" mood
    And I see 2 sentences
    And the status says "Sentence 1 of 2."

  Scenario: The first reading is in the Neutral mood
    Given no mood was used before
    When the text "Good morning. Two emails need you." is selected
    Then reading starts from sentence 1 in the "Neutral" mood
    And the mood button says "Neutral"

  Scenario: The reader reads in the last voice and names it
    Given the last voice was "amy (medium)"
    When the text "Good morning. Two emails need you." is selected
    Then reading starts from sentence 1 in the voice "amy (medium)"
    And the voice button says "amy (medium)"

  Scenario: The title row and the first focus
    When the text "Good morning. Two emails need you." is selected
    Then I see the heading "Read aloud"
    And I do not see "Reading from"
    And the "Pause" button is focused

  Scenario: Paragraphs follow the line breaks in the selection
    When the text below is selected:
      """
      Good morning.
      Two emails need you. Lunch is at noon.
      """
    Then I see 2 paragraphs
    And paragraph 1 holds sentence 1
    And paragraph 2 holds sentences 2 and 3

  Scenario: The spoken word is lit from the sentence timing
    Given the text "Good morning. Two emails need you." is selected
    When sentence 2 starts now with 1200 ms of speech
    And 700 ms pass
    Then the word "need" is lit
    And sentence 1 is shown as read and sentence 2 as being read

  Scenario: A sentence that starts later is not lit early
    Given the text "Good morning. Two emails need you." is selected
    When sentence 1 starts in 500 ms with 800 ms of speech
    And 400 ms pass
    Then no word is lit
    When 100 ms pass
    Then the word "Good" is lit

  Scenario: Sentences ahead are shown as not yet read
    Given a reading is under way at sentence 1 of 3
    Then sentence 1 is shown as being read
    And sentences 2 and 3 are shown as ahead

  Scenario: Reduced motion moves the light without gliding
    Given the system prefers reduced motion
    And the text "Good morning. Two emails need you." is selected
    When sentence 2 starts now with 1200 ms of speech
    And 300 ms pass
    Then the light moves to the next word without gliding

  Scenario: Clicking a word reads from its sentence
    Given a reading is under way at sentence 1 of 3
    When I click a word in sentence 3
    Then reading starts again from sentence 3 in the same mood

  Scenario Outline: Next sentence
    Given a reading is under way at sentence 2 of 3
    When I <move>
    Then reading starts again from sentence 3 in the same mood

    Examples:
      | move                             |
      | press "ArrowRight"               |
      | click the "Next sentence" button |

  Scenario Outline: Previous sentence goes back one early in a sentence and restarts it later
    Given a reading is under way at sentence 2 of 3
    And sentence 2 has played for <played> ms
    When I <move>
    Then reading starts again from sentence <sentence> in the same mood

    Examples:
      | played | move                                 | sentence |
      | 0      | press "ArrowLeft"                    | 1        |
      | 1500   | press "ArrowLeft"                    | 1        |
      | 1501   | press "ArrowLeft"                    | 2        |
      | 1500   | click the "Previous sentence" button | 1        |
      | 1501   | click the "Previous sentence" button | 2        |

  Scenario: Previous sentence at the first sentence restarts it
    Given a reading is under way at sentence 1 of 3
    When I press "ArrowLeft"
    Then reading starts again from sentence 1 in the same mood

  Scenario: Next sentence at the last sentence does nothing
    Given a reading is under way at sentence 3 of 3
    When I press "ArrowRight"
    Then no new reading starts
    And the status says "Sentence 3 of 3."

  Scenario: Pause and resume
    Given a reading is under way at sentence 2 of 3
    When I press "Space"
    Then the reading is stopped and the status says "Paused. Sentence 2 of 3."
    When I press "Space"
    Then reading starts again from sentence 2

  Scenario: Pause and resume with the button
    Given a reading is under way at sentence 2 of 3
    When I click the "Pause" button
    Then the reading is stopped and the status says "Paused. Sentence 2 of 3."
    When I click the "Play" button
    Then reading starts again from sentence 2

  Scenario Outline: A choice made while paused starts reading
    Given a reading is under way at sentence 2 of 3 in the "Neutral" mood
    And I press "Space"
    When I <choose>
    Then reading starts again from sentence <sentence> in the "<mood>" mood

    Examples:
      | choose                     | sentence | mood     |
      | click a word in sentence 3 | 3        | Neutral  |
      | press "ArrowRight"         | 3        | Neutral  |
      | press "3"                  | 2        | Cheerful |

  Scenario: A voice picked while paused starts reading
    Given a reading is under way at sentence 2 of 3
    And I press "Space"
    When I open the voice picker
    And I pick the voice "amy (medium)"
    Then reading starts again from sentence 2 in the voice "amy (medium)"

  Scenario: Changing the mood restarts the current sentence
    Given a reading is under way at sentence 2 of 3 in the "Neutral" mood
    When I press "5"
    Then reading starts again from sentence 2 in the "Urgent" mood
    And "Urgent" is remembered for the next reading

  Scenario: Choosing a mood from the mood button
    Given a reading is under way at sentence 2 of 3 in the "Neutral" mood
    When I choose the "Whisper" mood from the mood button
    Then reading starts again from sentence 2 in the "Whisper" mood
    And the mood button says "Whisper"

  Scenario: Picking a voice restarts the current sentence
    Given a reading is under way at sentence 2 of 3
    When I open the voice picker
    And I pick the voice "amy (medium)"
    Then reading starts again from sentence 2 in the voice "amy (medium)"
    And the voice button says "amy (medium)"

  Scenario: The text starts at 25 px
    When the text "Good morning. Two emails need you." is selected
    Then the text is 25 px

  Scenario Outline: Ctrl with plus or minus sizes the text in 3 px steps between 20 and 40 px
    Given a reading is under way
    When I press "<key>" <times> times
    Then the text is <size> px

    Examples:
      | key               | times | size |
      | Control+=         | 1     | 28   |
      | Control+=         | 5     | 40   |
      | Control+=         | 6     | 40   |
      | Control+NumpadAdd | 1     | 28   |
      | Control+-         | 1     | 22   |
      | Control+-         | 2     | 20   |
      | Control+-         | 3     | 20   |

  Scenario: The text size is remembered
    Given a reading is under way
    When I press "Control+=" 1 times
    And I reload the window
    Then the text is 28 px

  Scenario: Scrolling stops following
    Given a long reading is under way
    When I scroll the text with the wheel
    Then I see "Follow the voice"
    When I click "Follow the voice"
    Then the lit line is back in view

  Scenario: The lit line is followed while reading
    Given a long reading is under way
    When a sentence below the window starts
    Then the lit line is about a third of the way down the text

  Scenario: Progress shows the sentences done
    Given a reading is under way at sentence 3 of 4
    Then the progress shows 2 of 4 sentences done

  Scenario: The end of the reading
    Given a reading is under way at sentence 3 of 3
    When the reading finishes
    Then the status says "Finished reading." and no word is lit
    And the progress line is dimmed

  Scenario Outline: After the end, reading again
    Given a reading is under way at sentence 3 of 3
    And the reading finishes
    When I <action>
    Then reading starts again from sentence <sentence>

    Examples:
      | action                     | sentence |
      | press "Space"              | 1        |
      | click the "Play" button    | 1        |
      | click a word in sentence 2 | 2        |

  Scenario: The voice fails
    Given a reading is under way at sentence 2 of 3
    When the voice fails with "can't reach the voice server at 127.0.0.1:10200: connection refused"
    Then the status says "can't reach the voice server at 127.0.0.1:10200: connection refused"
    And no word is lit

  Scenario: The shortcut pressed while the reader is open reads the new selection
    Given a reading is under way at sentence 2 of 3
    When the text "Back at noon." is selected while the reader is open
    Then reading starts again from sentence 1
    And I see 1 sentence
    And the status says "Sentence 1 of 1."

  Scenario Outline: Esc and Close stop and close
    Given a reading is under way
    When I <action>
    Then the reading is stopped and the window is dismissed

    Examples:
      | action                   |
      | press "Escape"           |
      | click the "Close" button |

  Scenario: Nothing selected
    Given the text "   " is selected
    Then I see the alert "Select some text first, then press the read-aloud shortcut."
    And nothing is read

  Scenario: Close after nothing was selected
    Given the text "   " is selected
    Then the "Close" button is focused
    When I click the "Close" button
    Then the window is dismissed
