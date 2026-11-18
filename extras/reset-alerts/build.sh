#!/bin/bash
set -euo pipefail
script_dir="$(cd "$(dirname "$0")" && pwd)"
repo_dir="$(cd "$script_dir/../.." && pwd)"
output_dir="${1:?Pass an absolute output directory}"
mkdir -p "$output_dir"
build_tmp="$(mktemp -d)"
trap 'rm -rf "$build_tmp"' EXIT
helper="$output_dir/CC Switch Reset Alerts.app"
mkdir -p "$helper/Contents/MacOS"
cp "$script_dir/Info.plist" "$helper/Contents/Info.plist"
for arch in arm64 x86_64; do
  swiftc -target "$arch-apple-macosx14.0" "$script_dir/Notifier.swift" -o "$build_tmp/notifier-$arch" -framework AppKit -framework UserNotifications
done
lipo -create "$build_tmp/notifier-arm64" "$build_tmp/notifier-x86_64" -output "$helper/Contents/MacOS/ResetAlerts"
codesign --force --sign - "$helper"
cd "$repo_dir"
node node_modules/vite/bin/vite.js build
cargo test --manifest-path src-tauri/reset-alert-policy/Cargo.toml
cargo build --release --features tauri/custom-protocol --manifest-path src-tauri/Cargo.toml
case "$(uname -m)" in
  arm64) other_target=x86_64-apple-darwin ;;
  x86_64) other_target=aarch64-apple-darwin ;;
  *) echo 'Build on an Apple Silicon or Intel Mac.' >&2; exit 1 ;;
esac
rustup target add "$other_target"
cargo build --release --features tauri/custom-protocol --target "$other_target" --manifest-path src-tauri/Cargo.toml
app="$output_dir/CC Switch.app"
ditto '/Applications/CC Switch.app' "$app"
lipo -create src-tauri/target/release/cc-switch "src-tauri/target/$other_target/release/cc-switch" -output "$app/Contents/MacOS/cc-switch"
mkdir -p "$app/Contents/Resources"
printf 'banked-reset-alerts-v1\n' > "$app/Contents/Resources/reset-alerts-v1"
codesign --force --deep --sign - "$app"
codesign --verify --deep --strict "$app"
cp "$script_dir/Ativar alertas.command" "$script_dir/resolve-data-dir.sh" "$script_dir/configuration.sh" "$script_dir/LEIA-ME.txt" "$output_dir/"
