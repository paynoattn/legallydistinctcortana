#!/usr/bin/env bash
# Mutation testing with a minimum score. cargo-mutants fails on any surviving mutant and
# has no threshold of its own, so this runs it, then scores mutants.out/outcomes.json.
#   SCOPE=all      every mutant (default)
#   SCOPE=changed  only code changed since BASE (default origin/main), including
#                  uncommitted and untracked .rs files
set -euo pipefail
cd "$(dirname "$0")/.."

scope="${SCOPE:-all}"
base="${BASE:-origin/main}"
min="${MUTATION_MIN:-100}"
args=()

if [[ "$scope" == changed ]]; then
  if git rev-parse --verify --quiet "$base^{commit}" >/dev/null; then
    mkdir -p target
    diff_file="target/mutants.diff"
    git diff --relative "$base" -- . >"$diff_file"
    while IFS= read -r untracked; do
      git diff --no-index -- /dev/null "$untracked" >>"$diff_file" || true
    done < <(git ls-files --others --exclude-standard -- '*.rs')
    if [[ ! -s "$diff_file" ]]; then
      echo "mutate: ok, no changes since $base"
      exit 0
    fi
    echo "mutate: testing code changed since $base"
    args+=(--in-diff "$diff_file")
  else
    echo "mutate: BASE $base not found (shallow clone or no remote?); running the full set"
  fi
elif [[ "$scope" != all ]]; then
  echo "mutate: SCOPE must be 'all' or 'changed', got '$scope'" >&2
  exit 2
fi

rm -rf mutants.out
status=0
cargo mutants --no-shuffle "${args[@]+"${args[@]}"}" || status=$?
# 0 = all caught, 2 = some missed, 3 = some timed out: score them. Anything else
# (1 usage error, 4 baseline tests failed, ...) is a real failure.
case "$status" in
  0 | 2 | 3) ;;
  *)
    echo "mutate: cargo mutants failed with exit code $status (see mutants.out/debug.log)" >&2
    exit "$status"
    ;;
esac

if [[ ! -f mutants.out/outcomes.json ]]; then
  echo "mutate: ok, no mutants generated for this scope"
  exit 0
fi
cargo xtask mutation-score --outcomes mutants.out/outcomes.json --min "$min"
