#!/usr/bin/env bash
set -euo pipefail

readonly USAGE="usage: notes.sh <version> <changelog>"

version="${1:?$USAGE}"
changelog="${2:?$USAGE}"

section="$(awk -v heading="## [$version]" '
  inside && /^## / { exit }
  inside { print }
  index($0, heading) == 1 { inside = 1 }
' "$changelog" | sed -e '/./,$!d')"

[[ "$section" =~ [^[:space:]] ]] || { echo "notes: $changelog has no notes under \"## [$version]\"" >&2; exit 1; }
printf '%s\n' "$section"
