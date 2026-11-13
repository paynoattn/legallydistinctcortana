# Gate contract: docs/engineering/gate-contract.md. Every target exits non-zero on failure.
# Each gate runs the Rust half (`-rs`) and the UI half (`-ui`); call a half directly for
# faster feedback while you work.
SHELL := /bin/bash
.DEFAULT_GOAL := help

CRAP_MAX ?= 6
COVERAGE_MIN ?= 90
MUTATION_MIN ?= 100
BUNDLE_BUDGET_KB ?= 80
SCOPE ?= full
BASE ?= origin/main

# GNU make on Windows execs simple commands directly (no shell), and node_modules/.bin
# entries are shell shims Windows cannot exec; pnpm exec resolves the .cmd shims instead.
ifeq ($(OS),Windows_NT)
BIN := pnpm exec$(eval) 
else
BIN := node_modules/.bin/
endif
LCOV := target/lcov.info
# Pinned so every machine and CI runs the same gate tools.
CARGO_TOOLS := cargo-llvm-cov@0.9.1 cargo-mutants@27.1.0 cargo-deny@0.20.2
# Wiring only, exercised end to end by `make qa`: the headless binary's main.rs and the Tauri
# shell. Logic never lives in either. Matches both / and \ so it works on Windows too.
COVERAGE_IGNORE := crates[/\\]app[/\\]src[/\\]main\.rs|crates[/\\]desktop[/\\]

export CRAP_MAX BUNDLE_BUDGET_KB BASE MUTATION_MIN

.PHONY: help setup fmt fmt-check lint lint-rs lint-ui typecheck typecheck-rs typecheck-ui \
	test test-rs test-ui acceptance acceptance-rs acceptance-ui coverage coverage-rs coverage-ui \
	crap crap-rs crap-ui arch arch-rs arch-ui mutate mutate-rs mutate-ui qa qa-rs qa-ui \
	build dev bundle check check-rs check-ui gauntlet clean

help: ## List targets
	@grep -E '^[a-z-]+:.*## ' $(MAKEFILE_LIST) | awk -F ':.*## ' '{printf "  %-14s %s\n", $$1, $$2}'
	@echo "Thresholds: CRAP_MAX=$(CRAP_MAX) COVERAGE_MIN=$(COVERAGE_MIN) MUTATION_MIN=$(MUTATION_MIN) BUNDLE_BUDGET_KB=$(BUNDLE_BUDGET_KB)"
	@echo "Scoped mutation testing: make mutate SCOPE=changed [BASE=origin/main]"

setup: ## Install toolchains, deps, Chromium, cargo gate tools; point git at .githooks
	rustup toolchain install --no-self-update
	@for tool in $(CARGO_TOOLS); do \
	  name="$${tool%@*}"; version="$${tool#*@}"; \
	  if [ "$$(cargo "$${name#cargo-}" --version 2>/dev/null)" = "$$name $$version" ]; then \
	    echo "$$name $$version: installed"; \
	  else \
	    cargo install --locked "$$tool" || exit 1; \
	  fi; \
	done
	pnpm install --frozen-lockfile
	$(BIN)playwright install chromium --only-shell
	@if [ "$$(git rev-parse --show-toplevel 2>/dev/null)" = "$$(pwd -P)" ]; then \
	  git config core.hooksPath .githooks && echo "git hooks: .githooks"; \
	else \
	  echo "git hooks: skipped (not the root of a git repo)"; \
	fi

fmt: ## Rewrite formatting (rustfmt + prettier)
	cargo fmt --all
	$(BIN)prettier --write --log-level warn .

fmt-check: ## Fail on unformatted files
	cargo fmt --all -- --check
	$(BIN)prettier --check --log-level warn .

lint: lint-rs lint-ui ## All linters, zero warnings

lint-rs: ## clippy (pedantic + restriction set), no comments, cargo-deny supply chain
	cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
	cargo xtask comments
	cargo deny --locked check

lint-ui: ## ESLint (zero warnings) and knip (unused files, exports, deps)
	$(BIN)eslint . --max-warnings=0
	$(BIN)knip

typecheck: typecheck-rs typecheck-ui ## Compilers in strictest mode

typecheck-rs:
	cargo check --workspace --all-targets --all-features --locked
	RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked --quiet

typecheck-ui:
	$(BIN)tsc --noEmit -p src/tsconfig.json
	$(BIN)tsc --noEmit -p tsconfig.json

test: test-rs test-ui ## Unit tests

test-rs:
	cargo test --workspace --lib --bins --locked

test-ui:
	$(BIN)vitest run

acceptance: acceptance-rs acceptance-ui ## Gherkin: features/core (cucumber-rs), features/ui (cucumber-js + Playwright)

acceptance-rs:
	cargo test --package app --test acceptance --locked

acceptance-ui:
	$(BIN)cucumber-js

coverage: coverage-rs coverage-ui ## Unit tests with coverage thresholds

coverage-rs:
	cargo llvm-cov clean --workspace
	cargo llvm-cov --no-report --workspace --exclude xtask --lib --bins --locked
	cargo llvm-cov report --ignore-filename-regex '$(COVERAGE_IGNORE)' \
	  --fail-under-lines $(COVERAGE_MIN) --fail-under-regions $(COVERAGE_MIN) --fail-under-functions $(COVERAGE_MIN)
	cargo llvm-cov report --ignore-filename-regex '$(COVERAGE_IGNORE)' --lcov --output-path $(LCOV)

coverage-ui:
	rm -rf coverage
	$(BIN)vitest run --coverage

crap: crap-rs crap-ui ## Per-function CRAP from coverage + complexity (fails above CRAP_MAX)

crap-rs: coverage-rs
	cargo xtask crap --lcov $(LCOV) --max $(CRAP_MAX)

crap-ui: coverage-ui
	node scripts/crap.ts

arch: arch-rs arch-ui ## Dependency rules: arch.toml (crates), .dependency-cruiser.cjs (src)

arch-rs:
	cargo xtask arch --rules arch.toml

arch-ui:
	$(BIN)depcruise src features qa scripts --config .dependency-cruiser.cjs --output-type err-long

mutate: mutate-rs mutate-ui ## Mutation testing; SCOPE=changed mutates only your diff vs BASE

mutate-rs:
	SCOPE=$(subst full,all,$(SCOPE)) bash scripts/mutate-rs.sh

mutate-ui:
	SCOPE=$(SCOPE) bash scripts/mutate-ui.sh

bundle: ## Production UI build with React Compiler (fails on any bail-out), then the bundle budget
	rm -rf dist
	$(BIN)vite build --logLevel warn
	node scripts/bundle-budget.ts

build: bundle ## Installable desktop app (Tauri bundle) after the UI build and budget
	$(BIN)tauri build

dev: ## Desktop app with hot reload (Tauri + Vite)
	$(BIN)tauri dev

qa: qa-rs qa-ui ## Boot the headless core and the built UI, run qa/scripts/{core,ui}

qa-rs:
	bash scripts/qa-rs.sh

qa-ui: bundle
	bash scripts/qa-ui.sh

check-rs: lint-rs typecheck-rs arch-rs crap-rs acceptance-rs ## Rust half of check (crap runs coverage)

check-ui: lint-ui typecheck-ui arch-ui crap-ui acceptance-ui ## UI half of check (crap runs coverage)

check: fmt-check check-rs check-ui ## The gate: stop hook, pre-push, CI

gauntlet: check mutate qa ## check + mutation + QA

clean: ## Remove build, coverage, and mutation output
	rm -rf dist coverage reports .stryker-tmp mutants.out mutants.out.old
	cargo clean
