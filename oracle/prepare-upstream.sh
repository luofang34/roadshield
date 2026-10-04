#!/bin/sh
# Builds the pinned upstream checkout for the oracle sweep: installs its npm
# dependencies (lifecycle scripts disabled), builds shieldlib, the sprite
# sheets and the site bundle, regenerates ShieldJSON, and checks that the
# generated ShieldJSON and sprite layout equal the committed pack inputs.
set -eu

root="$(cd "$(dirname "$0")/.." && pwd)"
checkout="${1:-$root/.upstream/openstreetmap-americana}"
inputs="${2:-$root/packs/americana.inputs}"

cd "$checkout"
npm ci --ignore-scripts --no-audit --no-fund
(cd shieldlib && node scripts/build.js)
npx tsx scripts/sprites.ts
cp src/configs/config.maptiler.js src/config.js
npx tsx scripts/build.ts
npx tsx scripts/generate_shield_defs.ts -o dist/shields.json

cmp dist/shields.json "$inputs/shields.json"
cmp dist/sprites/sprite.json "$inputs/sprite.json"
echo "upstream build matches the pinned pack inputs"
