//! CPU raster adapter: turns a roadshield SVG symbol into RGBA pixels at a
//! chosen device pixel ratio, with an explicit colour space and alpha
//! convention, and a raster cache key separate from the semantic key.

mod raster;

pub use raster::{
    AlphaMode, RasterError, RasterImage, RasterOptions, encode_png, raster_key, rasterize,
};
