// Visual conformance sweep against the pinned upstream TypeScript renderer.
//
// Serves the upstream site build (dist/), the roadshield pack and the
// roadshield WASM build from one local origin, opens upstream's
// shieldtest.html in Chromium, and for every case compares the image
// upstream generates (via `styleimagemissing`) with roadshield's raster of
// the same route at the same device pixel ratio. Fonts are served from the
// pinned inputs so results do not depend on the live webfont host.
//
//   node oracle/sweep.mjs --upstream .upstream/openstreetmap-americana \
//     --pack packs/americana --wasm target/wasm-web --dpr 1 --out target/oracle
//
// Exits non-zero when any case differs beyond the anti-aliasing bounds.

import { createServer } from "node:http";
import { createRequire } from "node:module";
import { mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { extname, join, relative, resolve } from "node:path";
import { parseArgs } from "node:util";

const { values: opt } = parseArgs({
  options: {
    upstream: { type: "string", default: ".upstream/openstreetmap-americana" },
    pack: { type: "string", default: "packs/americana" },
    inputs: { type: "string", default: "packs/americana.inputs" },
    wasm: { type: "string", default: "target/wasm-web" },
    dpr: { type: "string", default: "1" },
    out: { type: "string", default: "target/oracle" },
    limit: { type: "string", default: "0" },
    "mean-max": { type: "string" },
    "frac-max": { type: "string" },
  },
});

// Bounds per device pixel ratio, sized to hold on both macOS (CoreText
// emboldens canvas text) and Linux (FreeType hinting); resvg does neither,
// so glyph rasterisation dominates the remaining difference. 1x guards
// presence, size and gross errors. 2x is the geometric gate: a half-pixel
// layout error moves edges a whole device pixel and fails hundreds of cases.
const bounds = { 1: { mean: 40, frac: 0.03 }, 2: { mean: 36, frac: 0.025 } }[opt.dpr] ?? { mean: 36, frac: 0.025 };
const meanMax = Number(opt["mean-max"] ?? bounds.mean);
const fracMax = Number(opt["frac-max"] ?? bounds.frac);

const upstream = resolve(opt.upstream);
const require = createRequire(join(upstream, "package.json"));
const { chromium } = require("playwright");

const types = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".mjs": "text/javascript",
  ".css": "text/css",
  ".json": "application/json",
  ".png": "image/png",
  ".svg": "image/svg+xml",
  ".wasm": "application/wasm",
  ".woff2": "font/woff2",
};

function* walk(dir) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) yield* walk(p);
    else yield p;
  }
}

const roots = {
  "/upstream/": join(upstream, "dist"),
  "/pack/": resolve(opt.pack),
  "/wasm/": resolve(opt.wasm),
};
const packFiles = [...walk(roots["/pack/"])].map((p) => relative(roots["/pack/"], p).split("\\").join("/"));

const server = createServer((req, res) => {
  const url = new URL(req.url, "http://localhost");
  if (url.pathname === "/pack-index.json") {
    res.writeHead(200, { "content-type": "application/json" });
    return res.end(JSON.stringify(packFiles));
  }
  for (const [prefix, dir] of Object.entries(roots)) {
    if (!url.pathname.startsWith(prefix)) continue;
    const file = resolve(dir, decodeURIComponent(url.pathname.slice(prefix.length)));
    if (!file.startsWith(dir)) break;
    try {
      const body = readFileSync(file);
      res.writeHead(200, { "content-type": types[extname(file)] ?? "application/octet-stream" });
      return res.end(body);
    } catch {
      break;
    }
  }
  res.writeHead(404).end();
});
await new Promise((r) => server.listen(0, "127.0.0.1", r));
const origin = `http://127.0.0.1:${server.address().port}`;

const browser = await chromium.launch({ channel: process.env.ORACLE_CHROME_CHANNEL || undefined });
const context = await browser.newContext({ deviceScaleFactor: Number(opt.dpr), viewport: { width: 800, height: 600 } });
const page = await context.newPage();
await page.route("https://webfont.americanamap.org/noto/*", (route) => {
  const name = new URL(route.request().url()).pathname.split("/").pop();
  try {
    route.fulfill({
      body: readFileSync(join(resolve(opt.inputs), name)),
      contentType: "font/woff2",
      // Cross-origin web fonts are discarded without CORS approval.
      headers: { "access-control-allow-origin": "*" },
    });
  } catch {
    // Upstream fonts the pack does not ship (its letterless Armenian
    // PropNums build) come from the live host, as upstream sees them.
    route.continue();
  }
});
page.on("pageerror", (e) => console.error("page error:", e.message));
await page.goto(`${origin}/upstream/shieldtest.html`, { waitUntil: "load" });
await page.waitForFunction(() => window.map && window.map.loaded());

const result = await page.evaluate(
  async ({ dpr, limit, meanMax, fracMax }) => {
    const stack = '"Noto Sans Condensed", "Noto Sans Armenian Condensed", sans-serif-condensed, "Arial Narrow", sans-serif';
    await document.fonts.load(`condensed 500 12px ${stack}`);
    await document.fonts.ready;
    // Comparing against a fallback font would be meaningless.
    const noto = [...document.fonts].find((f) => f.family.replace(/"/g, "") === "Noto Sans Condensed");
    if (!noto || noto.status !== "loaded") {
      throw new Error(`Noto Sans Condensed did not load (status ${noto?.status ?? "missing"})`);
    }
    const wasm = await import("/wasm/roadshield_wasm.js");
    await wasm.default("/wasm/roadshield_wasm_bg.wasm");
    const shields = new wasm.RoadShield();
    for (const path of await (await fetch("/pack-index.json")).json()) {
      shields.addFile(path, new Uint8Array(await (await fetch(`/pack/${path}`)).arrayBuffer()));
    }
    shields.load();
    // FontFace status "loaded" does not prove canvas text uses the face (some
    // headless builds draw canvas text in a default font). Require the page's
    // canvas metrics to equal roadshield's for the pinned font.
    const probeCtx = document.createElement("canvas").getContext("2d");
    probeCtx.font = `condensed 500 11.8px ${stack}`;
    const pageWidth = probeCtx.measureText("287").width;
    const ourWidth = shields.measureText("287", 11.8);
    if (Math.abs(pageWidth - ourWidth) > 0.01) {
      throw new Error(`canvas text is not using the pinned webfont: "287" measures ${pageWidth}px, expected ${ourWidth}px`);
    }
    const rules = JSON.parse(shields.rules()).networks;

    const cases = [];
    const seen = new Set();
    const add = (network, ref, name) => {
      const k = `${network}\n${ref}\n${name}`;
      if (!seen.has(k)) {
        seen.add(k);
        cases.push({ network, ref, name });
      }
    };
    for (const [network, def] of Object.entries(rules)) {
      if (!def) continue;
      for (const r of ["1", "22", "287", "1234", "I-5", "10A"]) add(network, r, "");
      add(network, "", "");
      for (const r of Object.keys(def.overrideByRef ?? {})) add(network, r, "");
      for (const n of Object.keys(def.overrideByName ?? {})) add(network, "", n);
      for (const n of Object.keys(def.refsByName ?? {})) add(network, "", n);
    }
    for (const r of ["7", "95", "M-11", "Ε65"]) add("ZZ:unknown", r, "");
    const todo = limit > 0 ? cases.slice(0, limit) : cases;

    const premul = (d, i) => {
      const a = d[i + 3];
      return [(d[i] * a) / 255, (d[i + 1] * a) / 255, (d[i + 2] * a) / 255, a];
    };
    const toPng = (w, h, data) => {
      const c = document.createElement("canvas");
      c.width = w;
      c.height = h;
      c.getContext("2d").putImageData(new ImageData(new Uint8ClampedArray(data), w, h), 0, 0);
      return c.toDataURL("image/png");
    };

    const failures = [];
    const stats = { cases: todo.length, compared: 0, both_none: 0, worst_mean: 0, worst_frac: 0 };
    for (const c of todo) {
      const id = `shield\n${c.network}\n${c.ref}\n${c.name}\n`;
      map.fire(new maplibregl.Event("styleimagemissing", { id }));
      const img = map.style.getImage(id);
      const up = img?.data;
      const upNone = !up || (up.width === 1 && up.height === 1);
      const route = JSON.stringify({ network: c.network, ref: c.ref || null, name: c.name || null });
      let ours;
      try {
        ours = shields.renderRgba(route, JSON.stringify({ pixel_grid: dpr > 1 ? 2 : 1 }), dpr);
      } catch (e) {
        failures.push({ ...c, kind: "error", detail: String(e) });
        continue;
      }
      if (ours.length === 0 || upNone) {
        if (ours.length === 0 && upNone) stats.both_none++;
        else failures.push({ ...c, kind: "presence", upstream: upNone ? "none" : `${up.width}x${up.height}`, ours: ours.length ? "symbol" : "none" });
        continue;
      }
      const view = new DataView(ours.buffer, ours.byteOffset);
      const w = view.getUint32(0, true);
      const h = view.getUint32(4, true);
      const px = ours.subarray(8);
      if (w !== up.width || h !== up.height) {
        failures.push({ ...c, kind: "size", upstream: `${up.width}x${up.height}`, ours: `${w}x${h}`, upstream_png: toPng(up.width, up.height, up.data), ours_png: toPng(w, h, px) });
        continue;
      }
      let sum = 0;
      let big = 0;
      const diff = (pa, ia, pb, ib) => {
        const a = premul(pa, ia);
        const b = premul(pb, ib);
        return Math.max(...a.map((v, k) => Math.abs(v - b[k])));
      };
      // A pixel is unexplained when nothing within one device pixel in the
      // other image matches it: anti-aliasing and glyph-weight differences
      // stay local, geometric errors do not.
      const near = (pa, pb, x, y) => {
        for (let dy = -1; dy <= 1; dy++) {
          for (let dx = -1; dx <= 1; dx++) {
            const xx = x + dx;
            const yy = y + dy;
            if (xx < 0 || yy < 0 || xx >= w || yy >= h) continue;
            if (diff(pa, (y * w + x) * 4, pb, (yy * w + xx) * 4) <= 96) return true;
          }
        }
        return false;
      };
      for (let y = 0; y < h; y++) {
        for (let x = 0; x < w; x++) {
          const i = (y * w + x) * 4;
          const d = diff(px, i, up.data, i);
          sum += d;
          if (d > 128 && !(near(px, up.data, x, y) && near(up.data, px, x, y))) big++;
        }
      }
      const n = w * h;
      const mean = sum / n;
      const frac = big / n;
      stats.compared++;
      stats.worst_mean = Math.max(stats.worst_mean, mean);
      stats.worst_frac = Math.max(stats.worst_frac, frac);
      if (mean > meanMax || frac > fracMax) {
        failures.push({ ...c, kind: "pixels", mean, frac, upstream_png: toPng(w, h, up.data), ours_png: toPng(w, h, px) });
      }
    }
    return { stats, failures };
  },
  { dpr: Number(opt.dpr), limit: Number(opt.limit), meanMax, fracMax },
);

await browser.close();
server.close();

mkdirSync(opt.out, { recursive: true });
const tag = `dpr${opt.dpr}`;
writeFileSync(join(opt.out, `sweep-${tag}.json`), JSON.stringify(result, null, 1));
const rows = result.failures
  .map(
    (f) =>
      `<tr><td>${f.network}</td><td>${f.ref}</td><td>${f.name}</td><td>${f.kind} ${f.detail ?? ""}${f.mean !== undefined ? `mean ${f.mean.toFixed(1)} unexplained ${(f.frac * 100).toFixed(2)}%` : ""}${f.upstream ?? ""} / ${f.ours ?? ""}</td>` +
      `<td>${f.upstream_png ? `<img src="${f.upstream_png}">` : ""}</td><td>${f.ours_png ? `<img src="${f.ours_png}">` : ""}</td></tr>`,
  )
  .join("\n");
writeFileSync(
  join(opt.out, `sweep-${tag}.html`),
  `<!doctype html><meta charset="utf-8"><title>Oracle sweep ${tag}</title><style>body{font:13px sans-serif;background:#f6f1ea}img{image-rendering:pixelated;width:auto;height:60px}td{border:1px solid #ccc;padding:2px 6px}</style><h1>${result.failures.length} failures / ${result.stats.cases} cases (${tag})</h1><pre>${JSON.stringify(result.stats)}</pre><table><tr><th>network</th><th>ref</th><th>name</th><th>why</th><th>upstream</th><th>roadshield</th></tr>${rows}</table>`,
);
console.log(JSON.stringify({ ...result.stats, failures: result.failures.length }));
if (result.failures.length > 0) process.exitCode = 1;
