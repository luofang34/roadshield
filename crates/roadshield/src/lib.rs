//! Road shield engine: selects a shield definition for a route from
//! Americana-compatible ShieldJSON rule data and renders it as a
//! self-contained SVG with layout metadata.
//!
//! The engine performs no network or file I/O; rule packs, SVG blanks and
//! fonts are supplied by the caller through a [`ResourceResolver`].
//!
//! ```no_run
//! # fn demo(resolver: &dyn roadshield::ResourceResolver) -> Result<(), Box<dyn std::error::Error>> {
//! use roadshield::{DisplayContext, Engine, Rendering, ResourcePack, RouteDescriptor};
//! let engine = Engine::new(ResourcePack::load(resolver)?)?;
//! let route = RouteDescriptor::new("US:I", "287");
//! if let Rendering::Symbol(symbol) = engine.render(&route, &DisplayContext::default())? {
//!     assert!(symbol.svg.starts_with("<svg"));
//! }
//! # Ok(()) }
//! ```

mod blank;
mod color;
mod compose;
mod document;
mod engine;
mod error;
mod font;
mod geometry;
mod key;
mod model;
mod pack;
mod route;
mod select;
mod shapes;
mod svg;
mod symbol;
#[cfg(test)]
mod testing;
mod text_layout;
mod validate;

pub use color::{Recolor, Rgba};
pub use engine::{Engine, TextMetrics};
pub use error::{NoShieldReason, PackError, ShieldError, Warning};
pub use geometry::Rect;
pub use key::{ENGINE_OUTPUT_VERSION, semantic_key};
pub use model::{
    Padding, ShapeBlank, ShapeParams, ShieldDef, ShieldOptions, ShieldSpec, SpriteBlank,
    TextLayoutDef, TextLayoutOptions,
};
pub use pack::{
    BlankEntry, FileRef, FontEntry, LicenseEntry, MANIFEST_FORMAT, MANIFEST_PATH, Manifest,
    ResourcePack, ResourceResolver, SourceRef, Subset, Upstream, UpstreamFile,
};
pub use route::{
    Accessibility, DisplayContext, InputLimits, MissingGlyphPolicy, PackExpectation,
    RouteDescriptor, TextDirection, TextHaloJoin, UnknownNetworkPolicy,
};
pub use select::{AppliedOverride, MAX_REF_UTF16, is_valid_ref, romanize};
pub use shapes::SHAPES;
pub use symbol::{
    Dependency, DependencyKind, Provenance, Rendering, RuleInfo, ShieldSymbol, TextInfo,
};
pub use text_layout::CONSTRAINTS;
pub use validate::{IssueKind, RuleIssue};
