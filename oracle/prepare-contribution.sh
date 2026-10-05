#!/bin/sh
# Builds upstream with the contribution draft (contrib/americana/*.patch)
# applied to a separate worktree of the pinned checkout, then checks that the
# draft's generated ShieldJSON equals the pinned rules plus exactly the
# pack's extension networks. Run oracle/sweep.mjs --include-extensions
# --upstream <worktree> afterwards to compare the drawings.
set -eu

root="$(cd "$(dirname "$0")/.." && pwd)"
checkout="${1:-$root/.upstream/openstreetmap-americana}"
work="${2:-$root/.upstream/americana-contrib}"
commit="$(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['upstream']['commit'])" "$root/packs/americana.import.json")"

if [ ! -d "$work/.git" ] && [ ! -f "$work/.git" ]; then
  git -C "$checkout" worktree add --detach "$work" "$commit"
fi
git -C "$work" checkout --quiet --force "$commit"
git -C "$work" clean --quiet --force -d -e node_modules
for patch in "$root"/contrib/americana/*.patch; do
  git -C "$work" apply "$patch"
done

cd "$work"
npm ci --ignore-scripts --no-audit --no-fund
(cd shieldlib && node scripts/build.js)
npx tsx scripts/sprites.ts
cp src/configs/config.maptiler.js src/config.js
npx tsx scripts/build.ts
npx tsx scripts/generate_shield_defs.ts -o dist/shields.json

python3 - "$work/dist/shields.json" "$root/packs/americana.inputs/shields.json" "$root/packs/americana.extensions.json" <<'PY'
import json, sys
draft = json.load(open(sys.argv[1]))
pinned = json.load(open(sys.argv[2]))
ext = json.load(open(sys.argv[3]))["networks"]
added = {k: v for k, v in draft["networks"].items() if k not in pinned["networks"]}
changed = [k for k, v in pinned["networks"].items() if draft["networks"].get(k) != v]
problems = []
if changed:
    problems.append(f"draft changes upstream networks: {changed}")
if draft["options"] != pinned["options"]:
    problems.append("draft changes global options")
if added != ext:
    problems.append(f"draft adds {sorted(added)}, extensions define {sorted(ext)} (or definitions differ)")
if problems:
    sys.exit("contribution draft does not match the pack:\n" + "\n".join(problems))
print(f"contribution draft adds exactly the pack's extensions: {sorted(added)}")
PY
