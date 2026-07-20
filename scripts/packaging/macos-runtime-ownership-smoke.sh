#!/usr/bin/env bash
set -euo pipefail

APP_PATH="/Applications/DesktopLab.app"
app_pid=""
tmp_root=""

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

if [[ ! -d "$APP_PATH" ]]; then
  echo "FAIL: DesktopLab app not found at $APP_PATH" >&2
  exit 1
fi

cleanup() {
  if [[ -n "$app_pid" ]] && kill -0 "$app_pid" >/dev/null 2>&1; then
    kill "$app_pid" >/dev/null 2>&1 || true
    wait "$app_pid" >/dev/null 2>&1 || true
  fi
  if [[ -n "$tmp_root" ]]; then
    rm -rf "$tmp_root"
  fi
  if ! pgrep -x ollama >/dev/null 2>&1 && ! pgrep -x Ollama >/dev/null 2>&1 && [[ -d "/Applications/Ollama.app" ]]; then
    open -a Ollama >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

if ! command -v pgrep >/dev/null 2>&1; then
  echo "FAIL: pgrep is required" >&2
  exit 1
fi

if ! pgrep -x ollama >/dev/null 2>&1 && ! pgrep -x Ollama >/dev/null 2>&1; then
  if [[ -d "/Applications/Ollama.app" ]]; then
    open -a Ollama
    for _ in {1..20}; do
      if pgrep -x ollama >/dev/null 2>&1 || pgrep -x Ollama >/dev/null 2>&1; then
        break
      fi
      sleep 1
    done
  fi
fi

if ! pgrep -x ollama >/dev/null 2>&1 && ! pgrep -x Ollama >/dev/null 2>&1; then
  echo "SKIP: user-owned Ollama is not running and /Applications/Ollama.app is unavailable"
  exit 3
fi

before="$(pgrep -x ollama || pgrep -x Ollama)"
tmp_root="$(mktemp -d "${TMPDIR:-/tmp}/desktoplab-runtime-ownership.XXXXXX")"
app_data_dir="$tmp_root/app-data"
mkdir -p "$app_data_dir"
DESKTOPLAB_APP_DATA_DIR="$app_data_dir" "$APP_PATH/Contents/MacOS/desktoplab-desktop" >/tmp/desktoplab-runtime-ownership-smoke.log 2>&1 &
app_pid="$!"
sleep 5
kill "$app_pid" >/dev/null 2>&1 || true
wait "$app_pid" >/dev/null 2>&1 || true
app_pid=""
sleep 3

if ! pgrep -x ollama >/dev/null 2>&1 && ! pgrep -x Ollama >/dev/null 2>&1; then
  echo "FAIL: DesktopLab quit stopped user-owned Ollama" >&2
  exit 1
fi

after="$(pgrep -x ollama || pgrep -x Ollama)"
echo "PASS: user-owned Ollama survived DesktopLab quit"
echo "ollama_pids_before=$before"
echo "ollama_pids_after=$after"
