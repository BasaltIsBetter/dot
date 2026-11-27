#!/usr/bin/env bash
set -euo pipefail

readonly PACKAGE="dot"
readonly SHELL_STAND_IN="dot-shell"
readonly AGENTS="dot-agents"
readonly DESKTOP_ENTRY="dev.dot.desktop.desktop"
readonly ICON="dev.dot.desktop.svg"
readonly NOTICES=(LICENSE NOTICE)

linux_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
manifest="$linux_dir/../../../Cargo.toml"
workspace="$linux_dir/../../.."

for notice in "${NOTICES[@]}"; do
  [[ -s "$workspace/$notice" ]] || { echo "bundle: $notice is missing or empty" >&2; exit 1; }
done

cargo build --release --locked --package "$PACKAGE" --package "$SHELL_STAND_IN" --package dot-container \
  --manifest-path "$manifest"

version="$(cargo pkgid --package "$PACKAGE" --manifest-path "$manifest" | sed 's/.*[#@]//')"
host="$(rustc -vV | sed -n 's/^host: //p')"
target_dir="$(cargo metadata --format-version 1 --no-deps --manifest-path "$manifest" \
  | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')"
name="dot-$version-$host"
bundle="${target_dir:?}/release/bundle/linux"
root="$bundle/$name"

rm -rf "$root"
for program in "$PACKAGE" "$SHELL_STAND_IN" "$AGENTS"; do
  install -D -m 755 "$target_dir/release/$program" "$root/bin/$program"
done
install -D -m 644 "$linux_dir/$DESKTOP_ENTRY" "$root/share/applications/$DESKTOP_ENTRY"
install -D -m 644 "$linux_dir/../assets/icon.svg" "$root/share/icons/hicolor/scalable/apps/$ICON"
install -m 755 "$linux_dir/install.sh" "$root/install.sh"
install -d "$root/share/doc/dot"
DOT_TARGET="$host" python3 -I "$linux_dir/../macos/licenses.py" "$manifest" \
  "$root/share/doc/dot/ThirdPartyLicenses.txt" \
  "$linux_dir/../../dot-desktop/assets/OFL.txt" "$linux_dir/../../dot-compact/third-party/rtk/LICENSE"
[[ -s "$root/share/doc/dot/ThirdPartyLicenses.txt" ]] || { echo "bundle: ThirdPartyLicenses.txt is empty" >&2; exit 1; }
for notice in "${NOTICES[@]}"; do
  install -m 644 "$workspace/$notice" "$root/share/doc/dot/"
done

tar -C "$bundle" -czf "$bundle/$name.tar.gz" "$name"
echo "$bundle/$name.tar.gz"
