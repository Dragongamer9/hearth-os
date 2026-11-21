//! The window every Hearth app opens, and what it shares at start.
//!
//! Each app draws its own title bar, as its approved mockup shows (ui::title_bar), so the window asks for no frame
//! from the desktop; it can still be moved, maximized, minimized and resized from its title bar and edges.

use iced::window;
use iced::{Font, Size};

pub const INTER: Font = Font::with_name("Inter");

/// The window: its app id (the .desktop file's name, so the taskbar shows the right icon), size and smallest size.
/// Also picks how the app draws (renderer), before iced starts: every app's builder calls this before `.run()`.
pub fn window(app_id: &str, size: (f32, f32), min: (f32, f32)) -> window::Settings {
    renderer(app_id);
    window::Settings {
        size: Size::new(size.0, size.1),
        min_size: Some(Size::new(min.0, min.1)),
        decorations: false,
        platform_specific: window::settings::PlatformSpecific { application_id: app_id.to_string(), override_redirect: false },
        ..window::Settings::default()
    }
}

/// `--page NAME` (and `--show NAME`) on the command line: which page or dialog opens first. Used by the menu's
/// actions and by CI's pictures.
pub fn arg(name: &str) -> Option<String> {
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        if a == name {
            return it.next();
        }
        if let Some(v) = a.strip_prefix(&format!("{name}=")) {
            return Some(v.to_string());
        }
    }
    None
}

pub fn flag(name: &str) -> bool {
    std::env::args().skip(1).any(|a| a == name)
}

/// `--version`, `--help`: answered without opening a window.
pub fn handle_cli(name: &str, help: &str) -> bool {
    if flag("--help") || flag("-h") {
        println!("{name}\n{help}");
        return true;
    }
    if flag("--version") {
        println!("{name} (Hearth, pure Rust)");
        return true;
    }
    false
}

/// What the window's own title bar and edges do (the app's only window).
pub mod win {
    use iced::window;
    use iced::Task;

    pub fn drag<M: Send + 'static>() -> Task<M> {
        window::latest().and_then(|id| window::drag(id))
    }
    pub fn toggle_maximize<M: Send + 'static>() -> Task<M> {
        window::latest().and_then(|id| window::toggle_maximize(id))
    }
    pub fn minimize<M: Send + 'static>() -> Task<M> {
        window::latest().and_then(|id| window::minimize(id, true))
    }
    pub fn resize<M: Send + 'static>(d: window::Direction) -> Task<M> {
        window::latest().and_then(move |id| window::drag_resize(id, d))
    }
    pub fn close<M: Send + 'static>() -> Task<M> {
        iced::exit()
    }
}

/// iced's own theme in the look's colours (scroll bars, text fields' insides, selections).
pub fn iced_theme(p: &crate::Pal) -> iced::Theme {
    iced::Theme::custom(
        "Hearth",
        iced::theme::Palette { background: p.bg, text: p.text, primary: p.accent, success: p.good, warning: p.warn, danger: p.danger },
    )
}

// ------------------------------------------------------------------ how the apps draw
// The owner (Nov 19, on the ROG Ally): scrolling in the Rust apps was slow and laggy with tiny-skia alone (the CPU
// draws the whole window again for every step of a scroll). iced draws with the graphics card (wgpu: Vulkan, or
// OpenGL ES where Vulkan isn't there; AMD, Intel and Nvidia) and falls back to tiny-skia by itself when wgpu can't
// start (iced_renderer's fallback compositor: wgpu first, then tiny-skia). Chosen here, once, before iced starts:
//   HEARTH_RENDERER=cpu|gpu|auto, else the first word of ~/.config/hearth/apps-renderer (written only by hand: Hearth
//   never writes it), else auto. An explicit ICED_BACKEND (iced's own: "wgpu", "tiny-skia") always wins.
//   auto: the graphics card when this PC has one (/dev/dri/renderD*), else tiny-skia straight away (no GPU driver is
//   loaded at all: CI, a VM without 3D). And a guard: if the last start with the graphics card didn't get going (the
//   app ended within 4 s of starting: a driver that crashes), the next start in this session draws with the CPU;
//   the mark lives in $XDG_RUNTIME_DIR, so the next sign-in tries the graphics card again (nothing kept for good).
// One line on stderr says which was picked (and, with wgpu, the graphics card iced chose).

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;

static PICKED: OnceLock<&'static str> = OnceLock::new();
static GPU_SEEN: AtomicBool = AtomicBool::new(false);
static PANICKED: AtomicBool = AtomicBool::new(false);
static MARK: OnceLock<String> = OnceLock::new();

/// How this app draws: "gpu" (wgpu, falling back to tiny-skia by itself) or "cpu" (tiny-skia). Decided once.
pub fn renderer(app_id: &str) -> &'static str {
    PICKED.get_or_init(|| {
        let (pick, why) = pick_renderer(app_id);
        if pick == "cpu" {
            std::env::set_var("ICED_BACKEND", "tiny-skia");
            eprintln!("hearth-ui: drawing with the CPU (tiny-skia): {why}");
        } else {
            eprintln!("hearth-ui: drawing with the graphics card if it starts (wgpu, else tiny-skia): {why}");
            start_watch(app_id);
        }
        pick
    })
}

/// Whether wgpu really started (its "Selected" adapter line was seen).
pub fn gpu_started() -> bool {
    GPU_SEEN.load(Ordering::Relaxed)
}

fn configured() -> Option<String> {
    if let Ok(v) = std::env::var("HEARTH_RENDERER") {
        if !v.trim().is_empty() {
            return Some(v.trim().to_lowercase());
        }
    }
    let home = std::env::var("XDG_CONFIG_HOME").ok().filter(|s| !s.is_empty()).or_else(|| std::env::var("HOME").ok().map(|h| format!("{h}/.config")))?;
    let t = std::fs::read_to_string(format!("{home}/hearth/apps-renderer")).ok()?;
    t.split_whitespace().next().map(|w| w.to_lowercase())
}

fn guard_mark(app_id: &str) -> Option<String> {
    let rt = std::env::var("XDG_RUNTIME_DIR").ok().filter(|s| !s.is_empty())?;
    Some(format!("{rt}/hearth-ui-gpu-starting-{}", app_id.replace('/', "_")))
}

fn has_render_node() -> bool {
    std::fs::read_dir("/dev/dri").map(|d| d.flatten().any(|e| e.file_name().to_string_lossy().starts_with("renderD"))).unwrap_or(false)
}

pub fn pick_renderer(app_id: &str) -> (&'static str, String) {
    if let Ok(b) = std::env::var("ICED_BACKEND") {
        if !b.trim().is_empty() {
            let cpu = b.trim().starts_with("tiny");
            return (if cpu { "cpu" } else { "gpu" }, format!("ICED_BACKEND={b}"));
        }
    }
    match configured().as_deref() {
        Some("cpu") | Some("tiny-skia") | Some("software") => return ("cpu", "chosen (HEARTH_RENDERER / apps-renderer)".into()),
        Some("gpu") | Some("wgpu") => return ("gpu", "chosen (HEARTH_RENDERER / apps-renderer)".into()),
        _ => {}
    }
    if !has_render_node() {
        return ("cpu", "no graphics card here (/dev/dri has no render node)".into());
    }
    if let Some(m) = guard_mark(app_id) {
        if std::path::Path::new(&m).exists() {
            return ("cpu", "the last start with the graphics card ended early; the next sign-in tries it again".into());
        }
    }
    ("gpu", "this PC has a graphics card".into())
}

fn start_watch(app_id: &str) {
    // The adapter iced picks is logged by iced_wgpu ("Selected: AdapterInfo {..}"): a small logger shows that one line
    // (and warnings / errors from the graphics side), nothing else.
    let _ = log::set_boxed_logger(Box::new(GpuLog)).map(|_| log::set_max_level(log::LevelFilter::Info));
    let Some(mark) = guard_mark(app_id) else { return };
    let _ = std::fs::write(&mark, b"");
    // Closed normally within the 4 s: the mark goes too (a crash in the driver leaves it; so does a panic).
    let _ = MARK.set(mark.clone());
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        PANICKED.store(true, Ordering::Relaxed);
        prev(info)
    }));
    extern "C" fn at_exit() {
        if !PANICKED.load(Ordering::Relaxed) {
            if let Some(m) = MARK.get() {
                let _ = std::fs::remove_file(m);
            }
        }
    }
    unsafe {
        libc::atexit(at_exit);
    }
    // One short-lived thread: after 4 s the app has started; the mark goes (no thread is left running).
    let _ = std::thread::Builder::new().name("hearth-ui-gpu-guard".into()).stack_size(64 * 1024).spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(4));
        let _ = std::fs::remove_file(&mark);
        if !GPU_SEEN.load(Ordering::Relaxed) {
            eprintln!("hearth-ui: renderer: tiny-skia (the graphics card didn't start, so the CPU draws)");
        }
    });
}

struct GpuLog;

impl log::Log for GpuLog {
    fn enabled(&self, m: &log::Metadata) -> bool {
        let t = m.target();
        (t.starts_with("iced_wgpu") && m.level() <= log::Level::Info) || ((t.starts_with("wgpu") || t.starts_with("iced")) && m.level() <= log::Level::Warn)
    }
    fn log(&self, r: &log::Record) {
        if !self.enabled(r.metadata()) {
            return;
        }
        let msg = r.args().to_string();
        if r.target().starts_with("iced_wgpu") && r.level() == log::Level::Info {
            if let Some(rest) = msg.strip_prefix("Selected: ") {
                GPU_SEEN.store(true, Ordering::Relaxed);
                let one: String = rest.split_whitespace().collect::<Vec<_>>().join(" ");
                let field = |k: &str| one.split(&format!("{k}: ")).nth(1).map(|v| v.split(',').next().unwrap_or("").trim().trim_matches('"').to_string()).unwrap_or_default();
                eprintln!("hearth-ui: renderer: wgpu on {} ({}, {})", field("name"), field("backend"), field("device_type"));
            }
            return;
        }
        eprintln!("hearth-ui: {} {}: {}", r.level(), r.target(), msg.chars().take(300).collect::<String>());
    }
    fn flush(&self) {}
}

// ------------------------------------------------------------------ frames (CI's scroll measurement only)
// HEARTH_UI_FRAMES=FILE: the window's title bar carries a 1 px canvas that counts every frame drawn and writes the
// count to FILE (hearth-apps-rust-test.sh scrolls a long page for 3 s and reads it). Without it: nothing at all.
static FRAMES: AtomicU64 = AtomicU64::new(0);
static BENCH: OnceLock<Option<String>> = OnceLock::new();

pub fn bench() -> Option<&'static str> {
    BENCH.get_or_init(|| std::env::var("HEARTH_UI_FRAMES").ok().filter(|s| !s.is_empty())).as_deref()
}

pub fn frame_tick() {
    if let Some(p) = bench() {
        let n = FRAMES.fetch_add(1, Ordering::Relaxed) + 1;
        let _ = std::fs::write(p, n.to_string());
    }
}
