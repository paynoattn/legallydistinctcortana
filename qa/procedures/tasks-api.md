# Tasks

Preconditions: the app is running and reachable at its base URL. Other tasks may already exist.

1. `GET /health` → Expect: `200` with body `{"status":"ok"}`.
2. `POST /tasks` with `{"title":"  QA first  "}` (with a unique suffix) → Expect: `201`, title trimmed, `done` is `false`, a non-empty `id`.
3. `POST /tasks` with `{"title":"QA second"}` (same suffix) → Expect: `201`.
4. `GET /tasks` → Expect: `200`, and among the tasks with your suffix, the first comes before the second.
5. `GET /tasks/{first id}` → Expect: `200` with the same task as step 2.
6. `POST /tasks/{first id}/complete` → Expect: `200`, `done` is `true`.
7. `POST /tasks/{first id}/complete` again → Expect: `200`, still `done: true`, same id and title.
8. `GET /tasks/does-not-exist` → Expect: `404`.
9. `POST /tasks/does-not-exist/complete` → Expect: `404`.
10. `POST /tasks` with a whitespace-only title → Expect: `400`, nothing created.
11. `POST /tasks` with a 200-character title → Expect: `201`. With 201 characters → Expect: `400`.
12. `POST /tasks` with `{}` → Expect: `400`.
    Cleanup: none. Storage is in memory and there is no delete endpoint; restarting the app clears it.
