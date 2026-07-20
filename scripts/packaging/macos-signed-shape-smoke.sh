#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
app_path="${1:-/Applications/DesktopLab.app}"
cd "$repo_root"

npm run packaging:verify:macos-metadata -- --app "$app_path" --mode dev
bash scripts/packaging/macos-install-smoke.sh --dev-artifact --app "$app_path"
bash scripts/packaging/macos-runtime-ownership-smoke.sh --app "$app_path"
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test local_api_lifecycle
cargo test -p desktoplab-control-plane --test canonical_agent_tool_executor --test local_api_agent_terminal_execution
DESKTOPLAB_LIVE_KEYCHAIN_TEST=1 cargo test -p desktoplab-vault --test macos_keychain_live -- --ignored --nocapture
node scripts/packaging/audit-macos-entitlements.mjs --app "$app_path"

printf 'macOS signed-shape smoke passed for local API, Ollama ownership, terminal executors and Keychain.\n'
