#!/usr/bin/env bash
set -euo pipefail

readonly APP_NAME="Dot"
readonly PACKAGE="dot"

macos_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
manifest="$macos_dir/../../../Cargo.toml"

cargo build --release --package "$PACKAGE" --manifest-path "$manifest"

version="$(cargo pkgid --package "$PACKAGE" --manifest-path "$manifest" | sed 's/.*[#@]//')"
target_dir="$(cargo metadata --format-version 1 --no-deps --manifest-path "$manifest" \
  | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')"
app="${target_dir:?}/release/bundle/macos/$APP_NAME.app"

rm -rf "$app"
mkdir -p "$app/Contents/MacOS"
cp "$macos_dir/Info.plist" "$app/Contents/Info.plist"
plutil -replace CFBundleShortVersionString -string "$version" "$app/Contents/Info.plist"
plutil -replace CFBundleVersion -string "$version" "$app/Contents/Info.plist"
cp "$target_dir/release/$PACKAGE" "$app/Contents/MacOS/$PACKAGE"
codesign --force --sign - "$app"

echo "$app"
