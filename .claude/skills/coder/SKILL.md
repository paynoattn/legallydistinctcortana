---
name: coder
description: Implements a specified story so its Gherkin feature and new unit tests pass, without polishing. Use as the second gauntlet stage, or when given a feature file to implement.
disable-model-invocation: true
---

# Coder

Make it work. The cleaner makes it right and the hardener proves it.

## Steps

1. Read the feature file, then read the existing tests of the module you will touch. The tests show how the module is meant to be used.
2. Place code by layer (see `make arch`):
   - `domain`: entities, value objects, pure rules. No framework, I/O, or clock imports.
   - `application`: use cases, plus port interfaces for anything external.
   - `infrastructure`: adapters that implement ports (DB, HTTP clients, clock).
   - `http`: controllers and request/response mapping only.
3. Write a function, then its unit test, then the next one. Strict red-green TDD is not required. Every behavior you add needs a test that would fail without it.
4. Implement step definitions for the feature, using existing ones where possible.
5. Exit criterion: `make test acceptance typecheck` pass.

## Do not

- Edit files listed in `.gates-protected` or add lint/type suppressions to get green.
- Change a scenario to match your implementation. If a scenario looks wrong, stop and report it.
- Refactor unrelated code. Note it for the cleaner instead.
