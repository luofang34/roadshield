//! CSS colour parsing and the blank recolouring transform.

/// Straight-alpha 8-bit sRGB colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgba(pub [u8; 4]);

impl Rgba {
    /// Opaque black.
    pub const BLACK: Rgba = Rgba([0, 0, 0, 255]);
    /// Opaque white.
    pub const WHITE: Rgba = Rgba([255, 255, 255, 255]);

    /// Parses any CSS colour string (`#rgb`, names, `hsl()`, …).
    #[must_use]
    pub fn parse(css: &str) -> Option<Rgba> {
        csscolorparser::parse(css).ok().map(|c| Rgba(c.to_rgba8()))
    }

    /// `#rrggbb` for SVG paint attributes.
    #[must_use]
    pub fn hex(self) -> String {
        let [r, g, b, _] = self.0;
        format!("#{r:02x}{g:02x}{b:02x}")
    }

    /// Alpha in `0..=1`.
    #[must_use]
    pub fn opacity(self) -> f64 {
        f64::from(self.0[3]) / 255.0
    }
}

/// Upstream `colorLighten`/`colorDarken`: per channel
/// `out = L + s·(D − L)/255`, so black maps to `lighten` and white to
/// `darken`. Alpha is untouched. The map is affine, so applying it to paint
/// colours before compositing equals applying it to composited pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Recolor {
    /// Colour black maps to.
    pub lighten: Rgba,
    /// Colour white maps to.
    pub darken: Rgba,
}

impl Recolor {
    /// Builds the transform from the definition's optional CSS colours.
    /// `None` when neither is set (truthiness as upstream).
    ///
    /// # Errors
    ///
    /// Returns a message naming the colour when either value is not a CSS colour.
    pub fn from_css(
        lighten: Option<&str>,
        darken: Option<&str>,
    ) -> Result<Option<Recolor>, String> {
        let lighten = lighten.filter(|s| !s.is_empty());
        let darken = darken.filter(|s| !s.is_empty());
        if lighten.is_none() && darken.is_none() {
            return Ok(None);
        }
        let parse = |s: Option<&str>, default: Rgba| match s {
            None => Ok(default),
            Some(s) => Rgba::parse(s).ok_or_else(|| format!("invalid colour {s:?}")),
        };
        Ok(Some(Recolor {
            lighten: parse(lighten, Rgba::BLACK)?,
            darken: parse(darken, Rgba::WHITE)?,
        }))
    }

    /// Applies the transform to one colour.
    #[must_use]
    pub fn apply(self, c: Rgba) -> Rgba {
        let ch = |i: usize| -> u8 {
            let s = f64::from(c.0.get(i).copied().unwrap_or(0));
            let l = f64::from(self.lighten.0.get(i).copied().unwrap_or(0));
            let d = f64::from(self.darken.0.get(i).copied().unwrap_or(0));
            let v = 255.0 - (s / 255.0) * (255.0 - d) - (1.0 - s / 255.0) * (255.0 - l);
            // Uint8ClampedArray stores round-half-to-even of the clamped value.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let out = v.clamp(0.0, 255.0).round_ties_even() as u8;
            out
        };
        Rgba([ch(0), ch(1), ch(2), c.0[3]])
    }
}

#[cfg(test)]
mod tests;
