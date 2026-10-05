//! Assembles a selected rule into SVG, mirroring `generateShieldCtx`.
//!
//! Draw order matches upstream: banner halos, blank or shape, text halo,
//! text, banner text.

use std::collections::HashMap;

use crate::blank::PreparedBlank;
use crate::color::{Recolor, Rgba};
use crate::error::{ShieldError, Warning};
use crate::font::{FontStack, ShapedText};
use crate::geometry::{Rect, num};
use crate::model::{Padding, ShieldDef, ShieldOptions, TextLayoutDef};
use crate::route::{DisplayContext, MissingGlyphPolicy, TextHaloJoin};
use crate::select::{Selection, choose_blank, romanize};
use crate::shapes::{self, ShapeEnv};
use crate::svg;
use crate::symbol::TextInfo;
use crate::text_layout::{self, DEFAULT_MAX_FONT_SIZE, LayoutError, TextPlacement};

/// Shared inputs for one render.
pub(crate) struct ComposeEnv<'a> {
    pub options: &'a ShieldOptions,
    pub stack: &'a FontStack,
    pub blanks: &'a HashMap<String, PreparedBlank>,
    pub ctx: &'a DisplayContext,
    pub network: &'a str,
    /// Device pixels per layout pixel; all geometry is in device pixels.
    pub r: f64,
}

/// Unscaled composition result.
pub(crate) struct Composed {
    pub body: String,
    pub width: f64,
    pub height: f64,
    pub banner_area: f64,
    pub shield_height: f64,
    pub text: Option<TextInfo>,
    pub banners: Vec<TextInfo>,
    pub blank: Option<String>,
    pub shape: Option<String>,
    pub faces_used: Vec<usize>,
    pub warnings: Vec<Warning>,
    pub raster: bool,
}

impl ComposeEnv<'_> {
    fn unsupported(&self, detail: String) -> ShieldError {
        ShieldError::UnsupportedRule {
            network: self.network.into(),
            detail,
        }
    }

    fn color(&self, css: &str) -> Result<Rgba, ShieldError> {
        Rgba::parse(css).ok_or_else(|| self.unsupported(format!("invalid colour {css:?}")))
    }

    fn shape_text(
        &self,
        text: &str,
        policy: MissingGlyphPolicy,
    ) -> Result<ShapedText, ShieldError> {
        self.stack
            .shape(
                text,
                self.ctx.direction,
                self.ctx.language.as_deref(),
                policy,
            )
            .map_err(|e| match e {
                ShieldError::Render { detail, .. } => ShieldError::Render {
                    network: self.network.into(),
                    detail,
                },
                other => other,
            })
    }

    fn top_em(&self) -> f64 {
        self.stack
            .primary()
            .map_or(0.8, crate::font::FontFace::top_em)
    }

    fn layout(
        &self,
        shaped: &ShapedText,
        padding: Padding,
        bounds: (f64, f64),
        def: &TextLayoutDef,
        max_font: f64,
    ) -> Result<TextPlacement, ShieldError> {
        text_layout::layout(shaped, self.top_em(), padding, bounds, def, max_font).map_err(|e| {
            match e {
                LayoutError::UnknownConstraint(c) => {
                    self.unsupported(format!("textLayout.constraintFunc {c:?}"))
                }
                LayoutError::NoFit(px) => ShieldError::Render {
                    network: self.network.into(),
                    detail: format!("text does not fit (font size {px})"),
                },
            }
        })
    }

    /// Glyph path plus ink metadata for text at a placement; `dy` shifts the
    /// metadata into absolute coordinates.
    fn text_path(
        &self,
        shaped: &ShapedText,
        text: &str,
        p: TextPlacement,
        dy: f64,
    ) -> (String, TextInfo) {
        let left = p.x_center - shaped.advance * p.font_px / 2.0;
        let baseline = p.y_top + crate::font::top_px(self.top_em(), p.font_px);
        let d = self.stack.outline_path(shaped, p.font_px, left, baseline);
        let ink = shaped.ink.map_or(Rect::default(), |i| {
            Rect::new(
                left + i.min_x * p.font_px,
                dy + baseline - i.max_y * p.font_px,
                (i.max_x - i.min_x) * p.font_px,
                (i.max_y - i.min_y) * p.font_px,
            )
        });
        let info = TextInfo {
            text: text.into(),
            font_px: p.font_px,
            ink,
        };
        (d, info)
    }
}

fn halo_element(d: &str, color: Rgba, width: f64, join: TextHaloJoin) -> String {
    format!(
        "<path d=\"{d}\" fill=\"none\" {} {}/>",
        svg::stroke(color),
        svg::halo_stroke_style(width, join)
    )
}

fn fill_element(d: &str, color: Rgba) -> String {
    format!("<path d=\"{d}\" {}/>", svg::fill(color))
}

fn truthy(s: Option<&str>) -> Option<&str> {
    s.filter(|s| !s.is_empty())
}

struct Body {
    width: f64,
    height: f64,
    bounds: (f64, f64),
    elements: String,
    blank: Option<String>,
    shape: Option<String>,
    raster: bool,
}

fn sprite_body(
    env: &ComposeEnv<'_>,
    def: &ShieldDef,
    sprite: &crate::model::SpriteBlank,
    resolved_ref: &str,
) -> Result<Body, ShieldError> {
    let id = choose_blank(sprite, resolved_ref)
        .ok_or_else(|| env.unsupported("empty spriteBlank list".into()))?;
    let blank = env
        .blanks
        .get(id)
        .ok_or_else(|| ShieldError::MissingBlank {
            network: env.network.into(),
            blank: id.into(),
        })?;
    let recolor = Recolor::from_css(def.color_lighten.as_deref(), def.color_darken.as_deref())
        .map_err(|e| env.unsupported(e))?;
    let elements = blank
        .embed(0.0, def.vertical_reflect.unwrap_or(false), recolor, env.r)
        .map_err(|detail| ShieldError::Render {
            network: env.network.into(),
            detail: format!("embedding blank {id:?}: {detail}"),
        })?;
    Ok(Body {
        width: blank.width * env.r,
        height: blank.height * env.r,
        bounds: (blank.width * env.r, blank.height * env.r),
        elements,
        blank: Some(id.into()),
        shape: None,
        raster: blank.has_raster,
    })
}

fn shape_elements(env: &ComposeEnv<'_>, ops: Vec<shapes::DrawOp>) -> Result<String, ShieldError> {
    let mut elements = String::new();
    for op in ops {
        let fill = env.color(&op.fill)?;
        elements.push_str(&format!("<path d=\"{}\" {}", op.d, svg::fill(fill)));
        if let Some((stroke, w)) = &op.stroke {
            elements.push_str(&format!(
                " {} {}",
                svg::stroke(env.color(stroke)?),
                svg::stroke_style(*w)
            ));
        }
        elements.push_str("/>");
    }
    Ok(elements)
}

fn shape_body(
    env: &ComposeEnv<'_>,
    shape: &crate::model::ShapeBlank,
    resolved_ref: &str,
    display_ref: &str,
) -> Result<Body, ShieldError> {
    let size = env.options.shield_size * env.r;
    let advance =
        |t: &str, px: f64| Ok(env.shape_text(t, MissingGlyphPolicy::Notdef)?.advance * px);
    let measure_env = ShapeEnv {
        size,
        px: env.r,
        canvas_width: size,
        text: resolved_ref,
        advance: &advance,
    };
    let width = size.max(shapes::compute_width(
        &measure_env,
        &shape.params,
        Some(&shape.draw_func),
    )?);
    let height = shapes::shape_height(size, env.r, &shape.draw_func);
    // A canvas truncates fractional dimensions; text is still laid out in
    // the unrounded width, as upstream does.
    let canvas_width = width.floor();
    let draw_env = ShapeEnv {
        size,
        px: env.r,
        canvas_width,
        text: display_ref,
        advance: &advance,
    };
    let ops = shapes::draw(&draw_env, &shape.draw_func, &shape.params)?
        .ok_or_else(|| env.unsupported(format!("shapeBlank.drawFunc {:?}", shape.draw_func)))?;
    Ok(Body {
        width: canvas_width,
        height: height.floor(),
        bounds: (width, height),
        elements: shape_elements(env, ops)?,
        blank: None,
        shape: Some(shape.draw_func.clone()),
        raster: false,
    })
}

fn body(
    env: &ComposeEnv<'_>,
    def: &ShieldDef,
    resolved_ref: &str,
    display_ref: &str,
) -> Result<Body, ShieldError> {
    if let Some(sprite) = &def.sprite_blank {
        return sprite_body(env, def, sprite, resolved_ref);
    }
    if let Some(shape) = &def.shape_blank {
        return shape_body(env, shape, resolved_ref, display_ref);
    }
    let size = env.options.shield_size * env.r;
    Ok(Body {
        width: size,
        height: size,
        bounds: (size, size),
        elements: String::new(),
        blank: None,
        shape: None,
        raster: false,
    })
}

/// Fonts and notdef warnings accumulated across text runs.
#[derive(Default)]
struct TextUse {
    faces: Vec<usize>,
    warnings: Vec<Warning>,
}

impl TextUse {
    fn note(&mut self, s: &ShapedText) {
        for f in &s.faces_used {
            if !self.faces.contains(f) {
                self.faces.push(*f);
            }
        }
        self.warnings.extend(
            s.notdef
                .iter()
                .map(|&codepoint| Warning::NotdefDrawn { codepoint }),
        );
    }
}

fn rect_layout() -> TextLayoutDef {
    TextLayoutDef {
        constraint_func: "rect".into(),
        options: None,
    }
}

/// Banner rows: rect constraint across the full width, one row each.
fn banners(
    env: &ComposeEnv<'_>,
    texts: &[String],
    width: f64,
    used: &mut TextUse,
) -> Result<(Vec<String>, Vec<TextInfo>), ShieldError> {
    let opts = env.options;
    let r = env.r;
    let row_bounds = (width, (opts.banner_height - opts.banner_padding) * r);
    let mut paths = Vec::with_capacity(texts.len());
    let mut infos = Vec::with_capacity(texts.len());
    for (i, text) in texts.iter().enumerate() {
        let shaped = env.shape_text(text, env.ctx.missing_glyph)?;
        used.note(&shaped);
        let mut p = env.layout(
            &shaped,
            Padding::default(),
            row_bounds,
            &rect_layout(),
            DEFAULT_MAX_FONT_SIZE * r,
        )?;
        #[allow(clippy::cast_precision_loss)]
        let row = i as f64;
        p.y_top += row * (opts.banner_height + opts.banner_padding) * r;
        let (d, info) = env.text_path(&shaped, text, p, 0.0);
        paths.push(d);
        infos.push(info);
    }
    Ok((paths, infos))
}

/// Shield text with optional halo, in shield-body coordinates.
fn shield_text(
    env: &ComposeEnv<'_>,
    def: &ShieldDef,
    text: &str,
    bounds: (f64, f64),
    banner_area: f64,
    used: &mut TextUse,
) -> Result<(String, TextInfo), ShieldError> {
    let shaped = env.shape_text(text, env.ctx.missing_glyph)?;
    used.note(&shaped);
    let layout_def = def.text_layout.clone().unwrap_or_else(rect_layout);
    let max_font = def
        .max_font_size
        .map_or(DEFAULT_MAX_FONT_SIZE, |m| m.min(DEFAULT_MAX_FONT_SIZE))
        * env.r;
    let pad = def.padding.unwrap_or_default();
    let padding = Padding {
        left: pad.left * env.r,
        right: pad.right * env.r,
        top: pad.top * env.r,
        bottom: pad.bottom * env.r,
    };
    let p = env.layout(&shaped, padding, bounds, &layout_def, max_font)?;
    let (d, info) = env.text_path(&shaped, text, p, banner_area);
    let mut out = String::new();
    if !d.is_empty() {
        let forced = env.ctx.accessibility.force_text_halo.as_deref();
        if let Some(h) = forced
            .filter(|s| !s.is_empty())
            .or(truthy(def.text_halo_color.as_deref()))
        {
            out.push_str(&halo_element(
                &d,
                env.color(h)?,
                2.0 * env.r,
                env.ctx.text_halo_join,
            ));
        }
        let fill = env.color(truthy(def.text_color.as_deref()).unwrap_or("black"))?;
        out.push_str(&fill_element(&d, fill));
    }
    Ok((out, info))
}

/// Renders a selection without the root `<svg>` wrapper.
pub(crate) fn compose(env: &ComposeEnv<'_>, sel: &Selection) -> Result<Composed, ShieldError> {
    let def = &sel.def;
    let opts = env.options;
    let display_ref =
        if def.numbering_system.as_deref() == Some("roman") && !sel.resolved_ref.is_empty() {
            romanize(&sel.resolved_ref)
        } else {
            sel.resolved_ref.clone()
        };
    let b = body(env, def, &sel.resolved_ref, &display_ref)?;
    let texts: Vec<String> = def.banners.clone().unwrap_or_default();
    #[allow(clippy::cast_precision_loss)]
    let n = texts.len() as f64;
    let banner_area = if texts.is_empty() {
        0.0
    } else {
        (n * opts.banner_height + (n - 1.0) * opts.banner_padding) * env.r
    };
    let mut used = TextUse::default();
    let (banner_paths, banner_infos) = banners(env, &texts, b.width, &mut used)?;
    let banner_halo = env.color(
        truthy(def.banner_text_halo_color.as_deref()).unwrap_or(&opts.banner_text_halo_color),
    )?;
    let banner_fill =
        env.color(truthy(def.banner_text_color.as_deref()).unwrap_or(&opts.banner_text_color))?;

    let mut out = String::new();
    for d in banner_paths.iter().filter(|d| !d.is_empty()) {
        out.push_str(&halo_element(
            d,
            banner_halo,
            2.0 * env.r,
            env.ctx.text_halo_join,
        ));
    }
    out.push_str(&format!(
        "<g transform=\"translate(0 {})\">",
        num(banner_area)
    ));
    out.push_str(&b.elements);
    let mut text_info = None;
    if let Some(text) = &sel.text {
        let (elements, info) = shield_text(env, def, text, b.bounds, banner_area, &mut used)?;
        out.push_str(&elements);
        text_info = Some(info);
    }
    out.push_str("</g>");
    for d in banner_paths.iter().filter(|d| !d.is_empty()) {
        out.push_str(&fill_element(d, banner_fill));
    }
    if b.raster
        && let Some(id) = &b.blank
    {
        used.warnings
            .push(Warning::RasterContent { blank: id.clone() });
    }
    Ok(Composed {
        body: out,
        width: b.width,
        height: b.height + banner_area,
        banner_area,
        shield_height: b.height,
        text: text_info,
        banners: banner_infos,
        blank: b.blank,
        shape: b.shape,
        faces_used: used.faces,
        warnings: used.warnings,
        raster: b.raster,
    })
}
