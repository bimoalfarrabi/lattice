//! The drawing document: marks, colors and undo history.

use crate::grid;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::path::Path;

/// Maximum number of past states kept by [`History`].
const HISTORY_CAP: usize = 80;

/// Reference default drawing, returned when input cannot be trusted.
pub fn default_drawing() -> Drawing {
    Drawing {
        grid: "square-16".to_string(),
        ink: Ink::Auto,
        original_layouts: Vec::new(),
        marks: BTreeMap::new(),
        cell_colors: BTreeMap::new(),
    }
}

/// One filled cell. Either `"<index>"` or `"<tileX>,<tileY>:<index>"`.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Mark(pub String);

impl Mark {
    /// Wraps a raw mark string without validation.
    pub fn new(raw: impl Into<String>) -> Self {
        Mark(raw.into())
    }

    /// Parses the mark into (tileX, tileY, index), defaulting the tile to (0, 0).
    pub fn parse(&self) -> (i32, i32, usize) {
        let raw = &self.0;
        if let Some((tile, index)) = raw.split_once(':') {
            if let Some((x, y)) = tile.split_once(',') {
                if let (Ok(x), Ok(y), Ok(index)) = (x.parse::<i32>(), y.parse::<i32>(), index.parse::<usize>()) {
                    return (x, y, index);
                }
            }
        }
        (0, 0, raw.parse::<usize>().unwrap_or(usize::MAX))
    }

    /// True when the string matches the canonical mark grammar and all
    /// integers fit in the safe-integer range.
    pub fn is_valid(&self) -> bool {
        let raw = &self.0;
        if raw.is_empty() {
            return false;
        }
        if let Some((tile, index)) = raw.split_once(':') {
            let Some((x, y)) = tile.split_once(',') else {
                return false;
            };
            return is_safe_int(x) && is_safe_int(y) && is_safe_int(index);
        }
        is_safe_int(raw)
    }
}

fn is_safe_int(s: &str) -> bool {
    let digits = s.strip_prefix('-').unwrap_or(s);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) && digits.len() <= 15
}

impl fmt::Display for Mark {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (x, y, index) = self.parse();
        if x == 0 && y == 0 {
            write!(f, "{index}")
        } else {
            write!(f, "{x},{y}:{index}")
        }
    }
}

impl fmt::Debug for Mark {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Mark({:?})", self.0)
    }
}

/// Ink color: `auto` (theme resolved) or an explicit `#rrggbb`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Ink {
    Auto,
    Hex(String),
}

impl Ink {
    /// Parses `auto` or a 6 hex digit color, else `None`.
    pub fn parse(raw: &str) -> Option<Self> {
        if raw == "auto" {
            return Some(Ink::Auto);
        }
        let hex = raw.strip_prefix('#')?;
        if hex.len() == 6 && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            Some(Ink::Hex(raw.to_string()))
        } else {
            None
        }
    }

    /// The JSON representation: `auto` or `#rrggbb`.
    pub fn as_str(&self) -> &str {
        match self {
            Ink::Auto => "auto",
            Ink::Hex(hex) => hex,
        }
    }

    /// True when a raw string is a valid explicit hex color.
    fn is_hex(raw: &str) -> bool {
        Ink::parse(raw) != Some(Ink::Auto) && raw != "auto" && Ink::parse(raw).is_some()
    }
}

/// A saved drawing with per grid marks and per mark colors.
#[derive(Clone, PartialEq, Debug)]
pub struct Drawing {
    pub grid: String,
    pub ink: Ink,
    pub original_layouts: Vec<String>,
    pub marks: BTreeMap<String, Vec<Mark>>,
    pub cell_colors: BTreeMap<String, BTreeMap<Mark, String>>,
}

impl Default for Drawing {
    fn default() -> Self {
        default_drawing()
    }
}

/// The wire shape of a drawing, matching the reference JSON keys.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonShape<'a> {
    grid: &'a str,
    ink: &'a str,
    original_layouts: &'a [String],
    marks: BTreeMap<String, Vec<String>>,
    cell_colors: BTreeMap<String, BTreeMap<String, String>>,
}

impl Drawing {
    /// Serializes with the reference JSON keys.
    pub fn to_json_pretty(&self) -> String {
        let out = self.json_shape();
        serde_json::to_string_pretty(&out).expect("drawing serializes")
    }

    /// The wire shape shared by pretty JSON, save and load.
    fn json_shape(&self) -> JsonShape<'_> {
        JsonShape {
            grid: &self.grid,
            ink: self.ink.as_str(),
            original_layouts: &self.original_layouts,
            marks: self
                .marks
                .iter()
                .map(|(id, list)| {
                    (id.clone(), list.iter().map(|m| m.0.clone()).collect::<Vec<_>>())
                })
                .collect(),
            cell_colors: self
                .cell_colors
                .iter()
                .map(|(id, per)| {
                    (
                        id.clone(),
                        per.iter().map(|(m, c)| (m.0.clone(), c.clone())).collect(),
                    )
                })
                .collect(),
        }
    }

    /// Parses a drawing, mirroring the reference `St()` rules. Invalid JSON,
    /// an unknown grid or a bad ink fall back to the default drawing.
    /// With `legacy` set, the legacy theme inks collapse to auto when they
    /// carry no marks.
    pub fn from_json(raw: &str, legacy: bool) -> Drawing {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Raw {
            #[serde(default)]
            grid: Option<String>,
            #[serde(default)]
            ink: Option<String>,
            #[serde(default)]
            original_layouts: Option<Vec<String>>,
            #[serde(default)]
            marks: Option<BTreeMap<String, Vec<String>>>,
            #[serde(default)]
            cell_colors: Option<BTreeMap<String, BTreeMap<String, String>>>,
        }

        let Ok(parsed) = serde_json::from_str::<Raw>(raw) else {
            return default_drawing();
        };

        let Some(grid_id) = parsed.grid else {
            return default_drawing();
        };
        if grid::find(&grid_id).is_none() {
            return default_drawing();
        }
        let ink = match parsed.ink.as_deref() {
            None => return default_drawing(),
            Some(raw_ink) => match Ink::parse(raw_ink) {
                Some(ink) => ink,
                None => return default_drawing(),
            },
        };

        let mut marks: BTreeMap<String, Vec<Mark>> = BTreeMap::new();
        let mut cell_colors: BTreeMap<String, BTreeMap<Mark, String>> = BTreeMap::new();
        let mut original_layouts: Vec<String> = Vec::new();

        for candidate in grid::all() {
            let Some(raw_marks) = parsed.marks.as_ref().and_then(|m| m.get(candidate.id)) else {
                continue;
            };
            if raw_marks.is_empty() {
                continue;
            }

            let has_refit = grid::is_free(candidate.id) || is_refit(candidate.id);
            let use_original = has_refit
                && parsed
                    .original_layouts
                    .as_ref()
                    .is_none_or(|list| list.iter().any(|id| id == candidate.id));

            let cell_count = grid::find(candidate.id)
                .map(|g| g.cells(use_original).len())
                .unwrap_or(0);

            let mut kept: Vec<Mark> = Vec::new();
            for raw_mark in raw_marks {
                let mark = Mark(raw_mark.clone());
                if !mark.is_valid() {
                    continue;
                }
                let (_, _, index) = mark.parse();
                if index >= cell_count {
                    continue;
                }
                if !kept.contains(&mark) {
                    kept.push(mark);
                }
            }
            if kept.is_empty() {
                continue;
            }

            if let Some(raw_colors) = parsed.cell_colors.as_ref().and_then(|c| c.get(candidate.id)) {
                let mut per: BTreeMap<Mark, String> = BTreeMap::new();
                for mark in &kept {
                    if let Some(color) = raw_colors.get(&mark.0) {
                        if color == "auto" || Ink::is_hex(color) {
                            per.insert(mark.clone(), color.clone());
                        }
                    }
                }
                if !per.is_empty() {
                    cell_colors.insert(candidate.id.to_string(), per);
                }
            }

            if use_original {
                original_layouts.push(candidate.id.to_string());
            }
            marks.insert(candidate.id.to_string(), kept);
        }

        // Legacy theme inks collapse to auto when there is nothing to color.
        let no_marks = marks.values().all(|list| list.is_empty());
        let legacy_ink = ink == Ink::Hex("#202124".into())
            || ink == Ink::Hex("#fafaf8".into())
            || (ink == Ink::Hex("#7055cf".into()) && no_marks);
        let ink = if legacy && legacy_ink { Ink::Auto } else { ink };

        Drawing {
            grid: grid_id,
            ink,
            original_layouts,
            marks,
            cell_colors,
        }
    }

    /// Writes plain JSON to `path` atomically, via a temp file plus rename.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let json = serde_json::to_string(&self.json_shape()).map_err(io::Error::other)?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, path)
    }

    /// Reads plain JSON from `path` and validates it like `from_json`.
    pub fn load(path: &Path) -> io::Result<Drawing> {
        let raw = std::fs::read_to_string(path)?;
        Ok(Drawing::from_json(&raw, false))
    }
}

/// Derived grids store their refit layout as the default one, so the
/// "original" flag flips for them. Free grids and grids without a refit
/// keep the original flag as asked.
fn is_refit(id: &str) -> bool {
    !grid::is_free(id) && grid::is_derived(id)
}

/// Undo and redo stack around a current drawing.
#[derive(Clone, Debug)]
pub struct History {
    pub past: Vec<Drawing>,
    pub current: Drawing,
    pub future: Vec<Drawing>,
}

impl History {
    /// Starts a history at `current`.
    pub fn new(current: Drawing) -> Self {
        History {
            past: Vec::new(),
            current,
            future: Vec::new(),
        }
    }

    /// Applies a mutation, pushing the previous state onto the past.
    /// Mutates the current drawing without recording a past state.
    ///
    /// A pointer drag paints many cells. The first one records history and the
    /// rest amend that same entry, so one undo reverses the whole stroke.
    pub fn amend(&mut self, f: impl FnOnce(Drawing) -> Drawing) {
        let current = std::mem::replace(&mut self.current, Drawing::default());
        self.current = f(current);
        self.future.clear();
    }

    pub fn edit(&mut self, f: impl FnOnce(Drawing) -> Drawing) {
        let next = f(self.current.clone());
        self.past.push(self.current.clone());
        if self.past.len() > HISTORY_CAP {
            self.past.remove(0);
        }
        self.current = next;
        self.future.clear();
    }

    /// Steps back one edit.
    pub fn undo(&mut self) {
        if let Some(previous) = self.past.pop() {
            self.future.insert(0, std::mem::replace(&mut self.current, previous));
        }
    }

    /// Steps forward one edit.
    pub fn redo(&mut self) {
        if !self.future.is_empty() {
            let next = self.future.remove(0);
            self.past.push(std::mem::replace(&mut self.current, next));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_json() {
        let mut drawing = default_drawing();
        drawing.ink = Ink::Hex("#112233".into());
        drawing.original_layouts.push("square-16".into());
        drawing
            .marks
            .insert("square-16".into(), vec![Mark::new("3"), Mark::new("1,0:5")]);
        drawing.cell_colors.insert(
            "square-16".into(),
            BTreeMap::from([(Mark::new("3"), "#abcdef".to_string())]),
        );

        let json = drawing.to_json_pretty();
        assert!(json.contains("\"originalLayouts\""));
        assert!(json.contains("\"cellColors\""));
        assert!(json.contains("\"ink\": \"#112233\""));

        let back = Drawing::from_json(&json, false);
        assert_eq!(back, drawing);
    }

    #[test]
    fn bad_input_falls_back_to_default() {
        let fallback = default_drawing();
        assert_eq!(Drawing::from_json("not json", false), fallback);
        assert_eq!(Drawing::from_json("{\"grid\":\"nope\",\"ink\":\"auto\"}", false), fallback);
        assert_eq!(
            Drawing::from_json("{\"grid\":\"square-16\",\"ink\":\"#12345\"}", false),
            fallback
        );
    }

    #[test]
    fn legacy_ink_collapses_to_auto() {
        let raw = "{\"grid\":\"square-16\",\"ink\":\"#202124\",\"marks\":{\"square-16\":[\"0\"]}}";
        assert_eq!(Drawing::from_json(raw, true).ink, Ink::Auto);
        assert_eq!(
            Drawing::from_json(raw, false).ink,
            Ink::Hex("#202124".into())
        );
    }

    #[test]
    fn mark_display_is_canonical() {
        assert_eq!(Mark::new("1,0:5").to_string(), "1,0:5");
        assert_eq!(Mark::new("0,0:5").to_string(), "5");
        assert_eq!(Mark::new("5").to_string(), "5");
        assert_eq!(Mark::new("5").parse(), (0, 0, 5));
        assert_eq!(Mark::new("-1,2:5").parse(), (-1, 2, 5));
        assert!(!Mark::new("1e999").is_valid());
        assert!(!Mark::new("a,b:1").is_valid());
    }

    #[test]
    fn out_of_range_indices_are_dropped() {
        let raw = "{\"grid\":\"square-16\",\"ink\":\"auto\",\"marks\":{\"square-16\":[\"0\",\"999\",\"0\"]}}";
        let drawing = Drawing::from_json(raw, false);
        assert_eq!(
            drawing.marks.get("square-16").map(|list| list.len()),
            Some(1)
        );
        assert!(drawing.cell_colors.get("square-16").is_none());
    }

    #[test]
    fn past_is_capped_at_80() {
        let mut history = History::new(default_drawing());
        for i in 0..100 {
            let mut next = history.current.clone();
            next.grid = format!("square-16-{i}");
            history.edit(|_| next.clone());
        }
        assert_eq!(history.past.len(), 80);
        history.undo();
        assert_eq!(history.future.len(), 1);
        history.redo();
        assert_eq!(history.future.len(), 0);
    }
}
