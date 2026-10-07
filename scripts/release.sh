#!/usr/bin/env bash
set -euo pipefail
version=${1:?Usage: release.sh VERSION APP_PATH DMG_PATH [--dry-run OUTPUT_DIRECTORY]}
app=$(cd "$(dirname "${2:?Provide the notarized universal app}")" && pwd)/$(basename "$2")
dmg=$(cd "$(dirname "${3:?Provide the notarized DMG}")" && pwd)/$(basename "$3")
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
: "${TAURI_SIGNING_PRIVATE_KEY_PATH:?Set the path to the existing updater private key}"
sha=$(node scripts/release-metadata.mjs preflight "$version")
output=$(mktemp -d)
trap 'rm -rf "$output"' EXIT
codesign --verify --deep --strict --all-architectures -R='identifier "app.maxxit.desktop" and anchor apple generic and certificate leaf[subject.OU] = "DXYF58SJPA"' "$app"
# The signed binary must identify the same revision as the reviewed checkout.
[[ "$("$app/Contents/MacOS/maxxit" --version)" == "maxxit $version $sha" ]] || { echo 'App source revision/version mismatch.' >&2; exit 1; }
tar -czf "$output/Maxxit-universal.app.tar.gz" -C "$(dirname "$app")" "$(basename "$app")"
cp "$dmg" "$output/Maxxit-universal.dmg"
pnpm tauri signer sign --private-key-path "$TAURI_SIGNING_PRIVATE_KEY_PATH" --password "${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}" --app-version "$version" "$output/Maxxit-universal.app.tar.gz" > "$output/signing.log"
node scripts/release-metadata.mjs manifest "$version" "$output"
(cd "$output" && shasum -a 256 Maxxit-universal.app.tar.gz Maxxit-universal.dmg Maxxit-universal.app.tar.gz.sig latest.json > SHA256SUMS)
bash scripts/verify-release.sh "$version" "$output" "$sha"
if [[ "${4:-}" == --dry-run ]]; then
  destination=${5:?Provide a new output directory}
  [[ ! -e "$destination" ]] || { echo 'Output directory must not exist.' >&2; exit 1; }
  mkdir -m 700 "$destination"
  cp "$output/"{Maxxit-universal.app.tar.gz,Maxxit-universal.dmg,Maxxit-universal.app.tar.gz.sig,latest.json,SHA256SUMS} "$destination/"
  echo 'Rehearsal verified. Nothing published.'
else
  [[ $# == 3 ]] || { echo 'Unknown release option.' >&2; exit 1; }
  node --input-type=module - "$version" > "$output/release-notes.md" <<'JS'
import { readFileSync } from "node:fs";
import { changelogNotes } from "./scripts/release-lib.mjs";
console.log(changelogNotes(readFileSync("CHANGELOG.md", "utf8"), process.argv[2]));
JS
  gh release create "v$version" --repo maxxit-app/maxxit-desktop --target "$sha" --draft --title "Maxxit $version" --notes-file "$output/release-notes.md" "$output/Maxxit-universal.app.tar.gz" "$output/Maxxit-universal.app.tar.gz.sig" "$output/Maxxit-universal.dmg" "$output/latest.json" "$output/SHA256SUMS"
  echo 'Verified draft created. Complete native installation and upgrade acceptance before publication.'
fi
