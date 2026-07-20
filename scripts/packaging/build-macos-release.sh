#!/usr/bin/env bash
set -euo pipefail

printf '%s\n' \
  'The monolithic macOS release command is disabled.' \
  'Run desktop:package:macos:prepare, certify that exact candidate, then run desktop:package:macos:promote.' >&2
exit 1
