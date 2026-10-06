#!/usr/bin/env bash
set -euo pipefail
version=${1:?Usage: release.sh VERSION APP_PATH DMG_PATH}
app=${2:?Provide a signed and notarized universal Maxxit.app}
dmg=${3:?Provide a signed and notarized DMG}
: "${TAURI_SIGNING_PRIVATE_KEY_PATH:?Set the path to the private updater key}"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo 'Use a stable semantic version.' >&2; exit 1; }
codesign --verify --deep --strict "$app"
spctl --assess --type execute "$app"
xcrun stapler validate "$app"
xcrun stapler validate "$dmg"
file "$app/Contents/MacOS/maxxit" | grep -q 'x86_64' || { echo 'The app must contain Intel code.' >&2; exit 1; }
file "$app/Contents/MacOS/maxxit" | grep -q 'arm64' || { echo 'The app must contain Apple silicon code.' >&2; exit 1; }
output=$(mktemp -d)
trap 'rm -rf "$output"' EXIT
tar -czf "$output/Maxxit-universal.app.tar.gz" -C "$(dirname "$app")" "$(basename "$app")"
cp "$dmg" "$output/Maxxit-universal.dmg"
pnpm tauri signer sign --private-key-path "$TAURI_SIGNING_PRIVATE_KEY_PATH" --password "${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}" --app-version "$version" "$output/Maxxit-universal.app.tar.gz" > "$output/signing.log"
node --input-type=module - "$output" "$version" <<'JS'
import {readFileSync,writeFileSync} from 'node:fs';
const [folder,version]=process.argv.slice(2);
const artifact={signature:readFileSync(`${folder}/Maxxit-universal.app.tar.gz.sig`,'utf8').trim(),url:`https://github.com/maxxit-app/maxxit-desktop/releases/download/v${version}/Maxxit-universal.app.tar.gz`};
writeFileSync(`${folder}/latest.json`,JSON.stringify({version,notes:'Maxxit desktop update.',pub_date:new Date().toISOString(),platforms:{'darwin-aarch64':artifact,'darwin-x86_64':artifact}},null,2)+'\n');
JS
(cd "$output" && shasum -a 256 Maxxit-universal.app.tar.gz Maxxit-universal.dmg > SHA256SUMS)
gh release create "v$version" --repo maxxit-app/maxxit-desktop --draft --title "Maxxit $version" --generate-notes "$output/Maxxit-universal.app.tar.gz" "$output/Maxxit-universal.app.tar.gz.sig" "$output/Maxxit-universal.dmg" "$output/latest.json" "$output/SHA256SUMS"
echo 'Draft created. Test both Mac architectures, review the assets, then publish.'
