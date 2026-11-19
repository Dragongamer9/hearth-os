//! Hearth's file chooser: every app's Open / Save window (Mantel's picker.qml, in pure Rust for the Fedora edition).
//! hearth-portal starts it with HEARTH_PICK_REQUEST (the request, JSON: kind open|save|savefiles, title, accept,
//! multiple, directory, filters, filter, folder, name, files, choices) and reads the answer from HEARTH_PICK_RESULT
//! ({paths, filter, choices} or {cancel: true}), written whole when the window closes. Hearth Files' own window, with
//! the bar at the bottom (the name to save as, which files to show, the app's options, Cancel and Open / Save).
//!   hearth-file-chooser [REQUEST.json]     (a file instead of HEARTH_PICK_REQUEST: for checks)

fn main() -> iced::Result {
    if hearth_ui::app::handle_cli(
        "hearth-file-chooser",
        "Hearth's Open / Save window. HEARTH_PICK_REQUEST=request.json HEARTH_PICK_RESULT=result.json hearth-file-chooser",
    ) {
        return Ok(());
    }
    let path = std::env::var("HEARTH_PICK_REQUEST").ok().or_else(|| std::env::args().nth(1).filter(|a| !a.starts_with("--")));
    let req = path.and_then(|p| std::fs::read_to_string(p).ok()).and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok());
    let Some(req) = req else {
        // No request it can read: the app gets "cancelled" at once (never a window that answers nothing).
        if let Ok(r) = std::env::var("HEARTH_PICK_RESULT") {
            let part = format!("{r}.part");
            if std::fs::write(&part, "{\"cancel\": true}").is_ok() {
                let _ = std::fs::rename(&part, &r);
            }
        }
        eprintln!("hearth-file-chooser: no request to answer");
        std::process::exit(1);
    };
    hearth_files::run(hearth_files::Mode::Pick(req))
}
