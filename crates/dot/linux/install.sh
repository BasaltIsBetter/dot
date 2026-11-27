#!/usr/bin/env bash
set -euo pipefail

readonly PROGRAMS=(dot dot-shell dot-agents)
readonly DESKTOP_ENTRY="dev.dot.desktop.desktop"
readonly ICON="dev.dot.desktop.svg"
readonly ICON_FOLDER="icons/hicolor/scalable/apps"
readonly INSTALL_FOLDER="dot-app"

fail() {
  echo "install: $*" >&2
  exit 1
}

refresh_launchers() {
  local data="$1"
  if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$data/applications" 2>/dev/null || true
  fi
  if [[ -f "$data/icons/hicolor/icon-theme.cache" ]] && command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$data/icons/hicolor" >/dev/null 2>&1 || true
  fi
  if command -v dbus-send >/dev/null 2>&1; then
    dbus-send --session --type=signal /KIconLoader org.kde.KIconLoader.iconChanged int32:0 2>/dev/null || true
  fi
}

bundle="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
for program in "${PROGRAMS[@]}"; do
  [[ -x "$bundle/bin/$program" ]] || fail "$bundle/bin/$program is missing: run this from an unpacked Dot bundle"
done

data="${XDG_DATA_HOME:-${HOME:?}/.local/share}"
home="$data/$INSTALL_FOLDER"
bin="$HOME/.local/bin"
launcher="$bin/dot"
case "$home" in
  *[\"\`\$\\%]*) fail "cannot write a launcher entry for a path holding \", \`, \$, \\ or %: $home" ;;
esac
if [[ -e "$launcher" || -L "$launcher" ]] && [[ "$(readlink "$launcher" || true)" != "$home/bin/dot" ]]; then
  fail "$launcher already exists and was not written by Dot; move it away, then install again"
fi

mkdir -p "$data" "$bin" "$data/applications" "$data/$ICON_FOLDER"
rm -rf "$home.new" "$home.previous"
mkdir "$home.new"
cp -R "$bundle/bin" "$bundle/share" "$home.new/"
if [[ -e "$home" ]]; then
  mv "$home" "$home.previous"
fi
mv "$home.new" "$home"
rm -rf "$home.previous"
ln -sfn "$home/bin/dot" "$launcher"

while IFS= read -r line; do
  case "$line" in
    Exec=*) printf 'Exec="%s" %%f\n' "$home/bin/dot" ;;
    *) printf '%s\n' "$line" ;;
  esac
done < "$home/share/applications/$DESKTOP_ENTRY" > "$data/applications/$DESKTOP_ENTRY.new"
mv -f "$data/applications/$DESKTOP_ENTRY.new" "$data/applications/$DESKTOP_ENTRY"
cp "$home/share/$ICON_FOLDER/$ICON" "$data/$ICON_FOLDER/$ICON"
refresh_launchers "$data"

echo "Installed $home; quit and reopen Dot to use this build."
case ":$PATH:" in
  *":$bin:"*) ;;
  *) echo "Add $bin to your PATH to open a project with: dot <folder>" ;;
esac
