//! Canvas painting and pointer mapping.
//!
//! Plane units are 240 per tile, cells are indexed per tile, and every odd
//! column and row is mirrored. Painting goes paper, fills, rules, hover, focus,
//! so a rule always stays visible over a filled cell.

use gpui::{Bounds, PathBuilder, Pixels, Point, Rgba, Window, point, px};

use lattice_core::geom::{BOX, Guide, Point as Unit};
use lattice_core::tiling::{Placement, Shape, covering};

#[derive(Clone, Copy)]
pub struct View {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}

impl Default for View {
    fn default() -> Self {
        Self { x: BOX / 2.0, y: BOX / 2.0, zoom: 1.0 }
    }
}

pub const ZOOM_STEPS: [f64; 10] = [0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 4.0, 6.0, 8.0];

/// Pixels per plane unit. One tile is 640 pixels at zoom 1, so a 15 unit cell
/// renders at 40 pixels, matching the reference default framing.
pub fn per_unit(bounds: Bounds<Pixels>, repeats: usize) -> f64 {
    let side = 640.0_f64.min(f64::from(bounds.size.width)).min(f64::from(bounds.size.height));
    side / (BOX * repeats as f64)
}

pub fn to_screen(
    bounds: Bounds<Pixels>,
    view: View,
    scale: f64,
    plane: Unit,
) -> Point<Pixels> {
    let center_x = f64::from(bounds.origin.x) + f64::from(bounds.size.width) / 2.0;
    let center_y = f64::from(bounds.origin.y) + f64::from(bounds.size.height) / 2.0;
    point(
        px((center_x + (plane.x - view.x) * scale) as f32),
        px((center_y + (plane.y - view.y) * scale) as f32),
    )
}

/// Pointer position in plane units, before the tile flip is undone.
pub fn to_plane(bounds: Bounds<Pixels>, view: View, scale: f64, p: Point<Pixels>) -> Unit {
    let center_x = f64::from(bounds.origin.x) + f64::from(bounds.size.width) / 2.0;
    let center_y = f64::from(bounds.origin.y) + f64::from(bounds.size.height) / 2.0;
    Unit::new(
        view.x + (f64::from(p.x) - center_x) / scale,
        view.y + (f64::from(p.y) - center_y) / scale,
    )
}

/// Plane point to tile and local point, undoing the mirror and the squash.
pub fn locate(plane: Unit, scale_y: f64) -> (i32, i32, Unit) {
    let x = (plane.x / BOX).floor() as i32;
    let y = (plane.y / (BOX * scale_y)).floor() as i32;
    let placement = Placement::of(x, y);
    let local = placement.into(Unit::new(plane.x, plane.y / scale_y));
    (x, y, local)
}

pub struct MarkRef {
    pub tile: (i32, i32),
    pub index: usize,
    pub color: Option<Rgba>,
}

pub struct Paint<'a> {
    pub view: View,
    pub scale: f64,
    pub scale_y: f64,
    pub shapes: &'a [Shape],
    pub guides: &'a [Guide],
    pub marks: &'a [MarkRef],
    pub paper: Rgba,
    pub ink: Rgba,
    pub rule: Rgba,
    pub hover: Rgba,
    pub focus: Rgba,
    pub focus_stroke: Rgba,
    pub hovered: Option<(i32, i32, usize)>,
    pub focused: Option<(i32, i32, usize)>,
}

impl Paint<'_> {
    fn place(&self, bounds: Bounds<Pixels>, tile: (i32, i32), local: Unit) -> Point<Pixels> {
        let placement = Placement::of(tile.0, tile.1);
        let out = placement.out(local);
        to_screen(bounds, self.view, self.scale, Unit::new(out.x, out.y * self.scale_y))
    }

    fn outline(&self, bounds: Bounds<Pixels>, tile: (i32, i32), shape: &Shape) -> Option<gpui::Path<Pixels>> {
        let mut builder = PathBuilder::fill();
        let mut placed = false;
        for ring in &shape.rings {
            for (i, p) in ring.iter().enumerate() {
                let target = self.place(bounds, tile, *p);
                if i == 0 {
                    builder.move_to(target);
                } else {
                    builder.line_to(target);
                }
                placed = true;
            }
            builder.close();
        }
        if placed { builder.build().ok() } else { None }
    }
}

pub fn paint(window: &mut Window, bounds: Bounds<Pixels>, paint: &Paint<'_>) {
    window.paint_quad(gpui::quad(
        bounds,
        px(0.),
        paint.paper,
        px(0.),
        gpui::transparent_black(),
        Default::default(),
    ));

    let half_x = f64::from(bounds.size.width) / (2.0 * paint.scale);
    let half_y = f64::from(bounds.size.height) / (2.0 * paint.scale);
    let tiles = covering(
        paint.view.x - half_x,
        paint.view.y - half_y,
        half_x * 2.0,
        half_y * 2.0,
    );

    // Fills first.
    for tile in &tiles {
        for mark in paint.marks.iter().filter(|m| m.tile == *tile) {
            let Some(shape) = paint.shapes.get(mark.index) else {
                continue;
            };
            if let Some(path) = paint.outline(bounds, *tile, shape) {
                window.paint_path(path, mark.color.unwrap_or(paint.ink));
            }
        }
    }

    // Hover and focus tints sit below the rules, so a tinted cell never hides
    // the lattice. The focus ring is a stroke and finishes on top.
    if let Some((x, y, index)) = paint.hovered {
        if let Some(path) = paint
            .shapes
            .get(index)
            .and_then(|shape| paint.outline(bounds, (x, y), shape))
        {
            window.paint_path(path, paint.hover);
        }
    }
    if let Some((x, y, index)) = paint.focused {
        if let Some(path) = paint
            .shapes
            .get(index)
            .and_then(|shape| paint.outline(bounds, (x, y), shape))
        {
            window.paint_path(path, paint.focus);
        }
    }

    // Rules above the fills and the tints.
    let mut rules = PathBuilder::stroke(px(1.));
    for tile in &tiles {
        for guide in paint.guides {
            match guide {
                Guide::Line { a, b } => {
                    rules.move_to(paint.place(bounds, *tile, *a));
                    rules.line_to(paint.place(bounds, *tile, *b));
                }
                Guide::Circle { center, r } => {
                    let steps = 48;
                    for i in 0..=steps {
                        let t = i as f64 / steps as f64 * std::f64::consts::TAU;
                        let p = Unit::new(center.x + r * t.cos(), center.y + r * t.sin());
                        let target = paint.place(bounds, *tile, p);
                        if i == 0 {
                            rules.move_to(target);
                        } else {
                            rules.line_to(target);
                        }
                    }
                }
            }
        }
    }
    if let Ok(path) = rules.build() {
        window.paint_path(path, paint.rule);
    }

    // The focus ring last, so it reads as an outline.
    if let Some((x, y, index)) = paint.focused {
        if let Some(shape) = paint.shapes.get(index) {
            if let Some(first) = shape.rings.first() {
                let mut stroke = PathBuilder::stroke(px(1.));
                for (i, p) in first.iter().enumerate() {
                    let target = paint.place(bounds, (x, y), *p);
                    if i == 0 {
                        stroke.move_to(target);
                    } else {
                        stroke.line_to(target);
                    }
                }
                if let Ok(path) = stroke.build() {
                    window.paint_path(path, paint.focus_stroke);
                }
            }
        }
    }
}
