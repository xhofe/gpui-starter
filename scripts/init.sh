#!/usr/bin/env bash
# Rename this template from the placeholder identity (GPUI Starter /
# gpui-starter) to your app. Run once after cloning.
#
#   ./scripts/init.sh my-app
#   ./scripts/init.sh --dry-run          # scan leftover legacy tokens only
#
# `my-app` must be kebab-case. Pascal / snake / env / display names are
# derived. GitHub URL comes from `origin`; authors from `git config`.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# Split so this file itself is not a hit for the leftover scan.
LEGACY_A="ze"
LEGACY_B="dis"
LEGACY="${LEGACY_A}${LEGACY_B}"
LEGACY_TITLE="$(echo "${LEGACY_A:0:1}" | tr '[:lower:]' '[:upper:]')${LEGACY_A:1}${LEGACY_B}"
LEGACY_OWNER="vi""canso"

scan_legacy() {
  local hits
  hits="$(
    rg -n -i -e "$LEGACY" -e "$LEGACY_OWNER" \
      --glob '!.git/**' \
      --glob '!target/**' \
      --glob '!.claude/**' \
      --glob '!Cargo.lock' \
      --glob '!scripts/init.sh' \
      . 2>/dev/null || true
  )"
  if [ -n "$hits" ]; then
    echo "leftover legacy tokens:" >&2
    echo "$hits" >&2
    return 1
  fi
  echo "no leftover $LEGACY / $LEGACY_OWNER tokens"
}

kebab_ok() {
  [[ "$1" =~ ^[a-z][a-z0-9]*(-[a-z0-9]+)*$ ]]
}

to_snake() { echo "$1" | tr '-' '_'; }
to_env() { echo "$1" | tr '[:lower:]-' '[:upper:]_'; }
to_display() {
  echo "$1" | awk -F- '{
    for (i = 1; i <= NF; i++) {
      $i = toupper(substr($i, 1, 1)) substr($i, 2)
    }
    print
  }' OFS=' '
}
to_pascal() {
  echo "$1" | awk -F- '{
    for (i = 1; i <= NF; i++) {
      printf "%s", toupper(substr($i, 1, 1)) substr($i, 2)
    }
    print ""
  }'
}

github_from_origin() {
  local url
  url="$(git remote get-url origin 2>/dev/null || true)"
  url="${url%.git}"
  if [[ "$url" =~ github.com[:/]([^/]+)/([^/]+)$ ]]; then
    echo "${BASH_REMATCH[1]}/${BASH_REMATCH[2]}"
  else
    echo ""
  fi
}

authors_from_git() {
  local name email
  name="$(git config user.name 2>/dev/null || true)"
  email="$(git config user.email 2>/dev/null || true)"
  if [ -n "$name" ] && [ -n "$email" ]; then
    echo "$name <$email>"
  elif [ -n "$name" ]; then
    echo "$name"
  else
    echo ""
  fi
}

if [ "${1:-}" = "--dry-run" ]; then
  scan_legacy
  exit $?
fi

KEBAB="${1:-}"
if [ -z "$KEBAB" ]; then
  echo "usage: $0 <kebab-name>   or   $0 --dry-run" >&2
  exit 1
fi
if ! kebab_ok "$KEBAB"; then
  echo "error: name must be kebab-case like my-app (got: $KEBAB)" >&2
  exit 1
fi
if [ "${#KEBAB}" -lt 3 ]; then
  echo "error: name is too short" >&2
  exit 1
fi
case "$KEBAB" in
  gpui-starter|gpui|starter|"$LEGACY"|init|test|app) 
    echo "error: name collides with a reserved token: $KEBAB" >&2
    exit 1
    ;;
esac

SNAKE="$(to_snake "$KEBAB")"
ENV="$(to_env "$KEBAB")"
DISPLAY="$(to_display "$KEBAB")"
PASCAL="$(to_pascal "$KEBAB")"
GITHUB="$(github_from_origin)"
AUTHORS="$(authors_from_git)"
OWNER="${GITHUB%%/*}"
[ -n "$OWNER" ] || OWNER="example"

echo "kebab:    $KEBAB"
echo "snake:    $SNAKE"
echo "env:      $ENV"
echo "display:  $DISPLAY"
echo "pascal:   $PASCAL (used only if it appears)"
echo "github:   ${GITHUB:-<none — leaving example URLs>}"
echo "authors:  ${AUTHORS:-<none — leaving Andy Hsu>}"
echo "flatpak:  io.github.$OWNER.$KEBAB"

replace_in_tree() {
  local from="$1" to="$2"
  [ "$from" = "$to" ] && return 0
  # python for NUL-safe walk; skip binaries and VCS.
  FROM="$from" TO="$to" python3 - << 'PY'
import os, sys
frm, to = os.environ["FROM"], os.environ["TO"]
skip_dirs = {".git", "target", ".claude"}
skip_suffix = {".png", ".ico", ".icns", ".gif", ".woff2", ".ttf", ".otf", ".lock"}
for dirpath, dirnames, filenames in os.walk("."):
    dirnames[:] = [d for d in dirnames if d not in skip_dirs]
    for name in filenames:
        path = os.path.join(dirpath, name)
        if os.path.splitext(name)[1].lower() in skip_suffix:
            continue
        try:
            data = open(path, "r", encoding="utf-8").read()
        except (UnicodeDecodeError, OSError):
            continue
        if frm not in data:
            continue
        open(path, "w", encoding="utf-8").write(data.replace(frm, to))
PY
}

# Longest tokens first so they are not chopped by a shorter sibling.
replace_in_tree "io.github.xhofe.gpui-starter" "io.github.${OWNER}.${KEBAB}"
if [ -n "$GITHUB" ]; then
  replace_in_tree "https://github.com/xhofe/gpui-starter" "https://github.com/${GITHUB}"
  replace_in_tree "xhofe/gpui-starter" "$GITHUB"
fi
replace_in_tree "com.example.gpui-starter" "com.example.${KEBAB}"
if [ -n "$AUTHORS" ]; then
  replace_in_tree "Andy Hsu <i@nn.ci>" "$AUTHORS"
  replace_in_tree "Andy Hsu" "${AUTHORS%% <*}"
fi
replace_in_tree "GPUI_STARTER" "$ENV"
replace_in_tree "gpui_starter" "$SNAKE"
replace_in_tree "GPUI Starter" "$DISPLAY"
replace_in_tree "GpuiStarter" "$PASCAL"

# Rename paths that still contain the placeholder kebab, then rewrite
# the remaining in-file tokens so Cargo.toml `path =` matches the dirs.
if [ "$KEBAB" != "gpui-starter" ]; then
  mv crates/gpui-starter-ui "crates/${KEBAB}-ui"
  mv crates/gpui-starter-db "crates/${KEBAB}-db"
  mv assets/gpui-starter.desktop "assets/${KEBAB}.desktop"
  mv icons/gpui-starter.ico "icons/${KEBAB}.ico"
  mv icons/gpui-starter.icns "icons/${KEBAB}.icns"
  mv icons/gpui-starter-icon.svg "icons/${KEBAB}-icon.svg"
fi
replace_in_tree "gpui-starter" "$KEBAB"

# Flatpak files are named after the app id — rename if the owner/name moved.
old_flat="flatpak/io.github.xhofe.gpui-starter"
new_flat="flatpak/io.github.${OWNER}.${KEBAB}"
if [ "$old_flat" != "$new_flat" ]; then
  for ext in yml desktop metainfo.xml; do
    if [ -f "${old_flat}.${ext}" ]; then
      mv "${old_flat}.${ext}" "${new_flat}.${ext}"
    fi
  done
fi

echo "running cargo check…"
cargo check --workspace --offline 2>/dev/null || cargo check --workspace

scan_legacy
echo "done. next: make fmt && make lint"
