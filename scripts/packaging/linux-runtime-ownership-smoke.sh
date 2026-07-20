#!/usr/bin/env bash
set -euo pipefail

APP_PATH=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --app)
      APP_PATH="${2:?missing app path}"
      shift 2
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

if [[ -z "$APP_PATH" || ! -x "$APP_PATH" ]]; then
  echo "FAIL: pass --app with an executable packaged DesktopLab binary" >&2
  exit 1
fi

if ! command -v pgrep >/dev/null 2>&1; then
  echo "FAIL: pgrep is required" >&2
  exit 1
fi

if ! pgrep -x ollama >/dev/null 2>&1; then
  echo "SKIP: user-owned Ollama is not running on this Linux host"
  exit 3
fi

before="$(pgrep -x ollama)"
"$APP_PATH" >/tmp/desktoplab-runtime-ownership-smoke.log 2>&1 &
app_pid=$!
sleep 5
kill "$app_pid" >/dev/null 2>&1 || true
wait "$app_pid" >/dev/null 2>&1 || true
sleep 2

if ! pgrep -x ollama >/dev/null 2>&1; then
  echo "FAIL: DesktopLab shutdown stopped user-owned Ollama" >&2
  exit 1
fi

after="$(pgrep -x ollama)"
echo "PASS: user-owned Ollama survived DesktopLab shutdown"
echo "ollama_pids_before=$before"
echo "ollama_pids_after=$after"
