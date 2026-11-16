//! Icon glyphs drawn as paths.
//!
//! The reference uses a thin rounded stroke set at 18 pixels. Glyphs are
//! described in a 24 unit box and scaled to the requested size, so weight and
//! proportions stay under our control without an asset pipeline.

use gpui::{Bounds, PathBuilder, Pixels, Window, point, px};

#[derive(Clone, Copy)]
pub enum Icon {
    Pointer,
    Hand,
    Eraser,
    Undo,
    Redo,
    Trash,
    Eye,
    Minus,
    Plus,
    MagnifierMinus,
    MagnifierPlus,
    ChevronDown,
    ChevronRight,
    Share,
    Mail,
    Crown,
    User,
    Lock,
    Sun,
    Moon,
    PanelRight,
    Close,
}

fn arc(cx: f32, cy: f32, r: f32, from: f32, to: f32, steps: usize) -> Vec<(f32, f32)> {
    (0..=steps)
        .map(|i| {
            let t = from + (to - from) * i as f32 / steps as f32;
            (cx + r * t.cos(), cy + r * t.sin())
        })
        .collect()
}

fn circle(cx: f32, cy: f32, r: f32) -> Vec<(f32, f32)> {
    arc(cx, cy, r, 0.0, std::f32::consts::TAU, 28)
}

fn strokes(icon: Icon) -> Vec<Vec<(f32, f32)>> {
    use std::f32::consts::PI;
    match icon {
        Icon::Pointer => vec![vec![(4.0, 4.7), (20.0, 11.2), (13.4, 13.1), (11.4, 19.7), (4.0, 4.7)]],
        Icon::Hand => vec![
            vec![(18.0, 11.0), (18.0, 6.0)],
            vec![(14.0, 10.0), (14.0, 4.0)],
            vec![(10.0, 10.5), (10.0, 6.0)],
            vec![(6.0, 12.0), (6.0, 14.0)],
            arc(12.0, 15.0, 6.0, PI * 0.95, PI * 0.05, 12),
        ],
        Icon::Eraser => vec![
            vec![(4.7, 14.3), (14.0, 5.0), (18.5, 9.5), (11.0, 21.0), (9.0, 21.0), (4.0, 16.0), (4.7, 14.3)],
            vec![(8.0, 11.0), (13.0, 16.0)],
        ],
        Icon::Undo => vec![
            vec![(9.0, 14.0), (4.0, 9.0), (9.0, 4.0)],
            vec![(4.0, 9.0), (14.5, 9.0)],
            arc(14.5, 14.5, 5.5, -PI / 2.0, PI / 2.0, 12),
        ],
        Icon::Redo => vec![
            vec![(15.0, 14.0), (20.0, 9.0), (15.0, 4.0)],
            vec![(20.0, 9.0), (9.5, 9.0)],
            arc(9.5, 14.5, 5.5, -PI / 2.0, PI / 2.0, 12),
        ],
        Icon::Trash => vec![
            vec![(4.0, 7.0), (20.0, 7.0)],
            vec![(6.0, 7.0), (7.0, 21.0), (17.0, 21.0), (18.0, 7.0)],
            vec![(10.0, 11.0), (10.0, 17.0)],
            vec![(14.0, 11.0), (14.0, 17.0)],
        ],
        Icon::Eye => vec![
            arc(12.0, 12.0, 9.0, PI * 1.15, PI * 1.85, 12),
            arc(12.0, 12.0, 9.0, PI * 0.15, PI * 0.85, 12),
            circle(12.0, 12.0, 3.0),
        ],
        Icon::Minus => vec![vec![(5.0, 12.0), (19.0, 12.0)]],
        Icon::MagnifierMinus => vec![
            circle(10.5, 10.5, 6.5),
            vec![(15.4, 15.4), (20.0, 20.0)],
            vec![(7.5, 10.5), (13.5, 10.5)],
        ],
        Icon::MagnifierPlus => vec![
            circle(10.5, 10.5, 6.5),
            vec![(15.4, 15.4), (20.0, 20.0)],
            vec![(7.5, 10.5), (13.5, 10.5)],
            vec![(10.5, 7.5), (10.5, 13.5)],
        ],
        Icon::Plus => vec![vec![(12.0, 5.0), (12.0, 19.0)], vec![(5.0, 12.0), (19.0, 12.0)]],
        Icon::ChevronDown => vec![vec![(6.0, 9.0), (12.0, 15.0), (18.0, 9.0)]],
        Icon::ChevronRight => vec![vec![(9.0, 5.0), (15.0, 12.0), (9.0, 19.0)]],
        Icon::Share => vec![
            circle(18.0, 5.0, 2.6),
            circle(6.0, 12.0, 2.6),
            circle(18.0, 19.0, 2.6),
            vec![(8.3, 10.8), (15.7, 6.2)],
            vec![(8.3, 13.2), (15.7, 17.8)],
        ],
        Icon::Mail => vec![
            vec![(3.0, 5.5), (21.0, 5.5), (21.0, 18.5), (3.0, 18.5), (3.0, 5.5)],
            vec![(3.0, 7.0), (12.0, 13.0), (21.0, 7.0)],
        ],
        Icon::Crown => vec![
            vec![(3.0, 17.0), (5.0, 7.0), (9.0, 12.0), (12.0, 6.0), (15.0, 12.0), (19.0, 7.0), (21.0, 17.0), (3.0, 17.0)],
        ],
        Icon::User => vec![circle(12.0, 8.0, 4.0), arc(12.0, 21.0, 7.0, PI * 1.15, PI * 1.85, 12)],
        Icon::Lock => vec![
            vec![(5.0, 11.0), (19.0, 11.0), (19.0, 21.0), (5.0, 21.0), (5.0, 11.0)],
            vec![(8.0, 11.0), (8.0, 7.0)],
            arc(12.0, 7.0, 4.0, PI, PI * 2.0, 10),
        ],
        Icon::Sun => {
            let mut out = vec![circle(12.0, 12.0, 4.0)];
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                out.push(vec![
                    (12.0 + 6.6 * a.cos(), 12.0 + 6.6 * a.sin()),
                    (12.0 + 9.0 * a.cos(), 12.0 + 9.0 * a.sin()),
                ]);
            }
            out
        }
        Icon::Moon => vec![
            arc(12.5, 12.0, 8.5, PI * 0.42, PI * 1.62, 16),
            arc(15.5, 12.0, 8.0, PI * 1.62, PI * 0.42, 16),
        ],
        Icon::PanelRight => vec![
            vec![(3.0, 4.0), (21.0, 4.0), (21.0, 20.0), (3.0, 20.0), (3.0, 4.0)],
            vec![(15.0, 4.0), (15.0, 20.0)],
        ],
        Icon::Close => vec![vec![(6.0, 6.0), (18.0, 18.0)], vec![(18.0, 6.0), (6.0, 18.0)]],
    }
}

/// Draws `icon` centred in `bounds`, stroked at `weight`.
pub fn paint(
    window: &mut Window,
    icon: Icon,
    bounds: Bounds<Pixels>,
    color: gpui::Rgba,
    weight: Pixels,
) {
    let side = bounds.size.width.min(bounds.size.height);
    let scale = f32::from(side) / 24.0;
    let origin = bounds.origin;

    let mut builder = PathBuilder::stroke(weight);
    let mut placed = false;
    for line in strokes(icon) {
        for (index, (x, y)) in line.into_iter().enumerate() {
            let target = point(origin.x + px(x * scale), origin.y + px(y * scale));
            if index == 0 {
                builder.move_to(target);
            } else {
                builder.line_to(target);
            }
            placed = true;
        }
    }
    if placed {
        if let Ok(path) = builder.build() {
            window.paint_path(path, color);
        }
    }
}
