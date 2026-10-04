//! SVG blank artwork: sanitised once at pack load, embedded per render.
//!
//! Blanks are parsed with usvg (no file or network access, external images
//! dropped, scripts ignored) and re-serialised in its canonical form, which
//! writes every paint as a plain attribute. Recolouring then rewrites paint
//! attributes outside `<mask>`/`<clipPath>`, which is exact because the
//! upstream colour transform is affine and alpha-preserving.

use std::sync::Arc;

use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, Writer};

use crate::color::{Recolor, Rgba};
use crate::error::PackError;
use crate::geometry::num;

/// Size limit for one blank SVG source.
pub const MAX_BLANK_BYTES: usize = 512 * 1024;
const MAX_NODES: usize = 5_000;

/// A sanitised blank ready for embedding.
#[derive(Debug, Clone)]
pub struct PreparedBlank {
    /// Logical size from the sprite sheet (1x pixels).
    pub width: f64,
    /// Logical height.
    pub height: f64,
    canonical: Arc<str>,
    /// Coordinate system of `canonical` (usvg normalises it to the
    /// document size).
    view_box: (f64, f64),
    /// True if the blank contains raster image data.
    pub has_raster: bool,
}

fn safe_options() -> usvg::Options<'static> {
    usvg::Options {
        resources_dir: None,
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: usvg::ImageHrefResolver::default_data_resolver(),
            resolve_string: Box::new(|_, _| None),
        },
        ..usvg::Options::default()
    }
}

fn count_nodes(g: &usvg::Group, raster: &mut bool) -> usize {
    let mut n = 1;
    for child in g.children() {
        n += match child {
            usvg::Node::Group(sub) => count_nodes(sub, raster),
            usvg::Node::Image(_) => {
                *raster = true;
                1
            }
            _ => 1,
        };
    }
    n
}

fn sanitize_id(id: &str) -> String {
    id.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

impl PreparedBlank {
    /// Parses and canonicalises a blank.
    pub fn prepare(id: &str, svg: &[u8], width: f64, height: f64) -> Result<Self, PackError> {
        let err = |detail: String| PackError::Blank {
            id: id.into(),
            detail,
        };
        if svg.len() > MAX_BLANK_BYTES {
            return Err(err(format!(
                "{} bytes exceeds {MAX_BLANK_BYTES}",
                svg.len()
            )));
        }
        if !(width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0) {
            return Err(err(format!("invalid size {width}x{height}")));
        }
        let tree = usvg::Tree::from_data(svg, &safe_options()).map_err(|e| err(e.to_string()))?;
        let mut has_raster = false;
        let nodes = count_nodes(tree.root(), &mut has_raster);
        if nodes > MAX_NODES {
            return Err(err(format!("{nodes} nodes exceeds {MAX_NODES}")));
        }
        let write = usvg::WriteOptions {
            id_prefix: Some(format!("rs-{}-", sanitize_id(id))),
            coordinates_precision: 4,
            transforms_precision: 6,
            ..usvg::WriteOptions::default()
        };
        let size = tree.size();
        Ok(Self {
            width,
            height,
            canonical: Arc::from(tree.to_string(&write)),
            view_box: (f64::from(size.width()), f64::from(size.height())),
            has_raster,
        })
    }

    /// Nested `<svg>` element placing the blank at `(0, y)`, optionally
    /// flipped vertically and recoloured.
    pub fn embed(
        &self,
        y: f64,
        reflect: bool,
        recolor: Option<Recolor>,
        scale: f64,
    ) -> Result<String, String> {
        let inner = rewrite(
            &self.canonical,
            (self.width * scale, self.height * scale),
            self.view_box,
            recolor,
        )?;
        Ok(if reflect {
            format!(
                "<g transform=\"translate(0 {}) scale(1 -1)\">{inner}</g>",
                num(y + self.height * scale)
            )
        } else if y == 0.0 {
            inner
        } else {
            format!("<g transform=\"translate(0 {})\">{inner}</g>", num(y))
        })
    }
}

const PAINT_ATTRS: &[&str] = &[
    "fill",
    "stroke",
    "stop-color",
    "flood-color",
    "lighting-color",
];

fn rewrite(
    svg: &str,
    size: (f64, f64),
    view_box: (f64, f64),
    recolor: Option<Recolor>,
) -> Result<String, String> {
    let mut reader = Reader::from_str(svg);
    let mut writer = Writer::new(Vec::new());
    let mut stack: Vec<String> = Vec::new();
    let mut root_done = false;
    loop {
        let event = reader.read_event().map_err(|e| e.to_string())?;
        let out = match event {
            Event::Eof => break,
            Event::Decl(_) | Event::PI(_) | Event::DocType(_) | Event::Comment(_) => continue,
            Event::Start(e) => {
                let name = e.name().as_ref().to_owned();
                let el = transform(&e, &stack, &mut root_done, size, view_box, recolor)?;
                stack.push(name);
                Event::Start(el)
            }
            Event::Empty(e) => Event::Empty(transform(
                &e,
                &stack,
                &mut root_done,
                size,
                view_box,
                recolor,
            )?),
            Event::End(e) => {
                stack.pop();
                Event::End(e)
            }
            other => other,
        };
        writer.write_event(out).map_err(|e| e.to_string())?;
    }
    String::from_utf8(writer.into_inner()).map_err(|e| e.to_string())
}

fn transform(
    e: &BytesStart<'_>,
    stack: &[String],
    root_done: &mut bool,
    size: (f64, f64),
    view_box: (f64, f64),
    recolor: Option<Recolor>,
) -> Result<BytesStart<'static>, String> {
    let name = e.name().as_ref().to_owned();
    let mut el = BytesStart::new(name.clone());
    if !*root_done && name == "svg" {
        *root_done = true;
        // Stretch to the sprite-sheet box exactly, as the raster sprite is.
        let vb = format!("0 0 {} {}", num(view_box.0), num(view_box.1));
        el.push_attribute(("width", num(size.0).as_str()));
        el.push_attribute(("height", num(size.1).as_str()));
        el.push_attribute(("viewBox", vb.as_str()));
        el.push_attribute(("preserveAspectRatio", "none"));
        el.push_attribute(("overflow", "visible"));
        return Ok(el);
    }
    let in_matte = stack.iter().any(|n| n == "mask" || n == "clipPath");
    // Caps never apply to closed subpaths; dropping them there is exact and
    // avoids a tiny-skia stroker fault that shifts strokes on closed paths
    // with round caps.
    let closed = name == "path"
        && e.attributes()
            .flatten()
            .any(|a| a.key.as_ref() == "d" && only_closed_subpaths(a.value.as_ref()));
    for a in e.attributes() {
        let a = a.map_err(|x| x.to_string())?;
        let key = a.key.as_ref().to_owned();
        if closed && key == "stroke-linecap" {
            continue;
        }
        let value = a
            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
            .map_err(|x| x.to_string())?
            .into_owned();
        let value = match recolor {
            Some(rc) if !in_matte && PAINT_ATTRS.contains(&key.as_str()) => {
                match Rgba::parse(&value) {
                    Some(c) if !value.starts_with("url(") && value != "none" => rc.apply(c).hex(),
                    _ => value,
                }
            }
            _ => value,
        };
        el.push_attribute((key.as_str(), value.as_str()));
    }
    Ok(el)
}

/// True when every subpath of `d` ends with a close command.
fn only_closed_subpaths(d: &str) -> bool {
    let d = d.trim();
    let mut prev = None;
    for c in d.chars().filter(|c| !c.is_whitespace() && *c != ',') {
        if matches!(c, 'M' | 'm') && prev.is_some_and(|p| !matches!(p, 'Z' | 'z')) {
            return false;
        }
        prev = Some(c);
    }
    matches!(prev, Some('Z' | 'z'))
}

#[cfg(test)]
mod tests;
