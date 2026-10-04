// Renders a shield with the WASM build under Node.
//   wasm-pack build crates/roadshield-wasm --release --target nodejs --out-dir ../../target/wasm-node
//   node examples/wasm/render.mjs packs/americana US:I 287 > i287.svg
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const { RoadShield } = require("../../target/wasm-node/roadshield_wasm.js");

const [packDir = "packs/americana", network = "US:I", ref = "287"] = process.argv.slice(2);

function* files(dir) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) yield* files(p);
    else yield p;
  }
}

const shields = new RoadShield();
for (const path of files(packDir)) {
  shields.addFile(relative(packDir, path).split("\\").join("/"), readFileSync(path));
}
const manifest = JSON.parse(shields.load());
const rendering = JSON.parse(shields.render(JSON.stringify({ network, ref })));
if (rendering.kind !== "symbol") {
  console.error(`no shield (${manifest.id}):`, rendering.reason);
  process.exitCode = 1;
} else {
  process.stdout.write(rendering.svg);
  console.error(`${rendering.rule.rule_key}: ${rendering.width}x${rendering.height}`);
}
