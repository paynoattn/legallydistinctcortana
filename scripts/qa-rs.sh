#!/usr/bin/env bash
# Boots the app on a free port, waits for /health, runs every qa/scripts/core/* with
# QA_BASE_URL set, then stops the app with SIGTERM and checks it shut down cleanly.
# The app is killed on any exit path.
set -euo pipefail
cd "$(dirname "$0")/.."

boot_timeout="${QA_BOOT_TIMEOUT:-60}"
cargo build --package app --locked --quiet
port="$(cargo xtask free-port)"
export QA_BASE_URL="http://127.0.0.1:$port"
log="target/qa-app.log"

PORT="$port" HOST=127.0.0.1 target/debug/app >"$log" 2>&1 &
pid=$!
trap 'kill "$pid" 2>/dev/null || true' EXIT

deadline=$((SECONDS + boot_timeout))
until curl -fsS "$QA_BASE_URL/health" >/dev/null 2>&1; do
  if ! kill -0 "$pid" 2>/dev/null; then
    echo "qa: app exited during startup. Log:" >&2
    cat "$log" >&2
    exit 1
  fi
  if ((SECONDS >= deadline)); then
    echo "qa: /health did not answer within ${boot_timeout}s. Log:" >&2
    cat "$log" >&2
    exit 1
  fi
  sleep 0.1
done
echo "qa: app up at $QA_BASE_URL"

failed=()
for script in qa/scripts/core/*; do
  [[ -f "$script" ]] || continue
  echo "== $script"
  if ! "$script"; then
    failed+=("$script")
  fi
done

kill -TERM "$pid" 2>/dev/null || true
if ! wait "$pid"; then
  echo "qa: app did not shut down cleanly on SIGTERM. Log:" >&2
  cat "$log" >&2
  failed+=("graceful shutdown")
fi

if ((${#failed[@]} > 0)); then
  echo "qa: FAILED: ${failed[*]}" >&2
  exit 1
fi
echo "qa: ok, all scripts passed and the app shut down cleanly"
