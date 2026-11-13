Feature: Tasks
  As a person with things to do
  I want to record tasks and mark them done
  So that I can keep track of my work

  Background:
    Given I open the app

  Scenario: A new visitor sees an empty list
    Then I see "No tasks yet."
    And the title field is empty

  Scenario: Adding a task
    When I add a task titled "  Buy milk  "
    Then the tasks are "Buy milk"
    And "Buy milk" is open
    And the title field is empty

  Scenario Outline: Title length boundaries
    When I add a task with a title of <length> characters
    Then the task is <outcome>

    Examples:
      | length | outcome  |
      | 0      | rejected |
      | 1      | added    |
      | 200    | added    |
      | 201    | rejected |

  Scenario: A whitespace-only title is rejected and kept for editing
    When I add a task titled "   "
    Then I see the error "Enter a title."
    And the title field contains "   "
    And I see "No tasks yet."

  Scenario: A title over 200 characters is rejected
    When I add a task with a title of 201 characters
    Then I see the error "Keep the title to 200 characters or fewer."

  Scenario: The error clears after a valid task is added
    When I add a task titled "   "
    And I add a task titled "valid"
    Then I see no error

  Scenario: Tasks are listed in creation order
    When I add a task titled "first"
    And I add a task titled "second"
    Then the tasks are "first, second"

  Scenario: Completing a task is final
    Given I added a task titled "finish me"
    When I complete "finish me"
    Then "finish me" is done
    And "finish me" cannot be completed again

  Scenario: Tasks survive a reload
    Given I added a task titled "keep me"
    And I added a task titled "done already"
    And I completed "done already"
    When I reload the app
    Then the tasks are "keep me, done already"
    And "keep me" is open
    And "done already" is done

  Scenario: Unreadable saved tasks are ignored
    Given the saved tasks are unreadable
    When I reload the app
    Then I see "No tasks yet."
