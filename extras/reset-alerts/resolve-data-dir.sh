#!/bin/bash
set -euo pipefail
user_home="${1:?User home is required}"
store="$user_home/Library/Application Support/com.ccswitch.desktop/app_paths.json"
configured=""
if [ -f "$store" ]; then
  configured="$(/usr/bin/plutil -extract app_config_dir_override raw -expect string -o - "$store" 2>/dev/null || true)"
fi
configured="${configured#"${configured%%[![:space:]]*}"}"
configured="${configured%"${configured##*[![:space:]]}"}"
case "$configured" in
  '~') configured="$user_home" ;;
  '~/'*|'~\'*) configured="$user_home/${configured:2}" ;;
esac
if [ -n "$configured" ] && [ -d "$configured" ]; then
  printf '%s\n' "$configured"
else
  printf '%s\n' "$user_home/.cc-switch"
fi
