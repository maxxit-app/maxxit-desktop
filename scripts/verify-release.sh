#!/usr/bin/env bash
set -euo pipefail
version=${1:?Usage: verify-release.sh VERSION DIRECTORY [SOURCE_SHA]}
directory=$(cd "${2:?Provide release artifacts}" && pwd)
sha=${3:-}
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
node scripts/release-metadata.mjs verify "$version" "$directory"
cargo run --quiet --locked --manifest-path src-tauri/Cargo.toml --features release-tools --bin verify_updater -- "$directory/Maxxit-universal.app.tar.gz" "$directory/Maxxit-universal.app.tar.gz.sig" "$root/src-tauri/tauri.conf.json"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
tar -xzf "$directory/Maxxit-universal.app.tar.gz" -C "$work"
app="$work/Maxxit.app"
check_app() {
  candidate=$1
  codesign --verify --deep --strict --all-architectures -R='identifier "app.maxxit.desktop" and anchor apple generic and certificate leaf[subject.OU] = "DXYF58SJPA"' "$candidate"
  details=$(codesign -dv --verbose=4 "$candidate" 2>&1)
  [[ "$details" == *"runtime"* ]] || { echo 'Hardened runtime is required.' >&2; exit 1; }
  codesign -d --entitlements :- "$candidate" > "$work/entitlements.plist" 2>/dev/null
  python3 - "$work/entitlements.plist" <<'PYTHON'
import plistlib, pathlib, sys
raw = pathlib.Path(sys.argv[1]).read_bytes()
entitlements = plistlib.loads(raw) if raw.strip() else {}
for key in ['com.apple.security.get-task-allow', 'com.apple.security.cs.allow-dyld-environment-variables', 'com.apple.security.cs.disable-library-validation']:
    if entitlements.get(key):
        raise SystemExit('Unexpected release entitlement: ' + key)
identifier = entitlements.get('com.apple.application-identifier')
if identifier and identifier != 'DXYF58SJPA.app.maxxit.desktop':
    raise SystemExit('Unexpected application identity entitlement.')
PYTHON
  spctl --assess --type execute "$candidate"
  xcrun stapler validate "$candidate"
  actual=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$candidate/Contents/Info.plist")
  [[ "$actual" == "$version" ]] || { echo 'Bundle version mismatch.' >&2; exit 1; }
  architectures=$(lipo -archs "$candidate/Contents/MacOS/maxxit")
  [[ "$architectures" == *arm64* && "$architectures" == *x86_64* ]] || { echo 'Both Mac architectures are required.' >&2; exit 1; }
  if [[ -n "$sha" ]]; then
    [[ "$sha" =~ ^[a-f0-9]{40}$ ]] || { echo 'Invalid source revision.' >&2; exit 1; }
    [[ "$("$candidate/Contents/MacOS/maxxit" --version)" == "maxxit $version $sha" ]] || { echo 'Packaged source revision mismatch.' >&2; exit 1; }
  fi
}
check_app "$app"
xcrun stapler validate "$directory/Maxxit-universal.dmg"
mkdir "$work/mount"
hdiutil attach -readonly -nobrowse -mountpoint "$work/mount" "$directory/Maxxit-universal.dmg" >/dev/null
trap 'hdiutil detach "$work/mount" >/dev/null 2>&1 || true; rm -rf "$work"' EXIT
check_app "$work/mount/Maxxit.app"
# Compare all app bytes, not just the displayed version.
diff -qr "$app" "$work/mount/Maxxit.app"
printf 'Verified Maxxit %s artifacts.\n' "$version"
