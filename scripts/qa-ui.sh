#!/usr/bin/env bash
# Serves dist/ with vite preview on a free port, waits for /, runs every qa/scripts/ui/*.ts, always stops the server.
set -euo pipefail
cd "$(dirname "$0")/.."

timeout_s="${QA_BOOT_TIMEOUT:-30}"
port="$(node -e 'const s=require("node:net").createServer();s.listen(0,"127.0.0.1",()=>{console.log(s.address().port);s.close()})')"
export QA_BASE_URL="http://127.0.0.1:${port}"
log="$(mktemp)"

node_modules/.bin/vite preview --host 127.0.0.1 --port "$port" --strictPort >"$log" 2>&1 &
app_pid=$!
trap 'kill "$app_pid" 2>/dev/null || true; wait "$app_pid" 2>/dev/null || true; rm -f "$log"' EXIT

deadline=$((SECONDS + timeout_s))
until curl -fsS "$QA_BASE_URL/" >/dev/null 2>&1; do
  if ! kill -0 "$app_pid" 2>/dev/null || ((SECONDS >= deadline)); then
    echo "qa: app did not respond at $QA_BASE_URL within ${timeout_s}s. Server output:" >&2
    cat "$log" >&2
    exit 1
  fi
  sleep 0.2
done

status=0
for script in qa/scripts/ui/*.ts; do
  echo "qa: $script against $QA_BASE_URL"
  node "$script" || status=1
done
exit "$status"
