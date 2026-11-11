#!/bin/sh
# sets the app's version everywhere it's written:
#   pnpm version:bump --to z.y.z   (or ./scripts/bump-version.sh x.y.z)
set -e

[ "$1" = "--to" ] && shift
version="$1"
if ! echo "$version" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
  echo "usage: pnpm version:bump --to <major.minor.patch>" >&2
  exit 1
fi

cd "$(dirname "$0")/.."

sed -i.bak -E "s/^  \"version\": \"[^\"]*\"/  \"version\": \"$version\"/" package.json engine/tauri.conf.json
sed -i.bak -E "s/^version = \"[^\"]*\"/version = \"$version\"/" engine/Cargo.toml
rm package.json.bak engine/tauri.conf.json.bak engine/Cargo.toml.bak

grep -H -m1 -E '"version"|^version' package.json engine/tauri.conf.json engine/Cargo.toml
