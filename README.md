# roadshield

Road shield symbols from route attributes, compatible with the
[OSM Americana](https://github.com/osm-americana/openstreetmap-americana)
ShieldJSON rules. One Rust engine serves native, WebAssembly, server and
offline use; it outputs self-contained SVG plus layout metadata, and an
optional raster adapter produces RGBA.

The engine does no file or network I/O. Rules, SVG blanks and fonts come
from a hash-verified resource pack supplied by the caller.

## Crates

| crate | purpose |
|---|---|
| `roadshield` | engine: rule selection, text shaping/fitting, shapes, SVG, diagnostics |
| `roadshield-raster` | RGBA rasterisation (resvg), PNG encoding, raster cache keys |
| `roadshield-import` | pack builder, subset cutter, pack diff, disk loading |
| `roadshield-cli` | `roadshield` binary |
| `roadshield-wasm` | wasm-bindgen bindings |

## Quick start

```sh
cargo build --release
./target/release/roadshield render --network US:I --ref 287 > i287.svg
./target/release/roadshield render --network US:NJ:CR --ref 609 --format png --dpr 2 --out cr609.png
./target/release/roadshield render --network US:I --ref 287 --json   # metadata
```

`packs/americana/` is part of the repository, so rendering needs no network access.

## Library

```rust
use roadshield::{DisplayContext, Engine, Rendering, ResourcePack, RouteDescriptor};

let files = roadshield_import::load_pack_dir_blocking("packs/americana".as_ref())?;
let engine = Engine::new(ResourcePack::load(&files)?)?;
match engine.render(&RouteDescriptor::new("US:I", "287"), &DisplayContext::default())? {
    Rendering::Symbol(s) => tracing::info!("{}x{} key {}", s.width, s.height, s.semantic_key),
    Rendering::NoShield { reason, .. } => tracing::info!("no shield: {reason:?}"),
}
```

`ResourcePack::load` takes any `ResourceResolver`, so packs can come from
memory, an archive or a fetch layer. Unknown networks use the pack's explicit
`default` rule (recorded as a warning) or fail with
`UnknownNetworkPolicy::Unsupported`; missing blanks, missing glyphs,
unsupported rules, invalid input and pack mismatches are distinct
`ShieldError`s.

See `cargo run -p roadshield-import --example native`.

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
