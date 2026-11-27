#!/usr/bin/env bash
set -euo pipefail

readonly APP_NAME="Dot.app"
readonly PROGRAM="Contents/MacOS/dot"
readonly LAUNCH_SERVICES="/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister"

fail() {
  echo "install: $*" >&2
  exit 1
}

is_ours() {
  local link="$1"
  [[ -L "$link" ]] && return 1
  [[ -f "$link" ]] && grep -q "/$APP_NAME/$PROGRAM" "$link"
}

macos_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source_app="${1:-}"
if [[ -z "$source_app" ]]; then
  source_app="$("$macos_dir/bundle.sh" | tail -n 1)"
fi
[[ "$(basename "$source_app")" == "$APP_NAME" && -x "$source_app/$PROGRAM" ]] \
  || fail "expected a built $APP_NAME, got: $source_app"

applications="${HOME:?}/Applications"
app="$applications/$APP_NAME"
bin="$HOME/.local/bin"
launcher="$bin/dot"
if [[ -e "$launcher" || -L "$launcher" ]] && ! is_ours "$launcher"; then
  fail "$launcher already exists and was not written by Dot; move it away, then install again"
fi

stage="$(mktemp -d "${TMPDIR:-/tmp}/dot-install.XXXXXX")"
trap 'rm -rf "$stage"' EXIT
ditto "$source_app" "$stage/$APP_NAME"
codesign --verify --deep --strict "$stage/$APP_NAME" || fail "the signature of $source_app is invalid"

mkdir -p "$applications" "$bin"
if [[ -e "$app" ]]; then
  mv "$app" "$stage/previous.app"
fi
mv "$stage/$APP_NAME" "$app"

printf '#!/bin/bash\nexec %q "$@"\n' "$app/$PROGRAM" > "$launcher.new"
chmod 755 "$launcher.new"
mv -f "$launcher.new" "$launcher"

"${DOT_LAUNCH_SERVICES:-$LAUNCH_SERVICES}" -f "$app"

echo "Installed $app; quit and reopen Dot to use this build."
case ":$PATH:" in
  *":$bin:"*) ;;
  *) echo "Add $bin to your PATH to open a project with: dot <folder>" ;;
esac
