//! The window every Hearth app opens, and what it shares at start.
//!
//! Each app draws its own title bar, as its approved mockup shows (ui::title_bar), so the window asks for no frame
//! from the desktop; it can still be moved, maximized, minimized and resized from its title bar and edges.

use iced::window;
use iced::{Font, Size};

pub const INTER: Font = Font::with_name("Inter");

/// The window: its app id (the .desktop file's name, so the taskbar shows the right icon), size and smallest size.
pub fn window(app_id: &str, size: (f32, f32), min: (f32, f32)) -> window::Settings {
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
