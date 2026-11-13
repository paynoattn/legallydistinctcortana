---
name: architecture-guard
description: Fixes dependency-rule violations reported by make arch through structural changes (dependency inversion, ports, module splits) and explains module structure. Use when make arch fails, when adding a new module or feature folder, or when asked how modules depend on each other.
---

# Architecture guard

The rules file is the spec (see the starter's README for where it lives). It is protected; you do not edit it to make a violation go away.

## Default rules

- Inside a feature: `http` → `application` → `domain`, and `infrastructure` → `application`/`domain`. `domain` imports nothing outside itself and the standard library.
- Only the composition root (`main`/`app` module) imports `infrastructure` to wire adapters into ports.
- Features import each other only through the other feature's public entry point, never its internals.
- No cycles at any level.

## Fixing a violation

Pick the first fix that applies:

1. **Wrong-way dependency** (domain or application imports infrastructure or a framework): define a port interface in `application` that describes what is needed, implement it in `infrastructure`, and inject it at the composition root.
2. **Cycle between features:** move the shared concept into a new, smaller module that both depend on, or invert one edge with an event or callback port.
3. **Reaching into internals:** expose what is needed from the feature's public entry point. If the needed surface is large, the boundary is wrong; report it rather than widening the interface.
4. **Module doing two jobs:** split it along the line the violation exposed.

Run `make arch` after each fix.

## Explaining structure

When asked about architecture, report: the modules, each one's one-line responsibility, its public entry point, and its allowed dependencies. Flag any module whose responsibility needs "and", and any shallow module. Keep it to one screen; a human reviews this between stories.

## Changing the rules

Proposing a new rule or layer is fine. Write the proposed rule and the reason in your reply, and let the human edit the protected rules file.
