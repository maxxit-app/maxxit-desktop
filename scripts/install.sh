#!/usr/bin/env bash
set -euo pipefail
if [[ "$(uname -s)" != Darwin ]]; then echo 'Maxxit currently supports macOS.' >&2; exit 1; fi
major=$(sw_vers -productVersion | cut -d. -f1)
if (( major < 14 )); then echo 'Maxxit requires macOS 14 or later.' >&2; exit 1; fi
repo=https://github.com/maxxit-app/maxxit-desktop
release="$repo/releases/latest/download"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
if curl -fsSL "$release/Maxxit-universal.app.tar.gz" -o "$work/Maxxit-universal.app.tar.gz"; then
  curl -fsSL "$release/SHA256SUMS" -o "$work/SHA256SUMS"
  expected=$(awk '$2=="Maxxit-universal.app.tar.gz" {print $1}' "$work/SHA256SUMS")
  [[ "$expected" =~ ^[a-f0-9]{64}$ ]] || { echo 'Missing release checksum' >&2; exit 1; }
  actual=$(shasum -a 256 "$work/Maxxit-universal.app.tar.gz" | awk '{print $1}')
  [[ "$actual" == "$expected" ]] || { echo 'Release checksum mismatch' >&2; exit 1; }
  tar -xzf "$work/Maxxit-universal.app.tar.gz" -C "$work"
  codesign --verify --deep --strict "$work/Maxxit.app"
  spctl --assess --type execute "$work/Maxxit.app"
fi
if [[ ! -d "$work/Maxxit.app" ]]; then
  echo 'Signed release is not available yet. Building Maxxit on this Mac.'
  command -v brew >/dev/null || { echo 'Install Homebrew from https://brew.sh, then run this command again.' >&2; exit 1; }
  xcode-select -p >/dev/null 2>&1 || { echo 'Run xcode-select --install, finish installation, then run this command again.' >&2; exit 1; }
  brew install node@24 rust
  export PATH="$(brew --prefix node@24)/bin:$(brew --prefix rust)/bin:$PATH"
  git clone --depth 1 --branch v0.1.10 "$repo.git" "$work/source"
  cd "$work/source"
  npx --yes pnpm@10.19.0 install --frozen-lockfile
  npx --yes pnpm@10.19.0 tauri build --bundles app --config '{"bundle":{"createUpdaterArtifacts":false}}'
  ditto src-tauri/target/release/bundle/macos/Maxxit.app "$work/Maxxit.app"
  codesign --force --deep --sign - "$work/Maxxit.app"
  codesign --verify --deep --strict "$work/Maxxit.app"
fi
# Keep any existing installation recoverable.
destination="$HOME/Applications"
mkdir -p "$destination"
if [[ -d "$destination/Maxxit.app" ]]; then mv "$destination/Maxxit.app" "$destination/Maxxit-backup-$(date +%Y%m%d%H%M%S).app"; fi
ditto "$work/Maxxit.app" "$destination/Maxxit.app"
open "$destination/Maxxit.app"
printf 'Installed: %s/Maxxit.app\n' "$destination"
