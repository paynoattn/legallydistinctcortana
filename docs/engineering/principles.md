# Principles

Distilled from [Uncle Bob on Software Fundamentals in the Age of AI](https://www.youtube.com/watch?v=zcLPGC-tvgk) (Matt Pocock, live stream; transcript in [`sources/`](sources/uncle-bob-zcLPGC-tvgk.transcript.txt)), _Clean Code_, and Ousterhout's _A Philosophy of Software Design_. Every principle below names the deterministic gate that enforces it. If a principle has no gate, it is a guideline, and agents will treat it like one.

## 1. Gates over guidance

> "They treat those rules in the Pirates of the Caribbean sense. They're more like guidelines." [00:13:28]

Long instruction files sink into the middle of the context window, where they get ignored ("lost in the middle"). Deterministic tools do not fade with context length.

- Keep `AGENTS.md` to the minimum: commands, the gate contract, where the skills live.
- When a review comment repeats, turn it into a lint rule, type, test, or architecture rule instead of a new sentence in `AGENTS.md`. See the `encode-rule` skill.
- Warnings are not gates. Every linter runs at zero tolerance (`--max-warnings=0`, `-D warnings`, ruff errors only).
- Suppressions must carry a reason (`eslint-disable` needs a description, `#[allow]` needs `reason = "..."`, `# noqa` must be specific, and unused suppressions fail).
- No comments, because agents copy them and use them to justify workarounds instead of fixing the code (from [poteto](poteto-react.md)). Only tool directives survive, plus doc comments where Rust's `missing_docs` requires them. Names, types, tests, and lint rules carry the meaning.
- No unchecked casts (`as` in TypeScript and Rust, `typing.cast` in Python). Narrow with a guard or parse at the boundary.

| Gate                               | Enforced by                                                                               |
| ---------------------------------- | ----------------------------------------------------------------------------------------- |
| Agent cannot stop with red gates   | `.cursor/hooks/stop-gate.sh` (Cursor `stop`, Claude Code `Stop`)                          |
| Agent cannot silently loosen gates | `.cursor/hooks/protect-gates.sh` asks a human before edits to files in `.gates-protected` |

## 2. CRAP: complexity is only tolerable when it is tested

> "For a human I would keep CRAP numbers below four... for the agents I've set this at six." [00:32:52]

`CRAP(fn) = complexity² × (1 − coverage)³ + complexity`, where complexity is cyclomatic complexity and coverage is the fraction of the function's lines executed by tests. A fully covered function scores exactly its complexity, and an untested one scores roughly complexity² + complexity.

- Gate: `make crap` fails if any function scores above **6**.
- Backstop: the linter's complexity rule is also set to 6, so feedback arrives before tests run.

## 3. Mutation testing: coverage that proves something

> "If it doesn't fail, well, that's a surviving mutant and it must be killed." [00:07:21]

Line coverage only says code ran. Mutation testing flips operators and constants and expects a test to fail for each change.

- Gate: `make mutate` (Stryker / mutmut / cargo-mutants) with a minimum mutation score.
- Run by the hardener stage, not on every save. It is slow, and that is acceptable.

## 4. Architecture is a checked spec, not a diagram

> "A nice tight little specification file that the agents cannot violate... usually by inverting a dependency or inserting an interface or splitting a module." [00:27:44]

- Gate: `make arch` (dependency-cruiser / import-linter / `cargo xtask arch`).
- Default layering inside each feature: `domain` ← `application` ← `infrastructure` and `http`; only the composition root wires infrastructure into application. No cycles. Features do not reach into each other's internals.
- Fixes are structural (invert the dependency, add a port, split the module), never "add an exception to the rules file".

## 5. Deep modules, small interfaces

> "The models pay attention to interface names... It can allow them to not read the code beneath them." [00:30:37]

Agents, like humans, work better inside a well-partitioned module that has a consistent topic (Bob's "coffee and soap opera" point). Prefer few, narrow, well-named public entry points hiding substantial behavior. Agents also read tests to learn behavior, so tests are documentation.

- Gates: `make arch` (public entry points only), lint rules on file and function size, max params, and barrel discipline.

## 6. Values, not human disciplines

> "It's probably a mistake to impose a human discipline on an agent. It is not a mistake to impose human values." [00:35:04]

Do not force line-by-line TDD on agents. Do demand the outcome TDD buys: every behavior has a test, the tests can fail (mutation), and the code stays simple (CRAP). Tune thresholds for agents (CRAP 6, not 4) instead of dropping them.

## 7. Short-lived, single-purpose agents (the gauntlet)

> "Agents are born, do the task, and die so that the next one comes in with a clean context." [00:19:57]

Each stage runs in a fresh context with one job, which keeps the context's trajectory consistent:

1. **specifier**: human story → Gherkin acceptance feature + human-perspective QA procedure.
2. **coder**: makes the feature and unit tests pass. Not pretty, just working.
3. **cleaner**: CRAP, lint, architecture, code-smell review. Behavior-preserving refactors only.
4. **hardener**: mutation testing, kills surviving mutants, and is merciless about it.
5. **qa**: turns the QA procedure into an executable script against the running system.

See the `gauntlet` skill for orchestration.

## 8. Small increments over big upfront specs

> "The cost of change has plummeted... why wouldn't you just fiddle fiddle fiddle until it looks right?" [00:41:11]

Feed the gauntlet one story or two, then look at the architecture, then continue. Specs are ephemeral. The durable specification is the code plus its gates: features, tests, and architecture rules.

## 9. Fundamentals still matter

> "The rules you throw away are the ones you're going to pick up off the floor in a year." [00:55:39]

Clean code is how complexity is organized so it can be conceived, by people and by models. Agents hit the same wall humans do, just at a different threshold: when code gets messy they thrash, fixing one thing and breaking another.
