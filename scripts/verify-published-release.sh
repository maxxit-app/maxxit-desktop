#!/usr/bin/env bash
set -euo pipefail
version=${1:?Provide the published version}
[[ "$version" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]] || exit 1
root=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
gh release download "v$version" --repo maxxit-app/maxxit-desktop --dir "$work"
bash "$root/scripts/verify-release.sh" "$version" "$work" "${2:-}"
