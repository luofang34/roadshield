//! Font loading, fallback, shaping (HarfRust) and glyph outlines (Skrifa).
//!
//! Metrics are kept in em units (font units / unitsPerEm) so one shaping
//! pass serves every font size the layout needs.

use std::str::FromStr;
use std::sync::Arc;

use harfrust::{Buffer, Direction, Language, ShapeOptions, ShaperFont};
use skrifa::MetadataProvider;
use skrifa::instance::Size;
use skrifa::outline::OutlinePen;

use crate::error::{PackError, ShieldError};
use crate::geometry::num;
use crate::route::{MissingGlyphPolicy, TextDirection};

/// A parsed font face.
#[derive(Clone)]
pub struct FontFace {
    id: String,
    data: Arc<Vec<u8>>,
    font: harfrust::Font,
    upem: f64,
    top_em: f64,
}

impl std::fmt::Debug for FontFace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FontFace")
            .field("id", &self.id)
            .field("upem", &self.upem)
            .finish()
    }
}

impl FontFace {
    /// Parses a TrueType/OpenType font.
    pub fn parse(id: &str, data: Vec<u8>) -> Result<Self, PackError> {
        let data = Arc::new(data);
        let blob: Arc<dyn AsRef<[u8]> + Send + Sync> = data.clone();
        let font = harfrust::Font::new(blob, 0).ok_or_else(|| PackError::Font { id: id.into() })?;
        skrifa::FontRef::new(data.as_slice()).map_err(|_| PackError::Font { id: id.into() })?;
        let upem = f64::from(font.units_per_em());
        if upem <= 0.0 {
            return Err(PackError::Font { id: id.into() });
        }
        let top_em =
            normalized_typo_ascent(&font, upem).ok_or_else(|| PackError::Font { id: id.into() })?;
        Ok(Self {
            id: id.into(),
            data,
            font,
            upem,
            top_em,
        })
    }

    /// Distance from the em-box top to the alphabetic baseline, in em.
    pub fn top_em(&self) -> f64 {
        self.top_em
    }

    fn covers(&self, c: char) -> bool {
        self.font
            .charmap()
            .map_unicode(c)
            .is_some_and(|g| g.to_u32() != 0)
    }
}

/// Canvas `textBaseline = "top"` position as Blink computes it: typo
/// ascent normalised so ascent + |descent| equals one em, falling back to
/// hhea metrics.
fn normalized_typo_ascent(font: &harfrust::Font, upem: f64) -> Option<f64> {
    let m = font.metrics();
    let pick = |asc: f64, desc: f64| {
        let total = asc + desc.abs();
        (total > 0.0).then(|| asc / total)
    };
    let typo = m.typo_line.as_ref().and_then(|l| {
        pick(
            f64::from(l.ascender.to_f32()),
            f64::from(l.descender.to_f32()),
        )
    });
    let hhea = || {
        m.hhea_line.as_ref().and_then(|l| {
            pick(
                f64::from(l.ascender.to_f32()),
                f64::from(l.descender.to_f32()),
            )
        })
    };
    typo.or_else(hhea).or((upem > 0.0).then_some(0.8))
}

/// Distance in px from the em-box top to the alphabetic baseline at `size`.
/// Blink stores font heights as LayoutUnit (1/64 px), so the value is
/// rounded to the nearest 1/64 as canvas `textBaseline = "top"` sees it.
pub fn top_px(top_em: f64, size: f64) -> f64 {
    (top_em * size * 64.0).round() / 64.0
}

/// Ink bounds in em units, y up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InkBox {
    /// Left edge.
    pub min_x: f64,
    /// Bottom edge.
    pub min_y: f64,
    /// Right edge.
    pub max_x: f64,
    /// Top edge.
    pub max_y: f64,
}

impl InkBox {
    fn union(self, o: InkBox) -> InkBox {
        InkBox {
            min_x: self.min_x.min(o.min_x),
            min_y: self.min_y.min(o.min_y),
            max_x: self.max_x.max(o.max_x),
            max_y: self.max_y.max(o.max_y),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct PlacedGlyph {
    face: usize,
    gid: harfrust::GlyphId,
    /// Origin in em, relative to the text start, y up.
    x: f64,
    y: f64,
}

/// Shaped, unscaled text.
#[derive(Debug, Clone)]
pub struct ShapedText {
    glyphs: Vec<PlacedGlyph>,
    /// Total advance in em.
    pub advance: f64,
    /// Union of glyph ink boxes in em; `None` when nothing has ink.
    pub ink: Option<InkBox>,
    /// Faces used, by index into the stack.
    pub faces_used: Vec<usize>,
    /// Code points drawn as `.notdef`.
    pub notdef: Vec<u32>,
}

#[cfg(test)]
impl ShapedText {
    /// Text with the given metrics and no glyphs, for layout tests.
    pub(crate) fn synthetic(advance: f64, ink: Option<InkBox>) -> Self {
        Self {
            glyphs: Vec::new(),
            advance,
            ink,
            faces_used: Vec::new(),
            notdef: Vec::new(),
        }
    }
}

/// An ordered fallback list of faces.
#[derive(Debug, Clone)]
pub struct FontStack {
    faces: Vec<FontFace>,
}

impl FontStack {
    /// Builds a stack; the first face is primary.
    pub fn new(faces: Vec<FontFace>) -> Option<Self> {
        (!faces.is_empty()).then_some(Self { faces })
    }

    /// Primary face.
    pub fn primary(&self) -> Option<&FontFace> {
        self.faces.first()
    }

    /// Face IDs in order.
    pub fn ids(&self) -> Vec<String> {
        self.faces.iter().map(|f| f.id.clone()).collect()
    }

    /// Shapes `text`, splitting it into runs by first covering face.
    pub fn shape(
        &self,
        text: &str,
        direction: TextDirection,
        language: Option<&str>,
        policy: MissingGlyphPolicy,
    ) -> Result<ShapedText, ShieldError> {
        let mut runs: Vec<(usize, String)> = Vec::new();
        let mut notdef = Vec::new();
        for c in text.chars() {
            let face = match self.faces.iter().position(|f| f.covers(c)) {
                Some(i) => i,
                None if is_ignorable(c) => runs.last().map_or(0, |r| r.0),
                None => match policy {
                    MissingGlyphPolicy::Error => {
                        return Err(ShieldError::MissingGlyph {
                            text: text.into(),
                            codepoint: u32::from(c),
                            font_stack: self.ids(),
                        });
                    }
                    MissingGlyphPolicy::Notdef => {
                        notdef.push(u32::from(c));
                        0
                    }
                },
            };
            match runs.last_mut() {
                Some((f, s)) if *f == face => s.push(c),
                _ => runs.push((face, c.to_string())),
            }
        }
        let lang = language.and_then(|l| Language::from_str(l).ok());
        let mut out = ShapedText {
            glyphs: Vec::new(),
            advance: 0.0,
            ink: None,
            faces_used: Vec::new(),
            notdef,
        };
        for (fi, run) in runs {
            let Some(face) = self.faces.get(fi) else {
                continue;
            };
            if !out.faces_used.contains(&fi) {
                out.faces_used.push(fi);
            }
            self.shape_run(&mut out, fi, face, &run, direction, lang.clone())?;
        }
        Ok(out)
    }

    fn shape_run(
        &self,
        out: &mut ShapedText,
        fi: usize,
        face: &FontFace,
        run: &str,
        direction: TextDirection,
        lang: Option<Language>,
    ) -> Result<(), ShieldError> {
        let shaper = ShaperFont::new(&face.font);
        let mut buf = Buffer::new();
        buf.push_str(run);
        buf.guess_segment_properties();
        match direction {
            TextDirection::Auto => {}
            TextDirection::Ltr => buf.set_direction(Direction::LeftToRight),
            TextDirection::Rtl => buf.set_direction(Direction::RightToLeft),
        }
        if lang.is_some() {
            buf.set_language(lang);
        }
        harfrust::shape(&shaper, &mut buf, ShapeOptions::new()).map_err(|e| {
            ShieldError::Render {
                network: String::new(),
                detail: format!("shaping {run:?} with {} failed: {e:?}", face.id),
            }
        })?;
        let metrics = face.font.glyph_metrics();
        for (info, pos) in buf.glyph_infos().iter().zip(buf.glyph_positions()) {
            let gid = harfrust::GlyphId::new(info.glyph_id);
            let x = out.advance + f64::from(pos.x_offset) / face.upem;
            let y = f64::from(pos.y_offset) / face.upem;
            if let Some(e) = metrics
                .extents(gid)
                .filter(|e| e.width > 0.0 && e.height > 0.0)
            {
                let left = x + f64::from(e.x_bearing) / face.upem;
                let top = y + f64::from(e.y_bearing) / face.upem;
                let b = InkBox {
                    min_x: left,
                    max_x: left + f64::from(e.width) / face.upem,
                    max_y: top,
                    min_y: top - f64::from(e.height) / face.upem,
                };
                out.ink = Some(out.ink.map_or(b, |i| i.union(b)));
            }
            out.glyphs.push(PlacedGlyph {
                face: fi,
                gid,
                x,
                y,
            });
            out.advance += f64::from(pos.x_advance) / face.upem;
        }
        Ok(())
    }

    /// SVG path data for `shaped` drawn at `size` px with its left origin at
    /// `x` and alphabetic baseline at `baseline`.
    pub fn outline_path(&self, shaped: &ShapedText, size: f64, x: f64, baseline: f64) -> String {
        let mut pen = SvgPen::default();
        for g in &shaped.glyphs {
            let Some(face) = self.faces.get(g.face) else {
                continue;
            };
            let Ok(font) = skrifa::FontRef::new(face.data.as_slice()) else {
                continue;
            };
            let outlines = font.outline_glyphs();
            let Some(glyph) = outlines.get(skrifa::GlyphId::new(g.gid.to_u32())) else {
                continue;
            };
            let k = size / face.upem;
            pen.map = (x + g.x * size, baseline - g.y * size, k);
            // Glyphs that fail to draw simply contribute no ink.
            glyph.draw(Size::unscaled(), &mut pen).ok();
        }
        pen.d.trim_end().to_owned()
    }
}

fn is_ignorable(c: char) -> bool {
    matches!(c, '\u{200B}'..='\u{200F}' | '\u{2060}'..='\u{206F}' | '\u{FE00}'..='\u{FE0F}' | '\u{FEFF}')
}

#[derive(Default)]
struct SvgPen {
    d: String,
    /// (origin x, baseline y, px per font unit)
    map: (f64, f64, f64),
}

impl SvgPen {
    fn pt(&mut self, x: f32, y: f32) {
        let (ox, oy, k) = self.map;
        self.d.push_str(&num(ox + f64::from(x) * k));
        self.d.push(' ');
        self.d.push_str(&num(oy - f64::from(y) * k));
        self.d.push(' ');
    }
}

impl OutlinePen for SvgPen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.d.push('M');
        self.pt(x, y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.d.push('L');
        self.pt(x, y);
    }
    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.d.push('Q');
        self.pt(cx0, cy0);
        self.pt(x, y);
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.d.push('C');
        self.pt(cx0, cy0);
        self.pt(cx1, cy1);
        self.pt(x, y);
    }
    fn close(&mut self) {
        self.d.push_str("Z ");
    }
}

#[cfg(test)]
mod tests;
