//! Hearth Files: the file manager (Mantel's Hearth Files, in pure Rust for the Fedora edition; see lib.rs).
//!   hearth-files-app [FOLDER | FILE | C:\… | smb://… ] [--props] [--page home|thispc|apps|c|PATH] [--view NAME] [--show menu|view|pane|tabs|select|props|rename] [--describe WORDS]
//! A file opens its folder with it picked; --props shows only its Properties. --show is for CI's pictures.

fn main() -> iced::Result {
    if hearth_ui::app::handle_cli(
        "hearth-files-app",
        "Hearth Files: your folders, drives and files.\n  hearth-files-app [FOLDER|FILE] [--props] [--page home|thispc|apps|c|PATH] [--view xlarge|large|grid|small|tiles|compact|details]",
    ) {
        return Ok(());
    }
    // The first argument that isn't an option (or an option's value) is where to open.
    let mut open = String::new();
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        if matches!(a.as_str(), "--page" | "--view" | "--show" | "--describe") {
            let _ = it.next();
            continue;
        }
        if a.starts_with("--") {
            continue;
        }
        open = a;
        break;
    }
    let props = hearth_ui::app::flag("--props");
    hearth_files::run(hearth_files::Mode::Files { open, props })
}
