//! Hearth's look in Hearth's own apps (the Tauri ones: Paint, Photos, Text, Terminal, Disks, Tools...).
//!
//! Mantel's Settings › Appearance picks a look (Steam-style or Hearth), dark or light, and an accent;
//! hosts/linux/desktop/theme/hearth-theme writes it for apps to ~/.config/hearth/theme.css as CSS variables with
//! the names the apps' own style sheets use (--bg, --bg2, --panel, --panel2, --panel3, --line, --text, --muted,
//! --faint, --danger, --ember / --accent / --focus, --flame, --ink, --glow, --r, --corner, --font) and the same
//! tokens to theme.json. This file puts that CSS in every window of the app before its page shows, and follows
//! the file while the app is open, so a new look reaches open apps within a couple of seconds.
//!
//! Each app includes this one file (no crate, so no Cargo.lock changes):
//!     #[path = "../../hearth-apptheme/apptheme.rs"] mod apptheme;
//!     tauri::Builder::default().plugin(apptheme::plugin()) ...
//!     WebviewWindowBuilder::new(...).background_color(apptheme::bg())
//!
//! In the page: `window.hearthTheme` holds the tokens ({bg: "#171a21", ..., scheme: "dark"}) and a `hearth-theme`
//! event fires on window when they change (for colours an app draws itself, on a canvas or in a terminal).
//! The app's own :root colours stay as the fallback (Ember on Windows has no theme file). Without the file on
//! Linux (hearth-theme hasn't run yet) the default look, Steam-style dark, is used.
//!
//! Size: in a desktop window (Desktop Mode, or any desktop that isn't gamescope) desktop.css is added after the
//! look, so the apps' text is the size of Mantel's bar instead of growing with the window's width (their style
//! sheets are made big for Game Mode and TVs). Game Mode (gamescope, `--fullscreen`) keeps the big sizes.
//! `HEARTH_APP_SIZE=desktop|big` overrides the guess (tests, and anyone who wants the big sizes on a desktop).
#![allow(dead_code)]

use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tauri::plugin::{Builder, TauriPlugin};
use tauri::webview::PageLoadEvent;
use tauri::{Manager, Runtime};

/// Steam-style dark, Mantel's default look: `hearth-theme --app-css steam dark` (test_apps_theme.py checks it).
const DEFAULT_CSS: &str = r##"/* Written by hearth-theme (steam, dark, accent #1a9fff): the look for Hearth's apps. */
:root:root{color-scheme:dark;--bg:#171a21;--bg2:#1b2838;--panel:#1e2633;--panel2:#24303f;--panel3:#2a475e;--line:#32404f;--text:#dce5ed;--muted:#8f98a0;--faint:#646b72;--danger:#e5484d;--ember:#1a9fff;--accent:#1a9fff;--flame:#66c0f4;--ink:#06121d;--focus:#1a9fff;--glow:#172737;--chrome:#1b2838;--accentInk:#06121d;--r:4px;--corner:4px;--font:"Inter", system-ui, "Segoe UI", Cantarell, "Noto Sans", sans-serif}
"##;

/// Desktop Mode's size for the apps (the bar's text size; see desktop.css).
const DESKTOP_CSS: &str = include_str!("desktop.css");

/// Is this app in a desktop window? Not in gamescope (Steam Game Mode and Hearth's own Game Mode), not opened
/// `--fullscreen` (how Game Mode opens Hearth's apps), not in Steam's Big Picture, and only on Linux (Ember on
/// Windows is a gamepad shell). `HEARTH_APP_SIZE=desktop` or `big` decides it instead.
pub fn desktop_size() -> bool {
    desktop_size_for(
        cfg!(target_os = "linux"),
        std::env::var("HEARTH_APP_SIZE").ok().as_deref(),
        std::env::args().skip(1).any(|a| a == "--fullscreen"),
        // Desktop Mode's labwc first (as nimbus-mode decides), in case gamescope's variables were left behind.
        std::env::var_os("LABWC_PID").is_none()
            && (std::env::var_os("GAMESCOPE_WAYLAND_DISPLAY").is_some()
                || std::env::var("XDG_CURRENT_DESKTOP").map(|d| d.to_ascii_lowercase().contains("gamescope")).unwrap_or(false)
                || std::env::var("SteamGamepadUI").map(|v| v == "1").unwrap_or(false)),
    )
}

fn desktop_size_for(linux: bool, forced: Option<&str>, fullscreen: bool, gamescope: bool) -> bool {
    match forced {
        Some("desktop") => return true,
        Some("big") => return false,
        _ => {}
    }
    linux && !fullscreen && !gamescope
}

/// What goes in the page: the look, and Desktop Mode's size when on a desktop.
pub fn page_css() -> String {
    let mut c = css();
    if desktop_size() {
        c.push('\n');
        c.push_str(DESKTOP_CSS);
    }
    c
}

/// ~/.config/hearth/theme.css ($HEARTH_THEME_CSS for tests).
pub fn css_path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("HEARTH_THEME_CSS") {
        return Some(PathBuf::from(p));
    }
    let conf = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))?;
    Some(conf.join("hearth").join("theme.css"))
}

/// The CSS for the chosen look.
pub fn css() -> String {
    match css_path().and_then(|p| std::fs::read_to_string(p).ok()) {
        Some(s) if s.contains("--bg:") => s,
        _ if cfg!(target_os = "linux") => DEFAULT_CSS.to_string(),
        _ => String::new(),
    }
}

/// A token's value from the CSS (`token("bg")` -> "#171a21").
pub fn token(name: &str) -> Option<String> {
    let css = css();
    let key = format!("--{}:", name);
    let at = css.find(&key)? + key.len();
    let rest = &css[at..];
    let end = rest.find(|c| c == ';' || c == '}').unwrap_or(rest.len());
    Some(rest[..end].trim().to_string())
}

/// The window's background before the page draws: the look's --bg (Hearth's dark brown without one).
pub fn bg() -> tauri::window::Color {
    let parse = |h: &str| -> Option<(u8, u8, u8)> {
        let h = h.strip_prefix('#')?;
        if h.len() != 6 {
            return None;
        }
        let n = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
        Some((n(0)?, n(2)?, n(4)?))
    };
    let (r, g, b) = token("bg").as_deref().and_then(parse).unwrap_or((15, 11, 9));
    tauri::window::Color(r, g, b, 255)
}

/// `s` as a JavaScript string literal.
fn js_str(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '<' => o.push_str("\\u003c"),
            '\u{2028}' => o.push_str("\\u2028"),
            '\u{2029}' => o.push_str("\\u2029"),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// Runs before each page's own scripts: a <style id="hearth-theme"> with the CSS (its `:root:root` outranks the
/// app's own `:root`), window.hearthTheme, and window.__hearthTheme(css) for later changes.
fn script(css: &str) -> String {
    format!(
        r#"(function(){{
  if (window.__hearthTheme) return;
  var css = {css};
  function tokens(c) {{
    var t = {{}}, re = /--([\w-]+)\s*:\s*([^;}}]+)/g, m;
    while ((m = re.exec(c))) t[m[1]] = m[2].trim();
    var s = /color-scheme\s*:\s*(\w+)/.exec(c);
    t.scheme = s ? s[1] : 'dark';
    return t;
  }}
  function put() {{
    var d = document, root = d.documentElement;
    if (!root) return false;
    var s = d.getElementById('hearth-theme');
    if (!s) {{ s = d.createElement('style'); s.id = 'hearth-theme'; (d.head || root).appendChild(s); }}
    if (s.textContent !== css) s.textContent = css;
    root.classList.toggle('hearth-light', window.hearthTheme.scheme === 'light');
    return true;
  }}
  window.hearthTheme = tokens(css);
  window.__hearthTheme = function(c) {{
    if (typeof c !== 'string' || !c) return;
    var changed = c !== css;
    css = c; window.hearthTheme = tokens(c); put();
    if (changed) try {{ window.dispatchEvent(new CustomEvent('hearth-theme', {{ detail: window.hearthTheme }})); }} catch (e) {{}}
  }};
  if (!put()) {{
    var o = new MutationObserver(function() {{ if (put()) o.disconnect(); }});
    o.observe(document, {{ childList: true, subtree: true }});
  }}
  document.addEventListener('DOMContentLoaded', put);
}})();"#,
        css = js_str(css)
    )
}

fn apply_js(css: &str) -> String {
    format!("window.__hearthTheme && window.__hearthTheme({});", js_str(css))
}

fn stamp() -> Option<(SystemTime, u64)> {
    let m = std::fs::metadata(css_path()?).ok()?;
    Some((m.modified().ok()?, m.len()))
}

/// Checks the file's time every 1.5 s (a stat, nothing more) and gives every window the new look.
fn watch<R: Runtime>(app: tauri::AppHandle<R>) {
    let mut last = stamp();
    loop {
        std::thread::sleep(Duration::from_millis(1500));
        let now = stamp();
        if now != last {
            last = now;
            let js = apply_js(&page_css());
            for w in app.webview_windows().values() {
                let _ = w.eval(js.as_str());
            }
        }
    }
}

/// The plugin: the look in every page, and changes followed while the app is open.
pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("hearth-apptheme")
        .js_init_script(script(&page_css()))
        .on_page_load(|webview, payload| {
            // A page opened after the look changed (the script above carries the look from the app's start).
            if matches!(payload.event(), PageLoadEvent::Finished) {
                let _ = webview.eval(apply_js(&page_css()).as_str());
            }
        })
        .setup(|app, _api| {
            let app = app.clone();
            std::thread::spawn(move || watch(app));
            Ok(())
        })
        .build()
}

#[cfg(test)]
mod size_tests {
    use super::desktop_size_for;

    #[test]
    fn desktop_window_gets_the_bars_size_game_mode_stays_big() {
        assert!(desktop_size_for(true, None, false, false));          // Desktop Mode (labwc), GNOME, KDE
        assert!(!desktop_size_for(true, None, true, false));          // opened --fullscreen from Game Mode
        assert!(!desktop_size_for(true, None, false, true));          // in gamescope
        assert!(!desktop_size_for(false, None, false, false));        // Ember on Windows
        assert!(desktop_size_for(true, Some("desktop"), true, true));
        assert!(!desktop_size_for(true, Some("big"), false, false));
    }
}
