#!/usr/bin/env bash
# Executable form of qa/procedures/tasks.md. Needs curl and jq, and QA_BASE_URL.
# Idempotent: every run uses its own title suffix and never assumes an empty store.
set -euo pipefail

base="${QA_BASE_URL:?set QA_BASE_URL, e.g. http://127.0.0.1:3000}"
suffix="qa-$$-$RANDOM"
body_file="$(mktemp)"
trap 'rm -f "$body_file"' EXIT

status=""
body=""

request() {
  local method="$1" path="$2" data="${3:-}"
  local curl_args=(-sS -o "$body_file" -w '%{http_code}' -X "$method")
  if [[ -n "$data" ]]; then
    curl_args+=(-H 'content-type: application/json' --data "$data")
  fi
  status="$(curl "${curl_args[@]}" "$base$path")"
  body="$(cat "$body_file")"
}

fail() {
  echo "step $1: FAILED: $2" >&2
  exit 1
}

expect_status() {
  [[ "$status" == "$2" ]] || fail "$1" "expected status $2, got $status (body: $body)"
}

# expect_json STEP FILTER EXPECTED: jq FILTER applied to the body must print EXPECTED.
expect_json() {
  local actual
  actual="$(jq -c --arg suffix "$suffix" "$2" <<<"$body" 2>&1)" || fail "$1" "body is not JSON: $body"
  [[ "$actual" == "$3" ]] || fail "$1" "expected $2 to be $3, got $actual"
}

ok() { echo "step $1: ok"; }

title_of_length() { printf 'a%.0s' $(seq 1 "$1"); }

request GET /health
expect_status 1 200
expect_json 1 . '{"status":"ok"}'
ok 1

request POST /tasks "{\"title\":\"  QA first $suffix  \"}"
expect_status 2 201
expect_json 2 '[.title == "QA first \($suffix)", .done, (.id | type == "string" and length > 0)]' '[true,false,true]'
first="$body"
first_id="$(jq -r .id <<<"$first")"
ok 2

request POST /tasks "{\"title\":\"QA second $suffix\"}"
expect_status 3 201
second_id="$(jq -r .id <<<"$body")"
ok 3

request GET /tasks
expect_status 4 200
expect_json 4 '[.[] | select(.title | endswith($suffix)) | .id]' "[\"$first_id\",\"$second_id\"]"
ok 4

request GET "/tasks/$first_id"
expect_status 5 200
expect_json 5 . "$(jq -c . <<<"$first")"
ok 5

request POST "/tasks/$first_id/complete"
expect_status 6 200
expect_json 6 '[.id, .done]' "[\"$first_id\",true]"
ok 6

request POST "/tasks/$first_id/complete"
expect_status 7 200
expect_json 7 . "$(jq -c '.done = true' <<<"$first")"
ok 7

request GET "/tasks/does-not-exist-$suffix"
expect_status 8 404
ok 8

request POST "/tasks/does-not-exist-$suffix/complete"
expect_status 9 404
ok 9

request GET /tasks
count_before="$(jq length <<<"$body")"
request POST /tasks '{"title":"   "}'
expect_status 10 400
request GET /tasks
expect_json 10 length "$count_before"
ok 10

request POST /tasks "{\"title\":\"$(title_of_length 200)\"}"
expect_status 11 201
request POST /tasks "{\"title\":\"$(title_of_length 201)\"}"
expect_status 11 400
ok 11

request POST /tasks '{}'
expect_status 12 400
ok 12
