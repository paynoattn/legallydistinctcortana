# Assistant (working name)

A free, open-source, local-first desktop assistant for Windows 11. Hold a key and talk to type anywhere. Select text and hear it read aloud in the mood you pick. Get reminded, briefed on your mail and calendar, and shown the news and projects you care about. It talks back, it acts when you ask (and confirms first), and it syncs between your own computers end to end, with no account and no cloud of ours.

> The name is not final. Five brand concepts are in [`docs/brand/`](docs/brand/) (recommended: **Lampion**). The code is brand-neutral on purpose ([ADR 0011](docs/architecture/adr/0011-brand-neutral-code.md)).

## Status

Pre-alpha: architecture and plan are written, the gated repository is in place, and feature work starts at issue [#002](docs/product/mvp-plan.md#issue-index).

| Read                                                                | For                                                                                                   |
| ------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| [Product vision](docs/product/vision.md)                            | What it is, for whom, and the principles                                                              |
| [MVP plan](docs/product/mvp-plan.md)                                | Scope, milestones, critical path, risks, and the issue index                                          |
| [Architecture](docs/architecture/overview.md)                       | Processes, crates, data, and the component designs                                                    |
| [Decisions](docs/architecture/adr/)                                 | ADRs                                                                                                  |
| [Issues](docs/issues/)                                              | One file per story, ready for `gh issue create --body-file`                                           |
| [Brand](docs/brand/) and [design artboards](docs/design/artboards/) | Name and logo concepts; overlay, read-aloud menu, and Today mockups                                   |
| [Engineering gates](docs/engineering/gate-contract.md)              | What `make check` and `make gauntlet` enforce, and why ([principles](docs/engineering/principles.md)) |

## Develop

Requirements: Rust (pinned by `rust-toolchain.toml`), Node (see `.nvmrc`), pnpm, GNU make, jq, and on Windows, Git Bash and WebView2. On Windows: `winget install ezwinports.make jqlang.jq`.

```sh
git init            # hooks install only at a repository root
make setup          # toolchain, cargo gate tools, pnpm deps, Chromium, git hooks
make dev            # desktop app with hot reload
make check          # the gate: format, lint, types, architecture, coverage ≥ 90%, CRAP ≤ 6, acceptance
make gauntlet       # check + 100% mutation score + scripted QA
```

Every gate has a Rust (`-rs`) and a UI (`-ui`) half, for example `make lint-ui` or `make coverage-rs`. `make help` lists them.

### Try read aloud

The first working feature: select text in any app, press **Ctrl+Alt+R**, and pick a mood (click, arrow keys, or 1–9). It speaks through a [Wyoming](https://github.com/rhasspy/wyoming) Piper server you run yourself, the same one `voice.py` uses:

```sh
# on any machine on your LAN or tailnet
docker run -p 10200:10200 rhasspy/wyoming-piper --voice en_US-amy-medium

# on the Windows PC
VOICE_HOST=my-mac make dev   # VOICE_TTS_PORT (default 10200), VOICE_TTS_VOICE, READ_ALOUD_SHORTCUT
```

The moods are approximated with speed, loudness, and pauses until the expressive voice model lands. The manual test plan is in [`qa/procedures/read-aloud.md`](qa/procedures/read-aloud.md).

Contributors and coding agents start with [`AGENTS.md`](AGENTS.md). Stories go through the `gauntlet` skill (specifier → coder → cleaner → hardener → qa), and the agent hooks stop an agent from finishing while `make check` is red.

## License

MIT OR Apache-2.0, pending the owner's decision ([ADR 0010](docs/architecture/adr/0010-license.md)).
