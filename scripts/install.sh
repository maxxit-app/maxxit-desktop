#!/usr/bin/env bash
set -euo pipefail
[[ "$(uname -s)" == Darwin ]] || { echo 'Maxxit supports macOS.' >&2; exit 1; }
major=$(sw_vers -productVersion | cut -d. -f1)
(( major >= 14 )) || { echo 'Maxxit requires macOS 14 or later.' >&2; exit 1; }
release=https://github.com/maxxit-app/maxxit-desktop/releases/latest/download
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
# Fail closed on an unavailable release. Source builds are an explicit choice.
curl --fail --location --proto '=https' --tlsv1.2 --retry 2 "$release/Maxxit-universal.dmg" -o "$work/Maxxit.dmg"
curl --fail --location --proto '=https' --tlsv1.2 --retry 2 "$release/SHA256SUMS" -o "$work/SHA256SUMS"
expected=$(awk '$2=="Maxxit-universal.dmg" {print $1}' "$work/SHA256SUMS")
[[ "$expected" =~ ^[a-f0-9]{64}$ ]] || { echo 'Missing release checksum.' >&2; exit 1; }
actual=$(shasum -a 256 "$work/Maxxit.dmg" | awk '{print $1}')
[[ "$actual" == "$expected" ]] || { echo 'Release checksum mismatch.' >&2; exit 1; }
xcrun stapler validate "$work/Maxxit.dmg"
mkdir "$work/mount"
hdiutil attach -readonly -nobrowse -mountpoint "$work/mount" "$work/Maxxit.dmg" >/dev/null
trap 'hdiutil detach "$work/mount" >/dev/null 2>&1 || true; rm -rf "$work"' EXIT
app="$work/mount/Maxxit.app"
codesign --verify --deep --strict --all-architectures -R='identifier "app.maxxit.desktop" and anchor apple generic and certificate leaf[subject.OU] = "DXYF58SJPA"' "$app"
spctl --assess --type execute "$app"
# Stage the verified replacement before moving an existing installation.
destination="$HOME/Applications"
mkdir -p "$destination"
staged=$(mktemp -d "$destination/.maxxit-install.XXXXXX")
trap 'hdiutil detach "$work/mount" >/dev/null 2>&1 || true; rm -rf "$work" "$staged"' EXIT
ditto "$app" "$staged/Maxxit.app"
backup=""
if [[ -e "$destination/Maxxit.app" ]]; then
  backup="$destination/Maxxit-backup-$(date +%Y%m%d%H%M%S)-$$.app"
  mv "$destination/Maxxit.app" "$backup"
fi
if ! mv "$staged/Maxxit.app" "$destination/Maxxit.app"; then
  [[ -z "$backup" ]] || mv "$backup" "$destination/Maxxit.app"
  exit 1
fi
open "$destination/Maxxit.app"
printf 'Installed: %s/Maxxit.app\n' "$destination"
