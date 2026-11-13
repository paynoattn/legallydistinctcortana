# Tasks

Preconditions: the built app is served and `/` loads. The browser profile is fresh (no saved tasks).

1. Open `/` → Expect: the heading "Tasks", the text "No tasks yet.", and an empty "Title" field.
2. Type `  QA: buy milk  ` into "Title" and press Enter → Expect: one task "QA: buy milk", unchecked; the field is empty.
3. Add `QA: second` → Expect: the tasks read "QA: buy milk", "QA: second", in that order.
4. Click Add with an empty field → Expect: the alert "Enter a title." and still two tasks.
5. Add a 201-character title → Expect: the alert "Keep the title to 200 characters or fewer." and still two tasks.
6. Add a 200-character title → Expect: the alert disappears and there are three tasks.
7. Tick "QA: buy milk" → Expect: it is checked and disabled (completing is final).
8. Reload the page → Expect: the same three tasks, with "QA: buy milk" still checked and the others unchecked.
9. Add 100 more tasks one after another → Expect: the page never blocks the main thread long enough to drop frames (no long animation frame of 50 ms or more), and 103 tasks are listed.

Cleanup: close the browser profile. Tasks live in that profile's `localStorage`, so a fresh profile starts empty.
