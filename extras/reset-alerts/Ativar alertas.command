#!/bin/bash
set -euo pipefail
umask 077
script_dir="$(cd "$(dirname "$0")" && pwd)"
app='/Applications/CC Switch.app'
helper='/Applications/CC Switch Reset Alerts.app'
label='local.henrique.ccswitch-reset-alerts'
if [ ! -f "$app/Contents/Resources/reset-alerts-v1" ] || [ ! -x "$helper/Contents/MacOS/ResetAlerts" ]; then
  echo 'Copie os dois aplicativos deste pacote para Aplicativos antes de ativar.'
  exit 1
fi
if /usr/bin/pgrep -x cc-switch >/dev/null; then
  echo 'Encerre o CC Switch pelo menu Sair e execute este arquivo novamente.'
  exit 1
fi
config_dir="$(/bin/bash "$script_dir/resolve-data-dir.sh" "$HOME")"
agent_dir="$HOME/Library/LaunchAgents"
mkdir -p "$config_dir/logs" "$agent_dir"
config_tmp="$(mktemp "$config_dir/.reset-alerts.XXXXXX")"
agent_tmp="$(mktemp "$agent_dir/.reset-alerts.XXXXXX")"
trap 'rm -f "$config_tmp" "$agent_tmp"' EXIT
source "$script_dir/configuration.sh"
write_configuration "$config_tmp" "$helper/Contents/MacOS/ResetAlerts"
/usr/bin/plutil -create xml1 "$agent_tmp"
/usr/bin/plutil -insert Label -string "$label" "$agent_tmp"
/usr/bin/plutil -insert ProgramArguments -json '["/Applications/CC Switch.app/Contents/MacOS/cc-switch"]' "$agent_tmp"
/usr/bin/plutil -insert RunAtLoad -bool YES "$agent_tmp"
/usr/bin/plutil -insert KeepAlive -json '{"SuccessfulExit":false}' "$agent_tmp"
/usr/bin/plutil -insert ThrottleInterval -integer 30 "$agent_tmp"
/usr/bin/plutil -insert ProcessType -string Interactive "$agent_tmp"
/usr/bin/plutil -insert StandardOutPath -string "$config_dir/logs/reset-alerts-launch.log" "$agent_tmp"
/usr/bin/plutil -insert StandardErrorPath -string "$config_dir/logs/reset-alerts-launch-error.log" "$agent_tmp"
if /bin/launchctl print "gui/$(id -u)/$label" >/dev/null 2>&1; then
  /bin/launchctl bootout "gui/$(id -u)/$label"
fi
mv "$config_tmp" "$config_dir/reset-alerts.json"
mv "$agent_tmp" "$agent_dir/$label.plist"
/bin/launchctl bootstrap "gui/$(id -u)" "$agent_dir/$label.plist"
/usr/bin/open "$helper" --args --authorize
echo 'Monitor ativado. Autorize as notificações do CC Switch Reset Alerts no macOS.'
echo "Diagnóstico: $config_dir/reset-alerts-status.json"
