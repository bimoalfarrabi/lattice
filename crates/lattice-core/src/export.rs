//! SVG and PNG export, byte compatible with the reference endpoint.

use crate::doc::{Drawing, Ink};
use crate::grid;
use std::collections::HashMap;
use std::fmt::Write as _;

/// Cell space side length in SVG units.
const CELL: i64 = 240;
/// Longest PNG side the rasterizer allows.
const PNG_MAX_SIDE: u32 = 2048;

/// Render errors.
#[derive(Debug)]
pub enum ExportError {
    /// The SVG could not be parsed or rasterized.
    Raster(String),
    /// The PNG encoder failed.
    Encode(String),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportError::Raster(message) => write!(f, "raster failed: {message}"),
            ExportError::Encode(message) => write!(f, "encode failed: {message}"),
        }
    }
}

impl std::error::Error for ExportError {}

/// The ink to paint with, after resolving `auto`.
fn resolve_ink(ink: &Ink, fallback: &str) -> String {
    match ink {
        Ink::Auto => fallback.to_string(),
        Ink::Hex(hex) => hex.clone(),
    }
}

/// One grouped path: a tile, its resolved color and concatenated cell paths.
struct Group {
    x: i32,
    y: i32,
    color: Option<String>,
    d: String,
}

/// Groups marks by (tileX, tileY, resolved color) in first-seen order.
fn groups(drawing: &Drawing, grid_id: &str, ink: &str) -> Option<Vec<Group>> {    let candidate = grid::find(grid_id)?;
    let original = drawing.original_layouts.iter().any(|id| id == grid_id);
    let marks = drawing.marks.get(grid_id)?;
    if marks.is_empty() {
        return None;
    }

    let cells = candidate.cells(original);
    let colors = drawing.cell_colors.get(grid_id);

    let mut order: Vec<(i32, i32, Option<String>)> = Vec::new();
    let mut map: HashMap<(i32, i32, Option<String>), Vec<String>> = HashMap::new();

    for mark in marks {
        let (x, y, index) = mark.parse();
        if index >= cells.len() {
            continue;
        }
        let raw_color = colors.and_then(|per| per.get(mark));
        // None means the mark inherits the group fill; Some is explicit.
        let color = match raw_color {
            Some(c) if c == "auto" => Some(ink.to_string()),
            Some(c) => Some(c.clone()),
            None => None,
        };
        let key = (x, y, color);
        if !map.contains_key(&key) {
            order.push(key.clone());
            map.insert(key.clone(), Vec::new());
        }
        map.get_mut(&key).expect("just inserted").push(cells[index].clone());
    }

    if order.is_empty() {
        return None;
    }

    Some(
        order
            .into_iter()
            .map(|(x, y, color)| {
                let d = map.remove(&(x, y, color.clone())).expect("tracked").join("");
                Group { x, y, color, d }
            })
            .collect(),
    )
}

/// The reference transform string for a tile.
fn transform(x: i32, y: i32) -> String {
    let odd_x = x.rem_euclid(2) == 1;
    let odd_y = y.rem_euclid(2) == 1;
    let tx = (x + i32::from(odd_x)) * CELL as i32;
    let ty = (y + i32::from(odd_y)) * CELL as i32;
    let sx = if odd_x { -1 } else { 1 };
    let sy = if odd_y { -1 } else { 1 };
    format!("translate({tx} {ty}) scale({sx} {sy})")
}

/// Renders the drawing for one grid as SVG, or `None` for an unknown grid
/// or a drawing with no marks for it.
pub fn svg_for(drawing: &Drawing, grid_id: &str) -> Option<String> {
    svg_for_with_ink(drawing, grid_id, "#000000")
}

/// Renders with an explicit fallback for `auto` ink, which the caller resolves
/// from its theme.
pub fn svg_for_with_ink(drawing: &Drawing, grid_id: &str, fallback: &str) -> Option<String> {
    let ink = resolve_ink(&drawing.ink, fallback);
    let groups = groups(drawing, grid_id, &ink)?;

    let min_x = groups.iter().map(|g| g.x).min().unwrap_or(0);
    let max_x = groups.iter().map(|g| g.x).max().unwrap_or(0);
    let min_y = groups.iter().map(|g| g.y).min().unwrap_or(0);
    let max_y = groups.iter().map(|g| g.y).max().unwrap_or(0);
    let vx = min_x as i64 * CELL;
    let vy = min_y as i64 * CELL;
    let vw = (max_x - min_x + 1) as i64 * CELL;
    let vh = (max_y - min_y + 1) as i64 * CELL;

    let mut out = String::new();
    let _ = write!(out, "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{vx} {vy} {vw} {vh}\">");
    let _ = write!(out, "<g fill=\"{ink}\">");
    for group in &groups {
        let _ = write!(out, "<path");
        // Explicit colors are emitted per path; the group fill covers the rest.
        if let Some(color) = &group.color {
            let _ = write!(out, " fill=\"{color}\"");
        }
        let _ = write!(
            out,
            " transform=\"{}\" d=\"{}\"/>",
            transform(group.x, group.y),
            group.d
        );
    }
    out.push_str("</g></svg>");
    Some(out)
}

/// Rasterizes `svg` to PNG, longest side capped at 2048 while keeping the
/// aspect ratio.
pub fn png_for(svg: &str, max_side: u32) -> Result<Vec<u8>, ExportError> {
    let max_side = max_side.min(PNG_MAX_SIDE);
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_str(svg, &options)
        .map_err(|error| ExportError::Raster(error.to_string()))?;

    let size = tree.size();
    let scale = (max_side as f32 / size.width().max(size.height())).min(1.0);
    let width = ((size.width() * scale).round() as u32).max(1);
    let height = ((size.height() * scale).round() as u32).max(1);

    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| ExportError::Raster("pixmap allocation failed".into()))?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

    pixmap.encode_png().map_err(|error| ExportError::Encode(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::{default_drawing, History, Mark};

    fn golden() -> String {
        std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/export-test.svg"
        ))
        .expect("fixtures/export-test.svg")
    }

    fn golden_drawing() -> Drawing {
        let mut drawing = default_drawing();
        drawing.ink = Ink::Hex("#ffffff".into());
        drawing.marks.insert(
            "square-16".into(),
            [0usize, 1, 2, 3, 17, 18]
                .iter()
                .map(|i| Mark::new(i.to_string()))
                .collect(),
        );
        drawing
    }

    #[test]
    fn reproduces_the_golden_file() {
        let drawing = golden_drawing();
        let svg = svg_for(&drawing, "square-16").expect("marks present");
        assert_eq!(svg, golden());
    }

    #[test]
    fn handles_negative_tiles() {
        let mut drawing = default_drawing();
        drawing.ink = Ink::Hex("#ffffff".into());
        drawing.marks.insert(
            "square-16".into(),
            vec![Mark::new("-1,-1:0"), Mark::new("0,0:0")],
        );
        let svg = svg_for(&drawing, "square-16").expect("marks present");
        assert!(svg.contains("viewBox=\"-240 -240 480 480\""));
        // Tile (-1, -1) is odd on both axes: translate(0 0) with mirrored scale.
        assert!(svg.contains("transform=\"translate(0 0) scale(-1 -1)\""));
        assert!(svg.contains("transform=\"translate(0 0) scale(1 1)\""));
    }

    #[test]
    fn resolves_auto_ink() {
        let mut drawing = default_drawing();
        drawing.ink = Ink::Auto;
        drawing
            .marks
            .insert("square-16".into(), vec![Mark::new("0")]);
        let svg = svg_for(&drawing, "square-16").expect("marks present");
        assert!(svg.contains("<g fill=\"#000000\">"));
    }

    #[test]
    fn rejects_unknown_grid_and_empty_marks() {
        assert!(svg_for(&default_drawing(), "nope").is_none());
        assert!(svg_for(&default_drawing(), "square-16").is_none());
    }

    #[test]
    fn rasterizes_to_png() {
        let svg = svg_for(&golden_drawing(), "square-16").expect("marks present");
        let png = png_for(&svg, 512).expect("png");
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn history_undo_redo_round_trip() {
        let mut history = History::new(golden_drawing());
        let before = history.current.clone();
        history.edit(|mut d| {
            d.ink = Ink::Hex("#000000".into());
            d
        });
        assert_ne!(history.current, before);
        history.undo();
        assert_eq!(history.current, before);
        history.redo();
        assert_eq!(history.current.ink, Ink::Hex("#000000".into()));
    }
}
