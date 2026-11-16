//! The studio: state, shell, and pointer behaviour.

use std::cell::Cell as SharedCell;
use std::rc::Rc;

use gpui::{
    App, Bounds, Context, CursorStyle, FocusHandle, Focusable, KeyDownEvent, KeyUpEvent,
    MouseButton, MouseDownEvent, MouseMoveEvent, Pixels, Render, ScrollWheelEvent, SharedString,
    TitlebarOptions, Window, WindowBounds, WindowOptions, canvas, div, point, prelude::*, px, rgb,
    rgba, size,
};

use lattice_core::doc::{Drawing, History, Mark};
use lattice_core::export;
use lattice_core::geom::{BOX, Guide, Point as Unit};
use lattice_core::grid::{self, Grid};
use lattice_core::guides;
use lattice_core::tiling::{CellIndex, Placement, covering};

use crate::canvas::{self, MarkRef, Paint, View, ZOOM_STEPS};
use crate::icons::{self, Icon};
use crate::paths;
use crate::theme::{self, Mode, SWATCHES, Theme};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tool {
    Pointer,
    Hand,
    Eraser,
    Color,
}

type Cell = (i32, i32, usize);

struct Drag {
    erase: bool,
    last: Unit,
}

pub struct Studio {
    theme: Theme,
    doc: History,
    path: std::path::PathBuf,

    tool: Tool,
    space_held: bool,
    brush: String,
    palette_open: bool,
    hex_edit: Option<String>,

    view: View,
    preview: bool,
    panel_open: bool,
    tip_open: bool,
    armed_clear: bool,
    shot_mode: bool,

    hovered: Option<Cell>,
    focused: Option<Cell>,
    drag: Option<Drag>,
    panning: Option<(gpui::Point<Pixels>, View)>,

    focus_handle: FocusHandle,
    bounds: Rc<SharedCell<Bounds<Pixels>>>,
    index: Option<(String, CellIndex)>,
    guides: Vec<Guide>,
    shapes: Vec<lattice_core::tiling::Shape>,
    status: Option<SharedString>,
}

fn brush_color(brush: &str) -> Option<gpui::Rgba> {
    theme::swatch_color(brush)
}

impl Studio {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut studio = Self {
            theme: Theme::dark(),
            doc: History::new(Drawing::load(&paths::drawing_path()).unwrap_or_default()),
            path: paths::drawing_path(),
            tool: Tool::Pointer,
            space_held: false,
            brush: "auto".into(),
            palette_open: false,
            hex_edit: None,
            view: View::default(),
            preview: false,
            panel_open: true,
            tip_open: true,
            armed_clear: false,
            shot_mode: std::env::var_os("LATTICE_SHOT").is_some(),
            hovered: None,
            focused: None,
            drag: None,
            panning: None,
            focus_handle: cx.focus_handle(),
            bounds: Rc::new(SharedCell::new(Bounds {
                origin: point(px(0.), px(0.)),
                size: size(px(0.), px(0.)),
            })),
            index: None,
            guides: Vec::new(),
            shapes: Vec::new(),
            status: None,
        };
        studio.rebuild();
        if let Some(spec) = std::env::var_os("LATTICE_SHOT") {
            studio.shot_state(&spec.to_string_lossy(), cx);
        }
        if std::env::var_os("LATTICE_DEMO").is_some() && studio.cells().is_empty() {
            for cell in [(0, 0, 0), (0, 0, 1), (0, 0, 2), (0, 0, 3), (0, 0, 17), (0, 0, 18)] {
                studio.apply(cell, false, false);
            }
            studio.export("svg", cx);
            eprintln!("demo status: {:?}", studio.status);
        }
        studio.focus(window, cx);
        studio
    }

    /// Paints a fixed drawing for a screenshot run, chosen by `LATTICE_SHOT`.
    /// Tokens: `light`, `preview`, `nopanel`, `voronoi`, `empty`.
    fn shot_state(&mut self, spec: &str, cx: &mut Context<Self>) {
        if spec.contains("light") {
            self.theme = Theme::light();
        }
        if spec.contains("nopanel") {
            self.panel_open = false;
        }
        if spec.contains("voronoi") {
            let grid = "voronoi".to_string();
            self.doc.edit(|mut doc| {
                doc.grid = grid;
                doc
            });
            self.rebuild();
        }
        if !spec.contains("empty") {
            // Start from a clean grid, otherwise a second run on the same data
            // directory toggles the first cell off through pointer semantics.
            let grid = self.grid_id().to_string();
            self.doc.edit(|mut doc| {
                doc.marks.insert(grid.clone(), Vec::new());
                doc.cell_colors.insert(grid, Default::default());
                doc
            });
            for (index, cell) in [(0, 0, 0), (0, 0, 1), (0, 0, 2), (0, 0, 17), (1, 1, 5)]
                .into_iter()
                .enumerate()
            {
                self.apply(cell, index == 0, false);
            }
            self.brush = "#f16e60".to_string();
            self.tool = Tool::Color;
            self.apply((0, 0, 3), false, false);
            self.tool = Tool::Pointer;
            self.brush = "auto".to_string();
            self.hovered = Some((0, 0, 4));
            self.focused = Some((0, 0, 1));
        }
        if spec.contains("preview") {
            self.preview = true;
            self.hovered = None;
            self.focused = None;
        }
        self.persist();
        cx.notify();
    }

    /// Number of painted cells in the current grid, for tests.
    pub fn painted(&self) -> usize {
        self.cells().len()
    }

    /// Current tool name, for tests.
    pub fn tool_name(&self) -> &'static str {
        match self.tool {
            Tool::Pointer => "pointer",
            Tool::Hand => "hand",
            Tool::Eraser => "eraser",
            Tool::Color => "color",
        }
    }

    /// Current grid id, for tests.
    pub fn grid_name(&self) -> &str {
        self.grid_id()
    }

    /// Colour brush as stored, for tests.
    pub fn brush_value(&self) -> &str {
        &self.brush
    }

    /// True while preview mode is on, for tests.
    pub fn preview_on(&self) -> bool {
        self.preview
    }

    /// Zoom as a whole percentage, for tests.
    pub fn zoom_percent(&self) -> i64 {
        (self.view.zoom * 100.0).round() as i64
    }

    /// Status line text, for tests.
    pub fn status_text(&self) -> Option<String> {
        self.status.as_ref().map(|s| s.to_string())
    }

    fn focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.focus_handle.focus(window);
        cx.notify();
    }

    fn on_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        if event.keystroke.modifiers.platform || event.keystroke.modifiers.control {
            match event.keystroke.key.as_str() {
                "z" if event.keystroke.modifiers.shift => self.redo(cx),
                "z" => self.undo(cx),
                _ => {}
            }
            return;
        }
        if let Some(buffer) = self.hex_edit.clone() {
            match event.keystroke.key.as_str() {
                "escape" => {
                    self.hex_edit = None;
                    cx.notify();
                    return;
                }
                "enter" => {
                    if buffer.len() == 6 && buffer.chars().all(|c| c.is_ascii_hexdigit()) {
                        self.brush = format!("#{buffer}");
                    }
                    self.hex_edit = None;
                    cx.notify();
                    return;
                }
                "backspace" => {
                    let mut next = buffer;
                    next.pop();
                    self.hex_edit = Some(next);
                    cx.notify();
                    return;
                }
                _ => {}
            }
            if let Some(ch) = event.keystroke.key.chars().next() {
                if ch.is_ascii_hexdigit() && buffer.len() < 6 {
                    let mut next = buffer;
                    next.push(ch.to_ascii_lowercase());
                    self.hex_edit = Some(next);
                    cx.notify();
                }
            }
            return;
        }
        match event.keystroke.key.as_str() {
            "1" => self.tool = Tool::Pointer,
            "2" => self.tool = Tool::Hand,
            "3" => self.tool = Tool::Eraser,
            "4" => self.tool = Tool::Color,
            "space" => self.space_held = true,
            "escape" => self.preview = false,
            "enter" => {
                if let Some(cell) = self.focused {
                    self.apply(cell, true, false);
                }
            }
            "arrowleft" => self.nudge(-40.0, 0.0),
            "arrowright" => self.nudge(40.0, 0.0),
            "arrowup" => self.nudge(0.0, -40.0),
            "arrowdown" => self.nudge(0.0, 40.0),
            _ => return,
        }
        cx.notify();
    }

    fn nudge(&mut self, dx: f64, dy: f64) {
        let scale = self.scale();
        self.view.x += dx / scale;
        self.view.y += dy / scale;
    }

    fn grid(&self) -> Grid {
        grid::find(self.grid_id()).expect("catalogue always holds the current grid")
    }

    fn grid_id(&self) -> &str {
        &self.doc.current.grid
    }

    fn original(&self) -> bool {
        self.doc.current.original_layouts.iter().any(|id| id == self.grid_id())
    }

    fn rebuild(&mut self) {
        let original = self.original();
        let id = self.grid_id().to_string();
        match CellIndex::build(original, &id) {
            Some(index) => {
                self.shapes = index.shapes.clone();
                self.guides = self.grid().guides(original).to_vec();
                self.guides.extend(lattice_core::arrange::box_edges());
                self.index = Some((id.clone(), index));
                let total = self.shapes.len();
                let entry = self.doc.current.marks.entry(id).or_default();
                entry.retain(|mark| Self::parse_key(&mark.0).is_none_or(|(_, _, i)| i < total));
            }
            None => self.status = Some("Unknown grid".into()),
        }
    }

    fn cells(&self) -> &[Mark] {
        static EMPTY: std::sync::OnceLock<Vec<Mark>> = std::sync::OnceLock::new();
        self.doc
            .current
            .marks
            .get(self.grid_id())
            .map(Vec::as_slice)
            .unwrap_or_else(|| EMPTY.get_or_init(Vec::new))
    }

    fn scale(&self) -> f64 {
        canvas::per_unit(self.bounds.get(), self.grid().repeats())
    }

    fn persist(&self) {
        let _ = self.doc.current.save(&self.path);
    }

    fn undo(&mut self, cx: &mut Context<Self>) {
        self.doc.undo();
        self.rebuild();
        self.persist();
        cx.notify();
    }

    fn redo(&mut self, cx: &mut Context<Self>) {
        self.doc.redo();
        self.rebuild();
        self.persist();
        cx.notify();
    }

    fn hit(&self, position: gpui::Point<Pixels>) -> Option<Cell> {
        let (_, index) = self.index.as_ref()?;
        let plane = canvas::to_plane(self.bounds.get(), self.view, self.scale(), position);
        let (x, y, local) = canvas::locate(plane, self.grid().scale_y());
        index.hit(local).map(|cell| (x, y, cell))
    }

    fn key_of(cell: Cell) -> String {
        let (x, y, index) = cell;
        if x == 0 && y == 0 {
            index.to_string()
        } else {
            format!("{x},{y}:{index}")
        }
    }

    fn parse_key(key: &str) -> Option<Cell> {
        match key.split_once(',') {
            Some((x, rest)) => {
                let (y, index) = rest.split_once(':')?;
                Some((x.parse().ok()?, y.parse().ok()?, index.parse().ok()?))
            }
            None => Some((0, 0, key.parse().ok()?)),
        }
    }

    /// One painted cell. `first` decides the direction of a pointer drag.
    fn apply(&mut self, cell: Cell, first: bool, erase: bool) -> bool {
        let key = Self::key_of(cell);
        let mark = Mark::new(key.clone());
        let brush = self.brush.clone();
        let grid = self.grid_id().to_string();
        let tool = self.tool;
        let present = self.cells().iter().any(|m| m.0 == key);
        let removing = erase || tool == Tool::Eraser || (tool == Tool::Pointer && first && present);

        let mut changed = false;
        let record = first;
        let update = |mut doc: Drawing| {
            if removing {
                if let Some(list) = doc.marks.get_mut(&grid) {
                    let before = list.len();
                    list.retain(|m| m.0 != key);
                    changed = list.len() != before;
                }
                if let Some(colors) = doc.cell_colors.get_mut(&grid) {
                    colors.remove(&mark);
                }
            } else {
                let list = doc.marks.entry(grid.clone()).or_default();
                if !list.iter().any(|m| m.0 == key) {
                    list.push(mark.clone());
                    changed = true;
                }
                let inherited = doc
                    .cell_colors
                    .get(&grid)
                    .and_then(|c| c.get(&mark))
                    .cloned()
                    .unwrap_or_default();
                let value = if tool == Tool::Color { brush } else { inherited };
                let colors = doc.cell_colors.entry(grid.clone()).or_default();
                if value.is_empty() {
                    colors.remove(&mark);
                } else {
                    colors.insert(mark, value);
                }
            }
            doc
        };
        if record {
            self.doc.edit(update);
        } else {
            self.doc.amend(update);
        }
        if changed {
            self.persist();
        }
        changed
    }

    fn sample(&mut self, from: Unit, to: Unit, erase: bool) -> bool {
        let steps = ((from.x - to.x).hypot(from.y - to.y) / 2.0).ceil().max(1.0) as usize;
        let scale_y = self.grid().scale_y();
        let mut changed = false;
        for step in 1..=steps {
            let t = step as f64 / steps as f64;
            let plane = Unit::new(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t);
            let (x, y, local) = canvas::locate(plane, scale_y);
            if let Some(index) = self.index.as_ref().and_then(|(_, index)| index.hit(local)) {
                changed |= self.apply((x, y, index), false, erase);
            }
        }
        changed
    }

    fn zoom_by(&mut self, factor: f64, anchor: Option<gpui::Point<Pixels>>, cx: &mut Context<Self>) {
        let current = self.view.zoom;
        let target = (current * factor).clamp(ZOOM_STEPS[0], ZOOM_STEPS[ZOOM_STEPS.len() - 1]);
        if (target - current).abs() < f64::EPSILON {
            return;
        }
        let bounds = self.bounds.get();
        let scale = canvas::per_unit(bounds, self.grid().repeats());
        let anchor_plane = match anchor {
            Some(p) => canvas::to_plane(bounds, self.view, scale, p),
            None => Unit::new(self.view.x, self.view.y),
        };
        let ratio = self.view.zoom / target;
        self.view.x = anchor_plane.x - (anchor_plane.x - self.view.x) * ratio;
        self.view.y = anchor_plane.y - (anchor_plane.y - self.view.y) * ratio;
        self.view.zoom = target;
        cx.notify();
    }

    fn reset_view(&mut self, cx: &mut Context<Self>) {
        self.view = View::default();
        cx.notify();
    }

    fn step_zoom(&mut self, forward: bool, cx: &mut Context<Self>) {
        let at = ZOOM_STEPS.iter().position(|z| *z == self.view.zoom).unwrap_or(3);
        let next = if forward { (at + 1).min(ZOOM_STEPS.len() - 1) } else { at.saturating_sub(1) };
        let factor = ZOOM_STEPS[next] / self.view.zoom;
        self.zoom_by(factor, None, cx);
    }

    fn clear_grid(&mut self, cx: &mut Context<Self>) {
        let grid = self.grid_id().to_string();
        self.doc.edit(|mut doc| {
            doc.marks.insert(grid.clone(), Vec::new());
            doc.cell_colors.insert(grid.clone(), Default::default());
            doc.original_layouts.retain(|id| id != &grid);
            doc
        });
        self.rebuild();
        self.persist();
        cx.notify();
    }

    fn use_grid(&mut self, id: &str, cx: &mut Context<Self>) {
        if id == self.grid_id() {
            return;
        }
        let next = id.to_string();
        self.doc.edit(|mut doc| {
            doc.grid = next;
            doc
        });
        self.persist();
        self.preview = false;
        self.view = View::default();
        self.hovered = None;
        self.focused = None;
        self.rebuild();
        cx.notify();
    }

    fn picker(&self) -> Vec<Grid> {
        let marks = self.doc.current.marks.clone();
        grid::listed(Some(self.grid_id()), |id| marks.get(id).is_some_and(|m| !m.is_empty()))
    }

    fn mark_refs(&self) -> Vec<MarkRef> {
        let colors = self.doc.current.cell_colors.get(self.grid_id());
        self.cells()
            .iter()
            .filter_map(|mark| {
                let (x, y, index) = Self::parse_key(&mark.0)?;
                let color = match colors.and_then(|c| c.get(mark)).map(String::as_str) {
                    Some("auto") | None => Some(self.theme.auto_ink()),
                    Some(hex) => theme::swatch_color(hex),
                };
                Some(MarkRef { tile: (x, y), index, color })
            })
            .collect()
    }

    fn ink(&self) -> gpui::Rgba {
        self.theme.auto_ink()
    }

    fn toggle_theme(&mut self, cx: &mut Context<Self>) {
        self.theme = Theme::of(self.theme.mode.flipped());
        cx.notify();
    }

    fn tool_button(&self, tool: Tool, icon: Icon, label: &'static str, cx: &mut Context<Self>) -> gpui::Stateful<gpui::Div> {
        let active = self.tool == tool;
        let theme = self.theme;
        div()
            .id(SharedString::from(format!("tool-{label}")))
            .debug_selector(move || format!("tool-{label}"))
            .size(px(44.))
            .rounded_full()
            .flex()
            .items_center()
            .justify_center()
            .when(active, |el| el.bg(theme.accent_soft))
            .when(!active, |el| el.hover(|el| el.bg(theme.toolbar_hover)))
            .focusable()
            .focus(move |style| style.bg(theme.accent_soft))
            .child(
                canvas(
                    move |_, _, _| {},
                    move |bounds, _, window, _| {
                        icons::paint(window, icon, bounds, if active { theme.accent } else { theme.ink }, px(1.8))
                    },
                )
                .size(px(18.)),
            )
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, window, cx| {
                this.tool = tool;
                this.focus(window, cx);
                cx.notify();
            }))
    }

    fn icon_button(&self, id: &'static str, icon: Icon, disabled: bool, _cx: &mut Context<Self>) -> gpui::Stateful<gpui::Div> {
        let theme = self.theme;
        div()
            .id(id)
            .debug_selector(move || id.to_string())
            .size(px(44.))
            .rounded_full()
            .flex()
            .items_center()
            .justify_center()
            .when(!disabled, |el| el.hover(|el| el.bg(theme.toolbar_hover)))
            .focusable()
            .focus(move |style| style.bg(theme.accent_soft))
            .child(
                canvas(
                    move |_, _, _| {},
                    move |bounds, _, window, _| {
                        icons::paint(window, icon, bounds, if disabled { theme.muted } else { theme.ink }, px(1.8))
                    },
                )
                .size(px(18.)),
            )
    }

    fn panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let mut gallery = div().grid().grid_cols(3).gap(px(8.));
        for grid in self.picker() {
            let selected = grid.id == self.grid_id();
            let id = grid.id;
            let theme = theme;
            let preview = grid.clone();
            gallery = gallery.child(
                div()
                    .id(SharedString::from(format!("grid-{id}")))
                    .debug_selector(move || format!("grid-{id}"))
                    .h(px(96.))
                    .rounded(px(5.))
                    .overflow_hidden()
                    .border(px(3.))
                    .when(selected, |el| el.border_color(theme.accent))
                    .when(!selected, |el| el.border_color(gpui::transparent_black()))
                    .bg(theme.paper)
                    .focusable()
                    .focus(move |style| style.border_color(theme.accent))
                    .child(
                        canvas(
                            move |_, _, _| {},
                            move |bounds, _, window, _| {
                                paint_preview(window, bounds, &preview, theme.muted, theme.paper);
                            },
                        )
                        .size_full(),
                    )
                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                        this.use_grid(id, cx);
                    })),
            );
        }

        div()
            .id("panel")
            .w(px(348.))
            .h_full()
            .rounded(px(18.))
            .border(px(1.))
            .border_color(theme.border)
            .bg(theme.panel)
            .p(px(22.))
            .flex()
            .flex_col()
            .gap(px(12.))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(div().flex_1().min_h(px(0.)).id("gallery-scroll").overflow_scroll().child(gallery))
            .child(
                div()
                    .mt(px(12.))
                    .min_h(px(44.))
                    .rounded(px(10.))
                    .border(px(1.))
                    .border_color(theme.border)
                    .bg(theme.panel_control)
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(8.))
                    .text_size(px(13.))
                    .text_color(theme.ink)
                    .child("All 31 grids are unlocked here"),
            )
    }

    fn transport(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let pill = || div().h(px(44.)).rounded_full().border(px(1.)).border_color(theme.float_border).bg(theme.float_bg).flex().items_center().gap(px(6.)).px(px(5.));

        let active_preview = self.preview;
        let zoom_forward = self.view.zoom < ZOOM_STEPS[ZOOM_STEPS.len() - 1];
        let zoom_back = self.view.zoom > ZOOM_STEPS[0];
        let can_undo = !self.doc.past.is_empty();
        let can_redo = !self.doc.future.is_empty();
        let has_marks = !self.cells().is_empty();

        div()
            .id("transport")
            .absolute()
            .bottom(px(22.))
            .left_0()
            .right_0()
            .flex()
            .justify_center()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        pill()
                            .child(self.tool_button(Tool::Pointer, Icon::Pointer, "pointer", cx))
                            .child(self.tool_button(Tool::Hand, Icon::Hand, "hand", cx))
                            .child(self.tool_button(Tool::Eraser, Icon::Eraser, "eraser", cx))
                            .child(self.color_button(cx)),
                    )
                    .child(
                        pill()
                            .child(self.icon_button("undo", Icon::Undo, !can_undo, cx).on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.undo(cx)),
                            ))
                            .child(self.icon_button("redo", Icon::Redo, !can_redo, cx).on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.redo(cx)),
                            ))
                            .child(self.icon_button("clear", Icon::Trash, !has_marks, cx).on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    if this.armed_clear {
                                        this.clear_grid(cx);
                                        this.armed_clear = false;
                                    } else {
                                        this.armed_clear = true;
                                        this.status = Some("Click again to clear this grid".into());
                                        cx.notify();
                                    }
                                }),
                            )),
                    )
                    .child(
                        div()
                            .id("preview")
                            .debug_selector(|| "preview".into())
                            .focusable()
                            .focus(move |style| style.bg(theme.accent_soft))
                            .size(px(46.))
                            .rounded_full()
                            .border(px(1.))
                            .border_color(theme.float_border)
                            .bg(if self.preview { theme.accent_soft } else { theme.float_bg })
                            .flex()
                            .items_center()
                            .justify_center()
                            .hover(|el| el.bg(theme.toolbar_hover))
                            .child(
                                canvas(
                                    move |_, _, _| {},
                                    move |bounds, _, window, _| {
                                        icons::paint(
                                            window,
                                            Icon::Eye,
                                            bounds,
                                            if active_preview { theme.accent } else { theme.ink },
                                            px(1.8),
                                        )
                                    },
                                )
                                .size(px(18.)),
                            )
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                this.preview = !this.preview;
                                cx.notify();
                            })),
                    )
                    .child(
                        pill()
                            .child(self.icon_button("zoom-out", Icon::MagnifierMinus, !zoom_back, cx).on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.step_zoom(false, cx)),
                            ))
                            .child(
                                div()
                                    .w(px(49.))
                                    .h_full()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(11.))
                                    .text_color(theme.ink)
                                    .hover(move |el| el.bg(theme.toolbar_hover))
                                    .child(format!("{}%", (self.view.zoom * 100.0).round()))
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| this.reset_view(cx)))
                                    .id("zoom-reset")
                            .debug_selector(|| "zoom-reset".into()),
                            )
                            .child(self.icon_button("zoom-in", Icon::MagnifierPlus, !zoom_forward, cx).on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.step_zoom(true, cx)),
                            )),
                    )
                    .child(
                        div()
                            .h(px(44.))
                            .rounded_full()
                            .bg(theme.export)
                            .flex()
                            .items_center()
                            .overflow_hidden()
                            .child(
                                div()
                                    .id("export-svg")
                            .debug_selector(|| "export-svg".into())
                                    .h_full()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .px(px(32.))
                                    .focusable()
                                    .focus(move |style| style.bg(rgb(0x8f63d8)))
                                    .hover(|el| el.bg(rgb(0x8f63d8)))
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(theme.export_ink)
                                            .child("Export"),
                                    )
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| this.export("svg", cx))),
                            )
                            .child(div().w(px(1.)).h(px(18.)).bg(rgba(0xffffff40)))
                            .child(
                                div()
                                    .id("export-png")
                            .debug_selector(|| "export-png".into())
                                    .h_full()
                                    .w(px(38.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .focusable()
                                    .focus(move |style| style.bg(rgb(0x8f63d8)))
                                    .hover(|el| el.bg(rgb(0x8f63d8)))
                                    .child(
                                        canvas(
                                            move |_, _, _| {},
                                            move |bounds, _, window, _| {
                                                icons::paint(window, Icon::ChevronDown, bounds, theme.export_ink, px(1.8))
                                            },
                                        )
                                        .size(px(14.)),
                                    )
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| this.export("png", cx))),
                            ),
                    ),
            )
    }

    fn color_button(&self, cx: &mut Context<Self>) -> gpui::Stateful<gpui::Div> {
        let theme = self.theme;
        let active = self.tool == Tool::Color;
        let swatch = brush_color(&self.brush);
        div()
            .id("color-tool")
            .debug_selector(|| "color-tool".into())
            .size(px(44.))
            .rounded_full()
            .flex()
            .items_center()
            .justify_center()
            .when(active, |el| el.bg(theme.accent_soft))
            .hover(|el| el.bg(theme.toolbar_hover))
            .focusable()
            .focus(move |style| style.bg(theme.accent_soft))
            .child(div().size(px(20.)).rounded_full().border(px(1.)).border_color(theme.border).bg(
                match swatch {
                    Some(_) if active => theme.accent,
                    _ => theme.ink,
                },
            ))
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                this.tool = Tool::Color;
                this.palette_open = true;
                this.focus(window, cx);
                cx.notify();
            }))
    }

    fn palette(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let mut row = div().grid().grid_cols(5).gap(px(8.));
        for value in SWATCHES {
            let picked = self.brush == value;
            let swatch = theme::swatch_color(value);
            let value = value.to_string();
            let id_value = value.clone();
            let theme2 = theme;
            row = row.child(
                div()
                    .size(px(28.))
                    .rounded_full()
                    .border(px(1.))
                    .border_color(theme2.border)
                    .when(picked, |el| el.border_color(theme2.accent).border(px(2.)))
                    .map(|el| match swatch {
                        Some(color) => el.bg(color),
                        None => el.child(
                            div()
                                .size_full()
                                .rounded_full()
                                .bg(theme2.auto_ink()),
                        ),
                    })
                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                        this.brush = value.clone();
                        this.palette_open = false;
                        this.persist();
                        cx.notify();
                    }))
                    .id(SharedString::from(format!("swatch-{id_value}")))
                    .debug_selector(move || format!("swatch-{id_value}")),
            );
        }

        let digits = self.hex_edit.clone();
        let shown = match (&digits, theme::is_hex(&self.brush)) {
            (Some(typed), _) => format!("#{typed}"),
            (None, true) => self.brush.clone(),
            (None, false) => "#".to_string(),
        };
        let active = digits.is_some();
        let swatch = div()
            .h(px(30.))
            .w_full()
            .rounded(px(8.))
            .border(px(1.))
            .border_color(if active { theme.accent } else { theme.border })
            .bg(theme.panel_control)
            .flex()
            .items_center()
            .px(px(8.))
            .text_size(px(11.))
            .text_color(theme.ink)
            .child(shown)
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                this.hex_edit = Some(String::new());
                this.focus(window, cx);
                cx.notify();
            }))
            .id("hex-field")
            .debug_selector(|| "hex-field".into());

        div()
            .id("palette")
            .absolute()
            .bottom(px(78.))
            .left(px(16.))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .rounded(px(14.))
            .border(px(1.))
            .border_color(theme.border)
            .bg(theme.panel)
            .shadow_lg()
            .p(px(10.))
            .w(px(196.))
            .child(row)
            .child(swatch)
            .id("palette")
    }

    fn export(&mut self, kind: &'static str, cx: &mut Context<Self>) {
        let grid = self.grid_id().to_string();
        if self.cells().is_empty() {
            self.status = Some("Paint a few cells first".into());
            cx.notify();
            return;
        }
        let ink = format!("#{:02x}{:02x}{:02x}", (self.ink().r * 255.0).round() as u8, (self.ink().g * 255.0).round() as u8, (self.ink().b * 255.0).round() as u8);
        let Some(svg) = export::svg_for_with_ink(&self.doc.current, &grid, &ink) else {
            self.status = Some("Nothing to export".into());
            cx.notify();
            return;
        };
        let dir = paths::export_dir();
        let _ = std::fs::create_dir_all(&dir);
        let result: Result<std::path::PathBuf, String> = if kind == "svg" {
            let path = dir.join(format!("createx-{grid}.svg"));
            std::fs::write(&path, &svg).map(|()| path).map_err(|e| e.to_string())
        } else {
            export::png_for(&svg, 2048)
                .map_err(|e| format!("{e:?}"))
                .and_then(|bytes| {
                    let path = dir.join(format!("createx-{grid}.png"));
                    std::fs::write(&path, bytes).map(|()| path).map_err(|e| e.to_string())
                })
        };
        self.status = Some(match result {
            Ok(path) => format!("Saved {}", path.display()).into(),
            Err(error) => format!("Export failed: {error}").into(),
        });
        cx.notify();
    }

    fn topbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        div()
            .id("topbar")
            .absolute()
            .top(px(16.))
            .left(px(34.))
            .right(px(22.))
            .h(px(50.))
            .flex()
            .justify_between()
            .items_center()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(31.))
                    .text_size(px(20.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(div().text_color(theme.muted).child("LogoFX"))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_color(theme.muted)
                            .child("BrandX")
                            .child(
                                canvas(
                                    move |_, _, _| {},
                                    move |bounds, _, window, _| {
                                        icons::paint(window, Icon::Crown, bounds, theme.crown, px(1.6))
                                    },
                                )
                                .size(px(16.)),
                            ),
                    )
                    .child(div().text_color(theme.ink).child("CreateX")),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(self.round_button("share", Icon::Share, theme.ink, cx))
                    .child(self.round_button("contact", Icon::Mail, theme.ink, cx))
                    .child(self.round_button("pricing", Icon::Crown, theme.crown, cx))
                    .child(self.round_button("account", Icon::User, theme.ink, cx)),
            )
    }

    fn round_button(&self, id: &'static str, icon: Icon, color: gpui::Rgba, cx: &mut Context<Self>) -> gpui::Stateful<gpui::Div> {
        let theme = self.theme;
        div()
            .id(SharedString::from(format!("round-{id}")))
            .size(px(42.))
            .rounded_full()
            .border(px(1.))
            .border_color(theme.float_border)
            .bg(theme.float_bg)
            .flex()
            .items_center()
            .justify_center()
            .hover(move |el| el.bg(theme.toolbar_hover))
            .focusable()
            .focus(move |style| style.bg(theme.accent_soft))
            .child(
                canvas(
                    move |_, _, _| {},
                    move |bounds, _, window, _| icons::paint(window, icon, bounds, color, px(1.8)),
                )
                .size(px(18.)),
            )
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                this.status = Some(match id {
                    "share" => "Share needs the export pass".into(),
                    "contact" => "hello@logofx.app".into(),
                    "pricing" => "All grids unlocked".into(),
                    _ => "Local drawing".into(),
                });
                cx.notify();
            }))
    }

    fn stage(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let bounds = self.bounds.clone();
        let view = self.view;
        let scale_y = self.grid().scale_y();
        let guides = if self.preview { Vec::new() } else { self.guides.clone() };
        let shapes = self.shapes.clone();
        let marks = self.mark_refs();
        let hovered = if self.preview { None } else { self.hovered };
        let focused = self.focused;
        let ink = self.ink();
        let repeats = self.grid().repeats();
        let cursor = if self.panning.is_some() {
            CursorStyle::ClosedHand
        } else if self.tool == Tool::Hand || self.space_held {
            CursorStyle::OpenHand
        } else if self.tool == Tool::Color {
            CursorStyle::Crosshair
        } else {
            CursorStyle::Arrow
        };

        div()
            .flex_1()
            .h_full()
            .rounded(px(18.))
            .border(px(1.))
            .border_color(theme.border)
            .overflow_hidden()
            .bg(theme.paper)
            .child(
                div()
                    .size_full()
                    .id("canvas-surface")
                    .child(
                canvas(
                    move |b, _, _| bounds.set(b),
                    move |b, _, window, _| {
                        let scale = canvas::per_unit(b, repeats).max(0.0001) * view.zoom;
                        let paint = Paint {
                            view,
                            scale,
                            scale_y,
                            shapes: &shapes,
                            guides: &guides,
                            marks: &marks,
                            paper: theme.paper,
                            ink,
                            rule: theme.rule,
                            hover: theme.hover,
                            focus: theme.focus,
                            focus_stroke: theme.focus_stroke,
                            hovered,
                            focused,
                        };
                        window.set_window_cursor_style(cursor);
                        canvas::paint(window, b, &paint);
                    },
                )
                .size_full(),
                    )
                .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    this.focus(window, cx);
                    let pan = this.tool == Tool::Hand || this.space_held;
                    if pan {
                        this.panning = Some((event.position, this.view));
                        return;
                    }
                    if this.preview {
                        return;
                    }
                    if let Some(cell) = this.hit(event.position) {
                        let erase = this.tool == Tool::Eraser;
                        let first_erase = this.tool == Tool::Pointer
                            && this.cells().iter().any(|m| m.0 == Self::key_of(cell));
                        this.focused = Some(cell);
                        this.apply(cell, true, erase || first_erase);
                        let plane = canvas::to_plane(this.bounds.get(), this.view, this.scale(), event.position);
                        this.drag = Some(Drag { erase: erase || first_erase, last: plane });
                        cx.notify();
                    }
                }))
                .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _window, cx| {
                    if let Some((origin, start)) = this.panning {
                        let dx = f64::from(event.position.x - origin.x);
                        let dy = f64::from(event.position.y - origin.y);
                        let scale = this.scale();
                        this.view.x = start.x - dx / scale;
                        this.view.y = start.y - dy / scale;
                        cx.notify();
                        return;
                    }
                    let hovered = if this.preview || this.shot_mode {
                        None
                    } else {
                        this.hit(event.position)
                    };
                    if hovered != this.hovered {
                        this.hovered = hovered;
                        cx.notify();
                    }
                    if let Some((from, erase)) = this.drag.as_ref().map(|d| (d.last, d.erase)) {
                        let plane = canvas::to_plane(this.bounds.get(), this.view, this.scale(), event.position);
                        if let Some(drag) = this.drag.as_mut() {
                            drag.last = plane;
                        }
                        if this.sample(from, plane, erase) {
                            cx.notify();
                        }
                    }
                    if this.hovered.is_some() {
                        cx.notify();
                    }
                }))
                .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, cx| {
                    this.drag = None;
                    this.panning = None;
                    cx.notify();
                }))
                .on_mouse_up_out(MouseButton::Left, cx.listener(|this, _, _, cx| {
                    this.drag = None;
                    this.panning = None;
                    cx.notify();
                }))
                .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, cx| {
                    let delta = f64::from(event.delta.pixel_delta(px(18.)).y);
                    if delta.abs() < 0.01 {
                        return;
                    }
                    let factor = (-delta * 0.002).exp();
                    this.zoom_by(factor, Some(event.position), cx);
                })),
            )
    }

    fn tip(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        div()
            .id("tip")
            .absolute()
            .top(px(84.))
            .left_0()
            .right_0()
            .flex()
            .justify_center()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .h(px(50.))
                    .px(px(18.))
                    .rounded_full()
                    .border(px(1.))
                    .border_color(theme.float_border)
                    .bg(theme.float_bg)
                    .shadow_lg()
                    .flex()
                    .items_center()
                    .gap(px(14.))
                    .text_size(px(14.))
                    .text_color(theme.ink)
                    .child("Tap a cell to fill or clear it. Hold and drag to fill multiple cells.")
                    .child(
                        div()
                            .size(px(22.))
                            .rounded_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .hover(|el| el.bg(theme.toolbar_hover))
                            .child(
                                canvas(
                                    move |_, _, _| {},
                                    move |bounds, _, window, _| {
                                        icons::paint(window, Icon::Close, bounds, theme.muted, px(1.8))
                                    },
                                )
                                .size(px(14.)),
                            )
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                this.tip_open = false;
                                cx.notify();
                            }))
                            .id("dismiss-tip"),
                    ),
            )
    }
}

fn paint_preview(window: &mut Window, bounds: Bounds<Pixels>, grid: &Grid, muted: gpui::Rgba, paper: gpui::Rgba) {
    window.paint_quad(gpui::quad(
        bounds,
        px(0.),
        paper,
        px(0.),
        gpui::transparent_black(),
        Default::default(),
    ));

    // Grids built from two repeats render one tile at half size on the stage to
    // keep cell density equal. The thumbnail does the same, but tiles the
    // mirrored plane so the box is filled instead of showing one small tile in
    // a corner.
    let repeats = grid.repeats().max(1);
    let side = f64::from(bounds.size.width.max(bounds.size.height));
    let unit = side / (BOX * repeats as f64);
    let scale_y = grid.scale_y();
    let origin = bounds.origin;

    let guides: Vec<Guide> = grid
        .guides(false)
        .iter()
        .cloned()
        .chain(lattice_core::arrange::box_edges())
        .collect();

    let mut builder = gpui::PathBuilder::stroke(px(1.45 * repeats as f32));
    for tile_y in 0..repeats as i32 {
        for tile_x in 0..repeats as i32 {
            let placement = Placement::of(tile_x, tile_y);
            for guide in &guides {
                match guide {
                    Guide::Line { a, b } => {
                        let from = placement.out(*a);
                        let to = placement.out(*b);
                        builder.move_to(point(
                            origin.x + px((from.x * unit) as f32),
                            origin.y + px((from.y * scale_y * unit) as f32),
                        ));
                        builder.line_to(point(
                            origin.x + px((to.x * unit) as f32),
                            origin.y + px((to.y * scale_y * unit) as f32),
                        ));
                    }
                    Guide::Circle { center, r } => {
                        let steps = 40;
                        for i in 0..=steps {
                            let t = i as f64 / steps as f64 * std::f64::consts::TAU;
                            let p = placement.out(Unit::new(
                                center.x + r * t.cos(),
                                center.y + r * t.sin(),
                            ));
                            let target = point(
                                origin.x + px((p.x * unit) as f32),
                                origin.y + px((p.y * scale_y * unit) as f32),
                            );
                            if i == 0 {
                                builder.move_to(target);
                            } else {
                                builder.line_to(target);
                            }
                        }
                    }
                }
            }
        }
    }
    if let Ok(path) = builder.build() {
        window.paint_path(path, muted);
    }
}

impl Focusable for Studio {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Studio {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        div()
            .size_full()
            .font_family("Inter")
            .bg(theme.gutter)
            .p(px(8.))
            .flex()
            .gap(px(8.))
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| this.on_key(event, cx)))
            .on_key_up(cx.listener(|this, event: &KeyUpEvent, _, cx| {
                if event.keystroke.key == "space" {
                    this.space_held = false;
                    cx.notify();
                }
            }))
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .relative()
                    .child(self.stage(cx))
                    .child(self.topbar(cx))
                    .when(self.tip_open && self.cells().is_empty(), |el| el.child(self.tip(cx)))
                    .when(self.palette_open, |el| el.child(self.palette(cx)))
                    .child(self.transport(cx))
                    .child(
                        div()
                            .absolute()
                            .bottom(px(29.))
                            .left(px(22.))
                            .size(px(42.))
                            .rounded_full()
                            .border(px(1.))
                            .border_color(theme.float_border)
                            .bg(theme.float_bg)
                            .flex()
                            .items_center()
                            .justify_center()
                            .hover(move |el| el.bg(theme.toolbar_hover))
                            .child(
                                canvas(
                                    move |_, _, _| {},
                                    move |bounds, _, window, _| {
                                        icons::paint(
                                            window,
                                            if theme.mode == Mode::Dark { Icon::Sun } else { Icon::Moon },
                                            bounds,
                                            theme.ink,
                                            px(1.8),
                                        )
                                    },
                                )
                                .size(px(18.)),
                            )
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| this.toggle_theme(cx)))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .id("theme")
                            .debug_selector(|| "theme".into())
                            .focusable()
                            .focus(move |style| style.bg(theme.accent_soft)),
                    )
                    .child(
                        div()
                            .absolute()
                            .bottom(px(29.))
                            .right(px(22.))
                            .size(px(42.))
                            .rounded_full()
                            .border(px(1.))
                            .border_color(theme.float_border)
                            .bg(theme.float_bg)
                            .flex()
                            .items_center()
                            .justify_center()
                            .hover(move |el| el.bg(theme.toolbar_hover))
                            .child(
                                canvas(
                                    move |_, _, _| {},
                                    move |bounds, _, window, _| {
                                        icons::paint(window, Icon::ChevronRight, bounds, theme.ink, px(1.8))
                                    },
                                )
                                .size(px(18.)),
                            )
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                this.panel_open = !this.panel_open;
                                cx.notify();
                            }))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .id("toggle-panel")
                            .debug_selector(|| "toggle-panel".into())
                            .focusable()
                            .focus(move |style| style.bg(theme.accent_soft)),
                    ),
            )
            .when(self.panel_open, |el| el.child(self.panel(cx)))
            .when_some(self.status.clone(), |el, status| {
                el.child(
                    div()
                        .absolute()
                        .bottom(px(84.))
                        .left_0()
                        .right_0()
                        .flex()
                        .justify_center()
                        .child(
                            div()
                                .px(px(14.))
                                .h(px(32.))
                                .rounded_full()
                                .bg(theme.float_bg)
                                .border(px(1.))
                                .border_color(theme.float_border)
                                .flex()
                                .items_center()
                                .text_size(px(12.))
                                .text_color(theme.muted)
                                .child(status),
                        ),
                )
            })
    }
}

/// Opens the studio window.
/// Window size, from `LATTICE_SIZE=WxH` when set, else 1680 by 1000.
fn window_size() -> gpui::Size<Pixels> {
    let parsed = std::env::var("LATTICE_SIZE").ok().and_then(|value| {
        let (width, height) = value.split_once('x')?;
        Some(size(px(width.trim().parse::<f32>().ok()?), px(height.trim().parse::<f32>().ok()?)))
    });
    parsed.unwrap_or_else(|| size(px(1680.), px(1000.)))
}

pub fn open(cx: &mut App) {
    let _ = cx
        .text_system()
        .add_fonts(vec![std::borrow::Cow::Borrowed(
            include_bytes!("../assets/InterVariable.ttf").as_slice(),
        )]);
    let bounds = Bounds::centered(None, window_size(), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("Lattice".into()),
                ..Default::default()
            }),
            window_min_size: Some(size(px(1280.), px(800.))),
            ..Default::default()
        },
        |window, cx| cx.new(|cx| Studio::new(window, cx)),
    )
    .expect("open studio window");
    cx.activate(true);
}

#[allow(dead_code)]
fn unused(_: &KeyDownEvent) {}

#[allow(dead_code)]
const _: fn(&[Guide], Placement) = |_, _| {};
#[allow(dead_code)]
fn unused_imports() {
    let _ = guides::FREE;
    let _ = grid::is_free("square-16");
    let _ = covering;
}
