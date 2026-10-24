//! Notifications (notify-send), like Mantel's screenshots (desktop/open.sh): "Recording saved" with Open and
//! Show in folder. Clicking waits in a small shell of its own, so the app (or the recorder) doesn't have to stay.
//! Paths go to the shell through the environment, never pasted into the script.

use std::path::Path;
use std::process::{Command, Stdio};

const SAVED: &str = r#"act=$(notify-send -a "$N_APP" -i "$N_ICON" -A default=Open -A folder="Show in folder" "$N_TITLE" "$N_BODY" 2>/dev/null) \
  || { notify-send -a "$N_APP" -i "$N_ICON" "$N_TITLE" "$N_BODY" 2>/dev/null; exit 0; }
case "$act" in
  default) xdg-open "$N_FILE" ;;
  folder) s="${NIMBUS_SHARE:-/usr/share/nimbus}/desktop/open.sh"
          if [ -f "$s" ]; then sh "$s" files-at "$N_FILE"; else xdg-open "$(dirname "$N_FILE")"; fi ;;
esac"#;

fn base(app: &str, icon: &str, title: &str, body: &str) -> Command {
    let mut c = Command::new("sh");
    c.env("N_APP", app).env("N_ICON", icon).env("N_TITLE", title).env("N_BODY", body)
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    c
}

fn go(mut c: Command) {
    if !crate::tools::have("notify-send") { return; }
    if let Ok(mut ch) = crate::tools::spawn_detached(&mut c) {
        // Collected in the background so it doesn't linger as a finished process.
        std::thread::spawn(move || { let _ = ch.wait(); });
    }
}

/// "Saved": opens the file, or shows it in the file manager, when clicked.
pub fn saved(app: &str, icon: &str, title: &str, body: &str, file: &Path) {
    let mut c = base(app, icon, title, body);
    c.env("N_FILE", file).arg("-c").arg(SAVED);
    go(c);
}

/// Just a message.
pub fn info(app: &str, icon: &str, title: &str, body: &str) {
    let mut c = base(app, icon, title, body);
    c.arg("-c").arg(r#"notify-send -a "$N_APP" -i "$N_ICON" "$N_TITLE" "$N_BODY""#);
    go(c);
}

/// Opens a file with its app.
pub fn open(file: &Path) -> Result<(), String> {
    let mut c = Command::new("xdg-open");
    c.arg(file).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    crate::tools::spawn_detached(&mut c).map(|mut ch| { std::thread::spawn(move || { let _ = ch.wait(); }); }).map_err(|e| format!("Couldn't open it: {e}"))
}

/// Shows a file in Hearth's file manager (Mantel's open.sh files-at), else opens its folder.
pub fn show(file: &Path) -> Result<(), String> {
    let mut c = Command::new("sh");
    c.env("N_FILE", file).arg("-c").arg(r#"s="${NIMBUS_SHARE:-/usr/share/nimbus}/desktop/open.sh"; if [ -f "$s" ]; then exec sh "$s" files-at "$N_FILE"; else exec xdg-open "$(dirname "$N_FILE")"; fi"#)
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    crate::tools::spawn_detached(&mut c).map(|mut ch| { std::thread::spawn(move || { let _ = ch.wait(); }); }).map_err(|e| format!("Couldn't show it: {e}"))
}

/// A message that goes away by itself after a few seconds ("Recording", "Instant replay is on").
pub fn brief(app: &str, icon: &str, title: &str, body: &str) {
    let mut c = base(app, icon, title, body);
    c.arg("-c").arg(r#"notify-send -t 3000 -a "$N_APP" -i "$N_ICON" "$N_TITLE" "$N_BODY""#);
    go(c);
}
