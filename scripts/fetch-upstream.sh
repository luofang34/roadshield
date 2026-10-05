#!/bin/sh
# Checks out the pinned upstream commit into .upstream/ (git-ignored) for its
# SVG blanks, licence and engine sources. With --refresh-inputs it also
# re-downloads the HTTP inputs (ShieldJSON, sprite sizes, fonts, licences)
# into the committed inputs directory; `roadshield import` then fails unless
# the config's SHA-256 pins are updated, which makes every bump reviewable.
# Only data is fetched; no upstream code or package manager is run.
set -eu

refresh=no
if [ "${1:-}" = "--refresh-inputs" ]; then
  refresh=yes
  shift
fi
config="${1:-packs/americana.import.json}"
root="$(cd "$(dirname "$0")/.." && pwd)"
dest="$root/.upstream"
inputs="${config%.import.json}.inputs"

eval "$(python3 - "$config" <<'PY'
import json, shlex, sys
c = json.load(open(sys.argv[1]))
print("repo=" + shlex.quote(c["upstream"]["repository"]))
print("commit=" + shlex.quote(c["upstream"]["commit"]))
items = [c["rules"], c["sprite_sizes"]]
for f in c["fonts"]:
    items += [f, f["license"]]
print("downloads=" + shlex.quote("\n".join(i["url"] + " " + i["file"] for i in items)))
PY
)"

# UPSTREAM_REF checks out another ref (e.g. main) instead of the pin, for
# the scheduled upstream check.
if [ -n "${UPSTREAM_REF:-}" ]; then
  commit="$UPSTREAM_REF"
fi
checkout="$dest/$(basename "$repo")"
if [ ! -d "$checkout/.git" ]; then
  git clone --quiet --filter=blob:none --no-checkout "$repo" "$checkout"
fi
git -C "$checkout" fetch --quiet origin "$commit"
git -C "$checkout" -c advice.detachedHead=false checkout --quiet FETCH_HEAD

if [ "$refresh" = yes ]; then
  mkdir -p "$inputs"
  printf '%s\n' "$downloads" | while read -r url file; do
    curl --fail --silent --show-error --location --output "$inputs/$file" "$url"
  done
  echo "refreshed $inputs; update the sha256 pins in $config"
fi
echo "checked out $commit into $checkout"
