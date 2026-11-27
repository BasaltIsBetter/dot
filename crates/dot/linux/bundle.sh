#!/usr/bin/env bash
set -euo pipefail

readonly PACKAGE="dot"
readonly SHELL_STAND_IN="dot-shell"
readonly AGENTS="dot-agents"
readonly DESKTOP_ENTRY="dev.dot.desktop.desktop"

linux_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
manifest="$linux_dir/../../../Cargo.toml"

cargo build --release --package "$PACKAGE" --package "$SHELL_STAND_IN" --package dot-container \
  --manifest-path "$manifest"

version="$(cargo pkgid --package "$PACKAGE" --manifest-path "$manifest" | sed 's/.*[#@]//')"
host="$(rustc -vV | sed -n 's/^host: //p')"
target_dir="$(cargo metadata --format-version 1 --no-deps --manifest-path "$manifest" \
  | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')"
name="dot-$version-$host"
bundle="${target_dir:?}/release/bundle/linux"
root="$bundle/$name"

for program in "$PACKAGE" "$SHELL_STAND_IN" "$AGENTS"; do
  install -D -m 755 "$target_dir/release/$program" "$root/bin/$program"
done
install -D -m 644 "$linux_dir/$DESKTOP_ENTRY" "$root/share/applications/$DESKTOP_ENTRY"
install -d "$root/share/doc/dot"
DOT_TARGET="$host" python3 -I "$linux_dir/../macos/licenses.py" "$manifest" \
  "$root/share/doc/dot/ThirdPartyLicenses.txt" \
  "$linux_dir/../../dot-desktop/assets/OFL.txt" "$linux_dir/../../dot-compact/third-party/rtk/LICENSE"
install -m 644 "$linux_dir/../../../LICENSE" "$linux_dir/../../../NOTICE" "$root/share/doc/dot/"

tar -C "$bundle" -czf "$bundle/$name.tar.gz" "$name"
echo "$bundle/$name.tar.gz"
