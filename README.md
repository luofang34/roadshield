# roadshield

[![CI](https://github.com/luofang34/roadshield/actions/workflows/ci.yml/badge.svg)](https://github.com/luofang34/roadshield/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/roadshield.svg)](https://crates.io/crates/roadshield)
[![docs.rs](https://docs.rs/roadshield/badge.svg)](https://docs.rs/roadshield)

Road shields for **OpenStreetMap** route relations, from network, ref and
name, in native Rust.

This is a port of the shield renderer of
[OpenStreetMap Americana](https://github.com/osm-americana/openstreetmap-americana)
(commit `a6817ef`), driven by its ShieldJSON rules and SVG artwork as data.
Its output agrees with Americana's TypeScript renderer on all 14,278
network/ref combinations the sweep generates, at 1x and 2x pixel ratio in
Chrome: same shields drawn or declined, identical image sizes, pixels within
anti-aliasing bounds.

![Shields rendered by roadshield](https://github.com/luofang34/roadshield/raw/main/docs/images/shields.svg)

```rust
use std::collections::HashMap;
use std::path::Path;

use roadshield::{DisplayContext, Engine, Rendering, ResourcePack, RouteDescriptor};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Every file of the pack, keyed by its path relative to the pack root.
    let root = Path::new("packs/americana");
    let mut files = HashMap::new();
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                dirs.push(path);
            } else {
                let key = path
                    .strip_prefix(root)?
                    .to_string_lossy()
                    .replace('\\', "/");
                files.insert(key, std::fs::read(&path)?);
            }
        }
    }

    let engine = Engine::new(ResourcePack::load(&files)?)?;
    let route = RouteDescriptor::new("US:NJ:CR", "609");
    if let Rendering::Symbol(shield) = engine.render(&route, &DisplayContext::default())? {
        std::fs::write("cr609.svg", &shield.svg)?;
    }
    Ok(())
}
```

## Summary

- Americana's rule semantics: network rules, `overrideByRef`/`overrideByName`
  ordering, `noref`, name-to-ref maps, banner variants (`bannerMap`),
  width-dependent blanks, Roman numerals, recolouring and reflection.
- All 14 upstream shapes and 6 text-fitting constraints, with canvas text
  metrics reproduced to within 0.0005 px of Chrome's `measureText`.
- Self-contained SVG (text as glyph outlines, original text kept for
  accessibility) plus layout metadata: size, shield box, text and banner
  boxes, anchor, rule used, versions, dependencies and a deterministic cache
  key.
- Typed outcomes: rules that decline to draw return `NoShield`; unknown
  networks, missing blanks or glyphs, unsupported rules, invalid input and
  pack mismatches are distinct errors, never a blank image.
- No file or network I/O in the engine: rules, blanks and fonts come from a
  hash-verified resource pack. Builds for `wasm32`; `Engine` is
  `Send + Sync` and every render is a pure function of its inputs.
- Fonts: Noto Sans Condensed Medium for Latin, Greek and Cyrillic, plus Noto
  Sans Armenian and Georgian Condensed Medium for refs in those scripts.
- About 20,000 shields per second to SVG on one core.

## Getting started

```toml
[dependencies]
roadshield = "0.1"
```

Requires Rust 1.88 or later.

The engine needs a resource pack. The Americana pack is in this repository
at [`packs/americana`](packs/americana) (CC0-1.0 rules and artwork, OFL-1.1
fonts) and is not on crates.io; copy that directory, or build your own with
`roadshield import` (see below).

## Examples

```sh
cargo run -p roadshield --example readme          # the code above; writes cr609.svg
cargo run -p roadshield --example concurrency > concurrency.svg   # a row of shields for one road, on the 2x grid
cargo run -p roadshield --example diagnostics     # every kind of outcome, from symbols to typed errors
cargo run -p roadshield --example readme_images   # regenerates docs/images
```

They read the pack from `ROADSHIELD_PACK`, defaulting to this repository's
`packs/americana`.

## Crates

| crate | purpose |
|---|---|
| `roadshield` | engine: rule selection, text shaping and fitting, shapes, SVG, diagnostics |
| `roadshield-raster` | RGBA rasterisation (resvg), PNG encoding, raster cache keys |
| `roadshield-import` | pack builder, subset cutter, pack diff, disk loading |
| `roadshield-cli` | `roadshield` binary |
| `roadshield-wasm` | wasm-bindgen bindings |

Only `roadshield` is published; the others are used from this repository.

## Command line

```sh
cargo build --release
./target/release/roadshield render --network US:I --ref 287 > i287.svg
./target/release/roadshield render --network US:NJ:CR --ref 609 --format png --dpr 2 --out cr609.png
./target/release/roadshield render --network US:I --ref 287 --json   # metadata
```

`ResourcePack::load` takes any `ResourceResolver`, so packs can come from
memory, an archive or a fetch layer. Unknown networks use the pack's explicit
`default` rule (recorded as a warning) or fail with
`UnknownNetworkPolicy::Unsupported`.

## WebAssembly

```sh
wasm-pack build crates/roadshield-wasm --release --target nodejs --out-dir ../../target/wasm-node
node examples/wasm/render.mjs packs/americana US:I 287 > i287.svg
```

`RoadShield.addFile(path, bytes)` for each pack file, then `load()`,
`render(routeJson, contextJson?)` and `renderRgba(routeJson, contextJson?, pixelRatio)`.

## Batch rendering

```sh
roadshield batch --input routes.jsonl --out-dir out --format both --dpr 2 \
  --max-items 100000 --time-limit 60 --jobs 8
```

Each input line is a `RouteDescriptor` (`{"network": "US:I", "ref": "287"}`).
Outputs are deduplicated by semantic key; `out/index.jsonl` records the
result, rule or error for every line.

## Updating the Americana pack

Pins live in `packs/americana.import.json` (upstream commit and SHA-256 of
every input); the HTTP inputs themselves are committed in
`packs/americana.inputs/` so the pack rebuilds byte-for-byte offline.

```sh
./scripts/fetch-upstream.sh          # check out the pinned commit into .upstream/; runs no upstream code
./scripts/fetch-upstream.sh --refresh-inputs   # re-download inputs, then update the pins
roadshield import                    # verify pins, check source inventory, build packs/americana
roadshield diff old-pack packs/americana --html diff.html
roadshield subset --networks US:NJ,US:I --out packs/nj
roadshield inspect
```

The import fails if upstream declares a shape, text constraint, numbering
system or definition field the engine does not implement, if ShieldJSON has
unknown fields, or if a referenced blank is missing.

## Verification

```sh
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test --all-targets
```

Conformance against the upstream TypeScript renderer, at the pinned commit:

```sh
./scripts/fetch-upstream.sh
./oracle/prepare-upstream.sh   # npm build of upstream; checks generated ShieldJSON == pack input
wasm-pack build crates/roadshield-wasm --release --target web --out-dir ../../target/wasm-web
export ORACLE_CHROME_CHANNEL=chrome   # Google Chrome; Playwright's headless shell ignores web fonts on canvas
node oracle/sweep.mjs --dpr 1 && node oracle/sweep.mjs --dpr 2   # reports in target/oracle/
```

The sweep opens upstream's `shieldtest.html` in Chromium and compares every
network (about 14,000 cases) with roadshield's output at the same device
pixel ratio: presence, exact image size, and pixels within anti-aliasing
bounds. The 2x run is the geometric gate. A small offline fixture of the
same comparison runs in `cargo test`.

Set `DisplayContext::pixel_grid` to 2 when rasterising for DPR > 1, as
MapLibre uses 2x sprites there; geometry is then rounded on the 2x grid as
upstream does.

## Licences

roadshield is licensed under AGPL-3.0-or-later (`LICENSE`). Pack contents keep
their own licences, listed in `packs/americana/manifest.json` with texts in
`packs/americana/licenses/`: Americana rules and blanks are CC0-1.0; the Noto
fonts are OFL-1.1.

Fonts: Americana's Noto Sans Condensed Medium (proportional numerals) for
Latin, Greek and Cyrillic, plus Noto Sans Armenian and Noto Sans Georgian
Condensed Medium for route refs in those scripts. Text no font covers fails
with `ShieldError::MissingGlyph` rather than drawing boxes.
