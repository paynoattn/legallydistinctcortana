Feature: Tasks
  A client keeps a to-do list over HTTP.

  Scenario: The service reports that it is healthy
    When I send GET /health
    Then the response status is 200
    And the response body is {"status":"ok"}

  Scenario: Creating a task returns it open
    When I create a task titled "Buy milk"
    Then the response status is 201
    And the response is an open task titled "Buy milk"

  Scenario: Titles are trimmed
    When I create a task titled "   Buy milk   "
    Then the response status is 201
    And the response is an open task titled "Buy milk"

  Scenario Outline: Title length boundaries
    When I create a task with a title of <length> <kind> characters
    Then the response status is <status>

    Examples:
      | length | kind   | status |
      | 0      | letter | 400    |
      | 1      | letter | 201    |
      | 200    | letter | 201    |
      | 201    | letter | 400    |
      | 3      | space  | 400    |

  Scenario: A request without a title is rejected
    When I send POST /tasks with body {"name":"Buy milk"}
    Then the response status is 400

  Scenario: Tasks are listed in creation order
    Given a task titled "First" exists
    And a task titled "Second" exists
    When I send GET /tasks
    Then the response status is 200
    And the task titles are "First, Second"

  Scenario: The task list starts empty
    When I send GET /tasks
    Then the response status is 200
    And the response body is []

  Scenario: Fetching a task by id
    Given a task titled "Find me" exists
    When I fetch that task
    Then the response status is 200
    And the response is an open task titled "Find me"

  Scenario: Fetching an unknown task
    When I send GET /tasks/does-not-exist
    Then the response status is 404

  Scenario: Completing a task
    Given a task titled "Finish me" exists
    When I complete that task
    Then the response status is 200
    And the response is a done task titled "Finish me"

  Scenario: Completing a task twice is idempotent
    Given a task titled "Finish me" exists
    When I complete that task
    And I complete that task
    Then the response status is 200
    And the response is a done task titled "Finish me"

  Scenario: Completing an unknown task
    When I send POST /tasks/does-not-exist/complete
    Then the response status is 404
