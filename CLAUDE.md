# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

@AGENTS.md

## Claude Code specifics

- The `Stop` hook runs `make check` when the working tree has changes and sends you back with the failures (up to 5 times). The `PreToolUse` hook asks the user before you edit anything matching `.gates-protected`. Both live in `scripts/agent-hooks/` and need `jq` and `make` on PATH.
- On Windows, GNU make cannot exec `node_modules/.bin` shims, so the Makefile switches to `pnpm exec`. Call UI tools the same way (`pnpm exec eslint`, not `node_modules/.bin/eslint`).
- Before designing a feature, read its issue in `docs/issues/` and the architecture doc it links. Decisions are in `docs/architecture/adr/`; changing one means writing a new ADR.
