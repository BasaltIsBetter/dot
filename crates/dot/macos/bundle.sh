#!/usr/bin/env bash
set -euo pipefail

readonly APP_NAME="Dot"
readonly PACKAGE="dot"
readonly SHELL_STAND_IN="dot-shell"
readonly AGENTS="dot-agents"
readonly SIGNING_IDENTITY="Dot Dev"

macos_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
manifest="$macos_dir/../../../Cargo.toml"

cargo build --release --package "$PACKAGE" --package "$SHELL_STAND_IN" --package dot-container \
  --manifest-path "$manifest"

version="$(cargo pkgid --package "$PACKAGE" --manifest-path "$manifest" | sed 's/.*[#@]//')"
target_dir="$(cargo metadata --format-version 1 --no-deps --manifest-path "$manifest" \
  | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')"
app="${target_dir:?}/release/bundle/macos/$APP_NAME.app"

rm -rf "$app" "$app.dSYM"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$app.dSYM"
cp "$macos_dir/Info.plist" "$app/Contents/Info.plist"
plutil -replace CFBundleShortVersionString -string "$version" "$app/Contents/Info.plist"
plutil -replace CFBundleVersion -string "$version" "$app/Contents/Info.plist"
cp "$target_dir/release/$PACKAGE" "$app/Contents/MacOS/$PACKAGE"
cp "$target_dir/release/$SHELL_STAND_IN" "$app/Contents/MacOS/$SHELL_STAND_IN"
cp "$target_dir/release/$AGENTS" "$app/Contents/MacOS/$AGENTS"
cp -RL "$target_dir/release/$PACKAGE.dSYM" "$target_dir/release/$SHELL_STAND_IN.dSYM" \
  "$target_dir/release/$AGENTS.dSYM" "$app.dSYM/"
python3 -I "$macos_dir/licenses.py" "$manifest" "$app/Contents/Resources/ThirdPartyLicenses.txt" \
  "$macos_dir/../../dot-desktop/assets/OFL.txt" "$macos_dir/../../dot-compact/third-party/rtk/LICENSE"

if security find-identity -p codesigning | grep -q "\"$SIGNING_IDENTITY\""; then
  codesign --force --options runtime --sign "$SIGNING_IDENTITY" "$app/Contents/MacOS/$SHELL_STAND_IN"
  codesign --force --options runtime --sign "$SIGNING_IDENTITY" "$app/Contents/MacOS/$AGENTS"
  codesign --force --options runtime --sign "$SIGNING_IDENTITY" "$app"
else
  echo "warning: no \"$SIGNING_IDENTITY\" certificate, signing ad hoc: macOS will ask again for keychain access after every build" >&2
  codesign --force --options runtime --sign - "$app/Contents/MacOS/$SHELL_STAND_IN"
  codesign --force --options runtime --sign - "$app/Contents/MacOS/$AGENTS"
  codesign --force --options runtime --sign - "$app"
fi

echo "$app"
