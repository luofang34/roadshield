//! `RoadShield` JavaScript class.

use std::collections::HashMap;

use roadshield::{DisplayContext, Engine, Rendering, ResourcePack, RouteDescriptor};
use roadshield_raster::{AlphaMode, RasterOptions, rasterize};
use wasm_bindgen::prelude::*;

fn js_err(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}

/// A pack being assembled from files, then a loaded engine.
#[wasm_bindgen]
#[derive(Default)]
pub struct RoadShield {
    files: HashMap<String, Vec<u8>>,
    engine: Option<Engine>,
}

#[wasm_bindgen]
impl RoadShield {
    /// Empty instance; add pack files, then call `load`.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> RoadShield {
        RoadShield::default()
    }

    /// Adds one pack file by its path relative to the pack root.
    #[wasm_bindgen(js_name = addFile)]
    pub fn add_file(&mut self, path: String, bytes: Vec<u8>) {
        self.files.insert(path, bytes);
    }

    /// Verifies and loads the added files; returns the manifest as JSON.
    ///
    /// # Errors
    ///
    /// Throws the pack error message if the files do not form a valid pack.
    pub fn load(&mut self) -> Result<String, JsValue> {
        let pack = ResourcePack::load(&self.files).map_err(js_err)?;
        let engine = Engine::new(pack).map_err(js_err)?;
        let manifest = serde_json::to_string(engine.manifest()).map_err(js_err)?;
        self.files.clear();
        self.engine = Some(engine);
        Ok(manifest)
    }

    fn engine(&self) -> Result<&Engine, JsValue> {
        self.engine
            .as_ref()
            .ok_or_else(|| JsValue::from_str("pack not loaded"))
    }

    /// Network keys with rules (after `bannerMap` expansion) as JSON.
    ///
    /// # Errors
    ///
    /// Throws if no pack is loaded.
    pub fn networks(&self) -> Result<String, JsValue> {
        let keys: Vec<&str> = self.engine()?.networks().collect();
        serde_json::to_string(&keys).map_err(js_err)
    }

    /// Network keys the pack's extension rules define (no upstream oracle
    /// exists for them unless upstream adopts them), as JSON.
    ///
    /// # Errors
    ///
    /// Throws if no pack is loaded.
    #[wasm_bindgen(js_name = extensionNetworks)]
    pub fn extension_networks(&self) -> Result<String, JsValue> {
        let engine = self.engine()?;
        let keys: Vec<&str> = engine
            .networks()
            .filter(|n| engine.rule_origin(n) == Some(roadshield::RuleOrigin::Extension))
            .collect();
        serde_json::to_string(&keys).map_err(js_err)
    }

    /// The expanded rules (`ShieldSpec`) as JSON.
    ///
    /// # Errors
    ///
    /// Throws if no pack is loaded.
    pub fn rules(&self) -> Result<String, JsValue> {
        serde_json::to_string(self.engine()?.rules()).map_err(js_err)
    }

    /// Advance width of `text` at `font_px` with the pack's font stack, as
    /// canvas `measureText().width` would report it.
    ///
    /// # Errors
    ///
    /// Throws if no pack is loaded or no font covers a character.
    #[wasm_bindgen(js_name = measureText)]
    pub fn measure_text(&self, text: &str, font_px: f64) -> Result<f64, JsValue> {
        let m = self
            .engine()?
            .measure_text(text, font_px, &DisplayContext::default())
            .map_err(js_err)?;
        Ok(m.width)
    }

    /// Renders a route (`RouteDescriptor` JSON) with an optional
    /// `DisplayContext` JSON; returns the `Rendering` as JSON. Engine errors
    /// are thrown as JSON strings of `ShieldError`.
    ///
    /// # Errors
    ///
    /// Throws if no pack is loaded or the JSON is invalid; engine errors are
    /// thrown as `ShieldError` JSON.
    pub fn render(
        &self,
        route_json: &str,
        context_json: Option<String>,
    ) -> Result<String, JsValue> {
        let (route, ctx) = parse(route_json, context_json)?;
        match self.engine()?.render(&route, &ctx) {
            Ok(r) => serde_json::to_string(&r).map_err(js_err),
            Err(e) => Err(JsValue::from_str(
                &serde_json::to_string(&e).map_err(js_err)?,
            )),
        }
    }

    /// Renders straight-alpha sRGB RGBA at `pixel_ratio`. Returns
    /// `[width, height, ...pixels]` packed as bytes after an 8-byte header
    /// (two little-endian u32), or an empty array when no shield is shown.
    ///
    /// # Errors
    ///
    /// Throws if no pack is loaded, the JSON is invalid, rendering fails or the
    /// raster options are out of range.
    #[wasm_bindgen(js_name = renderRgba)]
    pub fn render_rgba(
        &self,
        route_json: &str,
        context_json: Option<String>,
        pixel_ratio: f32,
    ) -> Result<Vec<u8>, JsValue> {
        let (route, ctx) = parse(route_json, context_json)?;
        let rendering = self.engine()?.render(&route, &ctx).map_err(js_err)?;
        let Rendering::Symbol(symbol) = rendering else {
            return Ok(Vec::new());
        };
        let opts = RasterOptions {
            pixel_ratio,
            alpha: AlphaMode::Straight,
            ..RasterOptions::default()
        };
        let img = rasterize(&symbol, &opts).map_err(js_err)?;
        let mut out = Vec::with_capacity(8 + img.data.len());
        out.extend_from_slice(&img.width.to_le_bytes());
        out.extend_from_slice(&img.height.to_le_bytes());
        out.extend_from_slice(&img.data);
        Ok(out)
    }
}

fn parse(
    route_json: &str,
    context_json: Option<String>,
) -> Result<(RouteDescriptor, DisplayContext), JsValue> {
    let route: RouteDescriptor = serde_json::from_str(route_json).map_err(js_err)?;
    let ctx = match context_json {
        Some(c) => serde_json::from_str(&c).map_err(js_err)?,
        None => DisplayContext::default(),
    };
    Ok((route, ctx))
}
