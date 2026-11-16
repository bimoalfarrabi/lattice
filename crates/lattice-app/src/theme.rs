//! Colour tokens for both themes.
//!
//! Values are lifted from the reference stylesheet so the studio reads the
//! same. The only computed token is the grid rule, mixed at 22 percent of ink
//! into paper, exactly like the cascade does.

use gpui::{Rgba, rgb, rgba};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Dark,
    Light,
}

impl Mode {
    pub fn flipped(self) -> Self {
        match self {
            Mode::Dark => Mode::Light,
            Mode::Light => Mode::Dark,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Theme {
    pub mode: Mode,
    pub gutter: Rgba,
    pub paper: Rgba,
    pub panel: Rgba,
    pub panel_control: Rgba,
    pub panel_control_hover: Rgba,
    pub border: Rgba,
    pub toolbar: Rgba,
    pub toolbar_hover: Rgba,
    pub action: Rgba,
    pub action_hover: Rgba,
    pub accent: Rgba,
    pub accent_strong: Rgba,
    pub accent_soft: Rgba,
    pub ink: Rgba,
    pub muted: Rgba,
    pub section_ink: Rgba,
    pub float_bg: Rgba,
    pub float_border: Rgba,
    pub rule: Rgba,
    pub hover: Rgba,
    pub focus: Rgba,
    pub focus_stroke: Rgba,
    pub export: Rgba,
    pub export_ink: Rgba,
    pub crown: Rgba,
    pub success: Rgba,
    pub danger: Rgba,
}

/// 22 percent of `ink` mixed into `paper`, in sRGB space.
///
/// Eight digit values must go through `rgba`, not `rgb`: `rgb` keeps only the
/// low 24 bits, so `rgb(0xffffff17)` renders as opaque yellow.
fn mix_rule(ink: Rgba, paper: Rgba) -> Rgba {
    let blend = |a: f32, b: f32| a * 0.22 + b * 0.78;
    Rgba {
        r: blend(ink.r, paper.r),
        g: blend(ink.g, paper.g),
        b: blend(ink.b, paper.b),
        a: 1.0,
    }
}

impl Theme {
    pub fn dark() -> Self {
        let paper = rgb(0x171819);
        let ink = rgb(0xf0f0f1);
        Self {
            mode: Mode::Dark,
            gutter: rgb(0x101112),
            paper,
            panel: rgb(0x1b1c1e),
            panel_control: rgb(0x292a2d),
            panel_control_hover: rgb(0x35363a),
            border: rgba(0xffffff17),
            toolbar: rgb(0x27282b),
            toolbar_hover: rgb(0x36373b),
            action: rgb(0x303135),
            action_hover: rgb(0x3b3c40),
            accent: rgb(0xb99aff),
            accent_strong: rgb(0xa783ed),
            accent_soft: rgb(0x332c40),
            ink,
            muted: rgb(0x96989d),
            section_ink: rgb(0xb6b7bc),
            float_bg: rgba(0xffffff0f),
            float_border: rgba(0xffffff1a),
            rule: mix_rule(ink, paper),
            hover: Rgba { a: 0.27, ..rgb(0x9265dc) },
            focus: Rgba { a: 0.31, ..rgb(0x9265dc) },
            focus_stroke: rgb(0x764cb6),
            export: rgb(0x7950c8),
            export_ink: rgb(0xf5f3ec),
            crown: rgb(0xf2c66d),
            success: rgb(0x4ade80),
            danger: rgb(0xf16e60),
        }
    }

    pub fn light() -> Self {
        let paper = rgb(0xfafafa);
        let ink = rgb(0x202027);
        Self {
            mode: Mode::Light,
            gutter: rgb(0xe9e9eb),
            paper,
            panel: rgb(0xf7f7f8),
            panel_control: rgb(0xe9e9ed),
            panel_control_hover: rgb(0xdedee4),
            border: rgba(0x1a1b1f1f),
            toolbar: rgb(0xe8e8ec),
            toolbar_hover: rgb(0xdadade),
            action: rgb(0xf6f6f9),
            action_hover: rgb(0xffffff),
            accent: rgb(0x8960d7),
            accent_strong: rgb(0x7950c8),
            accent_soft: rgb(0xeee7fb),
            ink,
            muted: rgb(0x686a74),
            section_ink: rgb(0x555761),
            float_bg: rgb(0xfafafd),
            float_border: rgba(0x2a2a371f),
            rule: mix_rule(ink, paper),
            hover: Rgba { a: 0.27, ..rgb(0x9265dc) },
            focus: Rgba { a: 0.31, ..rgb(0x9265dc) },
            focus_stroke: rgb(0x764cb6),
            export: rgb(0x7950c8),
            export_ink: rgb(0xf5f3ec),
            crown: rgb(0xae7417),
            success: rgb(0x15803d),
            danger: rgb(0xc0392b),
        }
    }

    pub fn of(mode: Mode) -> Self {
        match mode {
            Mode::Dark => Self::dark(),
            Mode::Light => Self::light(),
        }
    }

    /// Ink for a cell: `auto` means the full contrast partner of the paper.
    pub fn auto_ink(self) -> Rgba {
        match self.mode {
            Mode::Dark => rgb(0xffffff),
            Mode::Light => rgb(0x000000),
        }
    }
}

/// The nine swatches, automatic first.
pub const SWATCHES: [&str; 9] = [
    "auto", "#808080", "#9470ed", "#4a91ee", "#37be91", "#f3cc4b", "#fa9d4d", "#f16e60", "#e96da9",
];

pub fn swatch_color(value: &str) -> Option<Rgba> {
    let hex = value.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    u32::from_str_radix(hex, 16).ok().map(rgb)
}

pub fn is_hex(value: &str) -> bool {
    value.len() == 7 && value.starts_with('#') && value[1..].chars().all(|c| c.is_ascii_hexdigit())
}
