//! Icons, drawn as strokes.
//!
//! `reference/anti-patterns.md` forbids a unicode character standing in for an
//! icon on a graphical target. Each icon here is a real path at a 1.5px stroke
//! in the colour the call site passes down.

use egui::{Color32, Painter, Pos2, Rect, Stroke, Vec2};

/// The stroke width every Forge icon uses.
pub const ICON_STROKE: f32 = 1.5;
/// The box a Forge icon is drawn inside.
pub const ICON_SIZE: f32 = 16.0;

/// A magnifier: a circle and a handle.
pub fn search(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(ICON_STROKE, color);
    let unit = rect.width() / ICON_SIZE;
    let center = rect.center() - Vec2::splat(1.0 * unit);
    painter.circle_stroke(center, 4.5 * unit, stroke);
    let start = center + Vec2::splat(3.2 * unit);
    let end = center + Vec2::splat(6.0 * unit);
    painter.line_segment([start, end], stroke);
}

/// A chevron pointing down.
pub fn chevron_down(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(ICON_STROKE, color);
    let unit = rect.width() / ICON_SIZE;
    let c = rect.center();
    let left = Pos2::new(c.x - 4.0 * unit, c.y - 2.0 * unit);
    let mid = Pos2::new(c.x, c.y + 2.0 * unit);
    let right = Pos2::new(c.x + 4.0 * unit, c.y - 2.0 * unit);
    painter.line_segment([left, mid], stroke);
    painter.line_segment([mid, right], stroke);
}

/// A check mark.
pub fn check(painter: &Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(ICON_STROKE, color);
    let unit = rect.width() / ICON_SIZE;
    let c = rect.center();
    let left = Pos2::new(c.x - 4.5 * unit, c.y);
    let mid = Pos2::new(c.x - 1.5 * unit, c.y + 3.0 * unit);
    let right = Pos2::new(c.x + 4.5 * unit, c.y - 3.5 * unit);
    painter.line_segment([left, mid], stroke);
    painter.line_segment([mid, right], stroke);
}
