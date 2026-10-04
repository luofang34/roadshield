//! Path construction with HTML canvas semantics, serialised as SVG path data.

/// Axis-aligned rectangle in logical pixels.
#[derive(Debug, Clone, Copy, Default, PartialEq, serde::Serialize)]
pub struct Rect {
    /// Left edge.
    pub x: f64,
    /// Top edge.
    pub y: f64,
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
}

impl Rect {
    /// Rectangle from origin and size.
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Multiplies every coordinate by `s`.
    #[must_use]
    pub fn scaled(self, s: f64) -> Self {
        Self::new(self.x * s, self.y * s, self.width * s, self.height * s)
    }
}

/// Formats a number for SVG output: at most 4 decimals, no trailing zeros,
/// no negative zero. Keeps output byte-stable across platforms.
pub fn num(v: f64) -> String {
    if !v.is_finite() {
        return "0".into();
    }
    let mut s = format!("{:.4}", v);
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    if s == "-0" { "0".into() } else { s }
}

/// A path builder mirroring the `CanvasRenderingContext2D` path API.
#[derive(Debug, Clone, Default)]
pub struct Path {
    d: String,
    current: Option<(f64, f64)>,
    start: Option<(f64, f64)>,
}

const EPS: f64 = 1e-9;

impl Path {
    /// Empty path.
    pub fn new() -> Self {
        Self::default()
    }

    /// SVG path data.
    pub fn data(&self) -> &str {
        self.d.trim_end()
    }

    fn push(&mut self, cmd: char, pts: &[f64]) {
        self.d.push(cmd);
        let mut first = true;
        for p in pts {
            if !first {
                self.d.push(' ');
            }
            first = false;
            self.d.push_str(&num(*p));
        }
        self.d.push(' ');
    }

    /// `moveTo`.
    pub fn move_to(&mut self, x: f64, y: f64) {
        self.push('M', &[x, y]);
        self.current = Some((x, y));
        self.start = Some((x, y));
    }

    /// `lineTo`; starts a subpath when there is no current point.
    pub fn line_to(&mut self, x: f64, y: f64) {
        if self.current.is_none() {
            self.move_to(x, y);
            return;
        }
        self.push('L', &[x, y]);
        self.current = Some((x, y));
    }

    /// `bezierCurveTo`.
    pub fn bezier_to(&mut self, c1x: f64, c1y: f64, c2x: f64, c2y: f64, x: f64, y: f64) {
        if self.current.is_none() {
            self.move_to(c1x, c1y);
        }
        self.push('C', &[c1x, c1y, c2x, c2y, x, y]);
        self.current = Some((x, y));
    }

    /// `closePath`.
    pub fn close(&mut self) {
        if self.current.is_some() {
            self.d.push_str("Z ");
            self.current = self.start;
        }
    }

    /// `arcTo(x1, y1, x2, y2, r)` per the HTML canvas specification: a line
    /// to the first tangent point, then the arc of radius `r` tangent to
    /// both lines. Degenerate cases draw a line to `(x1, y1)`.
    pub fn arc_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, r: f64) {
        let Some((x0, y0)) = self.current else {
            self.move_to(x1, y1);
            return;
        };
        let same =
            |ax: f64, ay: f64, bx: f64, by: f64| (ax - bx).abs() < EPS && (ay - by).abs() < EPS;
        let (ux, uy) = (x0 - x1, y0 - y1);
        let (vx, vy) = (x2 - x1, y2 - y1);
        let cross = ux * vy - uy * vx;
        if same(x0, y0, x1, y1) || same(x1, y1, x2, y2) || r.abs() < EPS || cross.abs() < EPS {
            self.line_to(x1, y1);
            return;
        }
        let lu = ux.hypot(uy);
        let lv = vx.hypot(vy);
        let cos = ((ux * vx + uy * vy) / (lu * lv)).clamp(-1.0, 1.0);
        let theta = cos.acos();
        let d = r / (theta / 2.0).tan();
        let (t1x, t1y) = (x1 + ux / lu * d, y1 + uy / lu * d);
        let (t2x, t2y) = (x1 + vx / lv * d, y1 + vy / lv * d);
        self.line_to(t1x, t1y);
        // Turning from (P1 - P0) to (P2 - P1); positive cross in y-down
        // space is a clockwise turn, which is SVG sweep-flag 1.
        let turn = (x1 - x0) * (y2 - y1) - (y1 - y0) * (x2 - x1);
        let sweep = if turn > 0.0 { 1.0 } else { 0.0 };
        self.push('A', &[r, r, 0.0, 0.0, sweep, t2x, t2y]);
        self.current = Some((t2x, t2y));
    }

    /// Full ellipse (`ctx.ellipse(cx, cy, rx, ry, 0, 0, 2π)`) as two arcs.
    pub fn ellipse(&mut self, cx: f64, cy: f64, rx: f64, ry: f64) {
        self.move_to(cx + rx, cy);
        self.push('A', &[rx, ry, 0.0, 1.0, 1.0, cx - rx, cy]);
        self.push('A', &[rx, ry, 0.0, 1.0, 1.0, cx + rx, cy]);
        self.close();
    }

    /// Axis-aligned rectangle (`ctx.rect`).
    pub fn rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.move_to(x, y);
        self.line_to(x + w, y);
        self.line_to(x + w, y + h);
        self.line_to(x, y + h);
        self.close();
    }
}

#[cfg(test)]
mod tests;
