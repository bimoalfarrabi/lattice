//! Every toolbar and panel control, clicked by its element id.

use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext, point};
use lattice_app::studio::Studio;

fn setup<'a>(cx: &'a mut TestAppContext, home: &str) -> (Entity<Studio>, &'a mut VisualTestContext) {
    cx.update(|cx| {
        // Safety: single threaded test process, set before any window exists.
        unsafe { std::env::set_var("LATTICE_DATA_DIR", home) };
        cx.activate(true);
    });
    cx.add_window_view(|window, cx| Studio::new(window, cx))
}

fn centre(cx: &mut VisualTestContext, id: &'static str) -> gpui::Point<gpui::Pixels> {
    let bounds = cx.debug_bounds(id).unwrap_or_else(|| panic!("no bounds for {id}"));
    point(
        bounds.origin.x + bounds.size.width / 2.0,
        bounds.origin.y + bounds.size.height / 2.0,
    )
}

fn read<T>(cx: &mut VisualTestContext, entity: &Entity<Studio>, f: impl Fn(&Studio) -> T) -> T {
    cx.update(|_, app| f(entity.read(app)))
}

fn text(cx: &mut VisualTestContext, entity: &Entity<Studio>, f: impl Fn(&Studio) -> &str) -> String {
    cx.update(|_, app| f(entity.read(app)).to_string())
}

fn click(cx: &mut VisualTestContext, id: &'static str) {
    let target = centre(cx, id);
    cx.simulate_click(target, Modifiers::default());
    cx.run_until_parked();
}

#[gpui::test]
fn controls_change_their_state(cx: &mut TestAppContext) {
    let dir = std::env::temp_dir().join(format!("lattice-controls-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let (entity, cx) = setup(cx, dir.to_str().expect("utf8 temp dir"));

    click(cx, "tool-eraser");
    assert_eq!(read(cx, &entity, |s| s.tool_name()), "eraser");

    click(cx, "tool-hand");
    assert_eq!(read(cx, &entity, |s| s.tool_name()), "hand");

    click(cx, "color-tool");
    assert_eq!(read(cx, &entity, |s| s.tool_name()), "color");
    click(cx, "swatch-#e96da9");
    assert_eq!(text(cx, &entity, |s| s.brush_value()), "#e96da9");

    click(cx, "grid-voronoi");
    assert_eq!(text(cx, &entity, |s| s.grid_name()), "voronoi");
    click(cx, "grid-square-16");
    assert_eq!(text(cx, &entity, |s| s.grid_name()), "square-16");

    click(cx, "zoom-in");
    click(cx, "zoom-reset");

    click(cx, "preview");
    click(cx, "preview");

    click(cx, "toggle-panel");
    click(cx, "theme");
    click(cx, "theme");
}

#[gpui::test]
fn painting_then_exporting_writes_a_file(cx: &mut TestAppContext) {
    let dir = std::env::temp_dir().join(format!("lattice-export-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let pictures = dir.join("pictures");
    let _ = std::fs::create_dir_all(&pictures);
    cx.update(|cx| {
        // Safety: single threaded test process, set before any window exists.
        unsafe { std::env::set_var("LATTICE_EXPORT_DIR", &pictures) };
        cx.activate(true);
    });
    let (entity, cx) = cx.add_window_view(|window, cx| Studio::new(window, cx));

    let viewport = cx.update(|window, _| window.viewport_size());
    let target = point(
        gpui::px(f32::from(viewport.width) * 0.5),
        gpui::px(f32::from(viewport.height) * 0.5),
    );
    cx.simulate_click(target, Modifiers::default());
    cx.run_until_parked();
    assert_eq!(read(cx, &entity, |s| s.painted()), 1);

    click(cx, "export-svg");
    let file = pictures.join("createx-square-16.svg");
    assert!(file.exists(), "export written to {}", file.display());
    let body = std::fs::read_to_string(&file).expect("read export");
    assert!(body.starts_with("<svg xmlns="));
    assert!(body.ends_with("</g></svg>"));
}
