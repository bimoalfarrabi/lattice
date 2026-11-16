//! Behaviour of the studio beyond single clicks: text entry, the two-step
//! trash confirmation, drag painting, persistence across a restart, PNG
//! export contents and the zoom controls.

use gpui::{Entity, Modifiers, MouseButton, TestAppContext, VisualTestContext, point};
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

fn canvas_centre(cx: &mut VisualTestContext) -> gpui::Point<gpui::Pixels> {
    // canvas-surface has no debug selector, so aim at the viewport centre.
    let viewport = cx.update(|window, _| window.viewport_size());
    point(
        gpui::px(f32::from(viewport.width) * 0.5),
        gpui::px(f32::from(viewport.height) * 0.5),
    )
}

// Environment variables are process wide and gpui tests run on parallel
// threads, so every test holds this lock for its whole body. Without it one
// test can pick up another test's LATTICE_DATA_DIR or LATTICE_EXPORT_DIR.
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[gpui::test]
fn hex_entry_applies_a_colour(cx: &mut TestAppContext) {
    let _guard = LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = std::env::temp_dir().join(format!("lattice-hex-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let (entity, cx) = setup(cx, dir.to_str().expect("utf8 temp dir"));

    click(cx, "color-tool");
    click(cx, "hex-field");
    cx.simulate_keystrokes("e 9 6 d a 9");
    cx.run_until_parked();
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(text(cx, &entity, |s| s.brush_value()), "#e96da9");

    // Invalid input must be ignored.
    click(cx, "hex-field");
    cx.simulate_keystrokes("z z");
    cx.run_until_parked();
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(text(cx, &entity, |s| s.brush_value()), "#e96da9");

    // Escape cancels the pending entry.
    click(cx, "hex-field");
    cx.simulate_keystrokes("1 2 3 4 5 6");
    cx.run_until_parked();
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert_eq!(text(cx, &entity, |s| s.brush_value()), "#e96da9");
}

#[gpui::test]
fn trash_needs_two_clicks(cx: &mut TestAppContext) {
    let _guard = LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = std::env::temp_dir().join(format!("lattice-trash-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let (entity, cx) = setup(cx, dir.to_str().expect("utf8 temp dir"));

    let target = canvas_centre(cx);
    cx.simulate_click(target, Modifiers::default());
    cx.run_until_parked();
    assert_eq!(read(cx, &entity, |s| s.painted()), 1);

    click(cx, "clear");
    assert_eq!(read(cx, &entity, |s| s.painted()), 1);
    let status = read(cx, &entity, |s| s.status_text()).unwrap_or_default();
    assert!(
        status.to_lowercase().contains("again") || status.to_lowercase().contains("second"),
        "status should ask for a second click, got: {status}"
    );

    click(cx, "clear");
    assert_eq!(read(cx, &entity, |s| s.painted()), 0);
}

#[gpui::test]
fn drag_paints_a_run_of_cells(cx: &mut TestAppContext) {
    let _guard = LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = std::env::temp_dir().join(format!("lattice-drag-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let (entity, cx) = setup(cx, dir.to_str().expect("utf8 temp dir"));

    let start = canvas_centre(cx);
    cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();
    let mid = point(start.x + gpui::px(60.0), start.y);
    cx.simulate_mouse_move(mid, None, Modifiers::default());
    cx.run_until_parked();
    let end = point(start.x + gpui::px(120.0), start.y);
    cx.simulate_mouse_move(end, None, Modifiers::default());
    cx.run_until_parked();
    cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();
    assert!(read(cx, &entity, |s| s.painted()) >= 2);

    cx.simulate_keystrokes("ctrl-z");
    cx.run_until_parked();
    assert_eq!(read(cx, &entity, |s| s.painted()), 0);
}

#[gpui::test]
fn the_drawing_survives_a_restart(cx: &mut TestAppContext) {
    let _guard = LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = std::env::temp_dir().join(format!("lattice-persist-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let (entity, cx) = setup(cx, dir.to_str().expect("utf8 temp dir"));

    let target = canvas_centre(cx);
    cx.simulate_click(target, Modifiers::default());
    cx.run_until_parked();
    assert_eq!(read(cx, &entity, |s| s.painted()), 1);

    let file = dir.join("drawing.json");
    let body = std::fs::read_to_string(&file).expect("drawing.json written");
    assert!(body.contains("\"marks\""), "marks key in {body}");
    assert!(body.contains("\"square-16\""), "square-16 key in {body}");
    // The mark list sits between the square-16 key and its closing bracket.
    let start = body.find("\"square-16\"").expect("key found");
    let rest = &body[start..];
    let list_end = rest.find(']').expect("closing bracket");
    assert!(
        rest[..list_end].contains('"'),
        "square-16 list is non-empty in {body}"
    );

    let (second, cx) = cx.add_window_view(|window, cx| Studio::new(window, cx));
    assert_eq!(read(cx, &second, |s| s.painted()), 1);
}

#[gpui::test]
fn png_export_writes_a_png(cx: &mut TestAppContext) {
    let _guard = LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = std::env::temp_dir().join(format!("lattice-png-{}", std::process::id()));
    let pictures = dir.join("pictures");
    let data = dir.join("data");
    let _ = std::fs::create_dir_all(&pictures);
    let _ = std::fs::create_dir_all(&data);
    cx.update(|cx| {
        // Safety: single threaded test process, set before any window exists.
        unsafe {
            std::env::set_var("LATTICE_EXPORT_DIR", &pictures);
            std::env::set_var("LATTICE_DATA_DIR", &data);
        }
        cx.activate(true);
    });
    let (entity, cx) = cx.add_window_view(|window, cx| Studio::new(window, cx));

    let target = canvas_centre(cx);
    cx.simulate_click(target, Modifiers::default());
    cx.run_until_parked();
    assert_eq!(read(cx, &entity, |s| s.painted()), 1);

    click(cx, "export-png");
    let file = pictures.join("createx-square-16.png");
    assert!(file.exists(), "png written to {}", file.display());
    let bytes = std::fs::read(&file).expect("read png");
    assert!(bytes.len() >= 24, "png too short: {} bytes", bytes.len());
    assert_eq!(
        &bytes[..8],
        &[0x89, b'P', b'N', 0x47, 0x0d, 0x0a, 0x1a, 0x0a],
        "missing png signature"
    );
    let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    assert!(width > 0 && width <= 2048, "width {width}");
    assert!(height > 0 && height <= 2048, "height {height}");
}

#[gpui::test]
fn zoom_controls_step_and_reset(cx: &mut TestAppContext) {
    let _guard = LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = std::env::temp_dir().join(format!("lattice-zoom-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let (entity, cx) = setup(cx, dir.to_str().expect("utf8 temp dir"));

    assert_eq!(read(cx, &entity, |s| s.zoom_percent()), 100);
    click(cx, "zoom-in");
    assert!(read(cx, &entity, |s| s.zoom_percent()) > 100);
    click(cx, "zoom-out");
    click(cx, "zoom-out");
    assert!(read(cx, &entity, |s| s.zoom_percent()) < 100);
    click(cx, "zoom-reset");
    assert_eq!(read(cx, &entity, |s| s.zoom_percent()), 100);
}
