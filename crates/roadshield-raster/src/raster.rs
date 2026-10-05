//! Rasterisation with resvg/tiny-skia.

use roadshield::ShieldSymbol;

/// Alpha convention of the output pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AlphaMode {
    /// Colour channels multiplied by alpha (what GPU atlases usually want).
    #[default]
    Premultiplied,
    /// Unassociated alpha (what `ImageData` and PNG use).
    Straight,
}

/// Raster parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RasterOptions {
    /// Device pixels per logical pixel; must be in `(0, 8]`.
    pub pixel_ratio: f32,
    /// Alpha convention.
    pub alpha: AlphaMode,
    /// Refuse outputs larger than this many pixels.
    pub max_pixels: u32,
}

impl Default for RasterOptions {
    fn default() -> Self {
        Self {
            pixel_ratio: 1.0,
            alpha: AlphaMode::Premultiplied,
            max_pixels: 1 << 20,
        }
    }
}

/// RGBA8 pixels in the sRGB colour space, rows top to bottom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RasterImage {
    /// Width in device pixels.
    pub width: u32,
    /// Height in device pixels.
    pub height: u32,
    /// Device pixels per logical pixel (for sprite metadata).
    pub pixel_ratio_milli: u32,
    /// Alpha convention of `data`.
    pub alpha: AlphaMode,
    /// `width * height * 4` bytes.
    pub data: Vec<u8>,
    /// Cache key for these pixels.
    pub raster_key: String,
}

/// Raster failures.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RasterError {
    /// Options are out of range.
    #[error("invalid raster options: {0}")]
    Options(String),
    /// Output would exceed `max_pixels`.
    #[error("{width}x{height} exceeds the {max} pixel limit")]
    TooLarge {
        /// Width.
        width: u32,
        /// Height.
        height: u32,
        /// Limit.
        max: u32,
    },
    /// The SVG could not be parsed.
    #[error("symbol {key} SVG does not parse: {detail}")]
    Parse {
        /// Semantic key of the symbol.
        key: String,
        /// Parser message.
        detail: String,
    },
    /// PNG encoding failed.
    #[error("PNG encoding failed: {0}")]
    Encode(String),
}

/// Raster cache key: the semantic key plus every raster parameter.
#[must_use]
pub fn raster_key(semantic_key: &str, opts: &RasterOptions) -> String {
    let alpha = match opts.alpha {
        AlphaMode::Premultiplied => "pm",
        AlphaMode::Straight => "st",
    };
    format!("{semantic_key}@{}x-srgb-{alpha}", opts.pixel_ratio)
}

fn device_size(logical: f64, ratio: f32) -> u32 {
    let v = (logical * f64::from(ratio)).round().max(1.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let out = v.min(f64::from(u32::MAX)) as u32;
    out
}

/// Renders `symbol` at `opts.pixel_ratio`.
///
/// # Errors
///
/// Fails with [`RasterError::Options`] for a pixel ratio outside `(0, 8]`,
/// [`RasterError::TooLarge`] when the image would exceed `max_pixels`, and
/// [`RasterError::Parse`] if the symbol's SVG does not parse.
pub fn rasterize(symbol: &ShieldSymbol, opts: &RasterOptions) -> Result<RasterImage, RasterError> {
    if !(opts.pixel_ratio.is_finite() && opts.pixel_ratio > 0.0 && opts.pixel_ratio <= 8.0) {
        return Err(RasterError::Options(format!(
            "pixel_ratio {} not in (0, 8]",
            opts.pixel_ratio
        )));
    }
    let width = device_size(symbol.width, opts.pixel_ratio);
    let height = device_size(symbol.height, opts.pixel_ratio);
    if u64::from(width) * u64::from(height) > u64::from(opts.max_pixels) {
        return Err(RasterError::TooLarge {
            width,
            height,
            max: opts.max_pixels,
        });
    }
    let parse_opts = usvg::Options {
        resources_dir: None,
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: usvg::ImageHrefResolver::default_data_resolver(),
            resolve_string: Box::new(|_, _| None),
        },
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_str(&symbol.svg, &parse_opts).map_err(|e| RasterError::Parse {
        key: symbol.semantic_key.clone(),
        detail: e.to_string(),
    })?;
    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| RasterError::Options(format!("{width}x{height}")))?;
    let size = tree.size();
    let sx = f64::from(width) / f64::from(size.width());
    let sy = f64::from(height) / f64::from(size.height());
    #[allow(clippy::cast_possible_truncation)]
    let transform = tiny_skia::Transform::from_scale(sx as f32, sy as f32);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    let data = match opts.alpha {
        AlphaMode::Premultiplied => pixmap.data().to_vec(),
        AlphaMode::Straight => pixmap
            .pixels()
            .iter()
            .flat_map(|p| {
                let c = p.demultiply();
                [c.red(), c.green(), c.blue(), c.alpha()]
            })
            .collect(),
    };
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let pixel_ratio_milli = (opts.pixel_ratio * 1000.0).round() as u32;
    Ok(RasterImage {
        width,
        height,
        pixel_ratio_milli,
        alpha: opts.alpha,
        data,
        raster_key: raster_key(&symbol.semantic_key, opts),
    })
}

/// Encodes a raster as PNG (straight alpha, sRGB).
///
/// # Errors
///
/// Fails with [`RasterError::Encode`] for an empty image or an encoder error.
pub fn encode_png(img: &RasterImage) -> Result<Vec<u8>, RasterError> {
    let mut pixmap = tiny_skia::Pixmap::new(img.width, img.height)
        .ok_or_else(|| RasterError::Encode(format!("{}x{}", img.width, img.height)))?;
    for (dst, &[r, g, b, a]) in pixmap
        .pixels_mut()
        .iter_mut()
        .zip(img.data.as_chunks::<4>().0)
    {
        *dst = match img.alpha {
            AlphaMode::Premultiplied => tiny_skia::PremultipliedColorU8::from_rgba(r, g, b, a)
                .unwrap_or(tiny_skia::PremultipliedColorU8::TRANSPARENT),
            AlphaMode::Straight => tiny_skia::ColorU8::from_rgba(r, g, b, a).premultiply(),
        };
    }
    pixmap
        .encode_png()
        .map_err(|e| RasterError::Encode(e.to_string()))
}

#[cfg(test)]
mod tests;
