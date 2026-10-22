//! Resolution, refresh rate and scale (Settings › Display). Port of hosts/linux/src/display.js. Which tool does
//! the work depends on where Hearth runs:
//!   gamescope (Game Mode)       an external screen's real modes from gamescope (GAMESCOPE_DISPLAY_MODE_LIST_EXTERNAL),
//!                               else the kernel's sizes; a new mode is a gamescope option (-W -H -r: gamescope picks
//!                               that exact mode when it starts, or falls back to the screen's own), applied by
//!                               restarting Game Mode, and switched back by itself unless you keep it (a screen that
//!                               can't show it stays black: 20 seconds later it's back as it was). The built-in
//!                               screen's refresh rate changes on the spot (GAMESCOPE_DYNAMIC_REFRESH, as SteamOS
//!                               does). Scale is Hearth's own interface zoom
//!   kde       (Plasma)          kscreen-doctor; Plasma remembers the settings itself
//!   wlr       (labwc, Sway...)  wlr-randr; saved to ~/.config/nimbus/display.sh, which Desktop Mode runs at start
//!   xrandr    (X11 desktops)    xrandr; scale is Hearth's interface zoom

use serde_json::{json, Value};
use std::path::Path;

pub const SCALES: [u32; 5] = [100, 125, 150, 175, 200];
// gamescope can't list refresh rates; these are the usual ones (it uses the nearest the screen has).
const COMMON_RATES: [f64; 9] = [30.0, 40.0, 50.0, 60.0, 75.0, 90.0, 120.0, 144.0, 165.0];

#[derive(Debug, Clone, PartialEq)]
pub struct Mode { pub id: String, pub w: u32, pub h: u32, pub rate: f64, pub exact: f64 }

#[derive(Debug, Clone, Default)]
pub struct Screen { pub output: String, pub modes: Vec<Mode>, pub current: Option<(u32, u32, f64)>, pub preferred: Option<(u32, u32)>, pub scale: u32 }

fn round2(n: f64) -> f64 { (n * 100.0).round() / 100.0 }
fn mode(w: u32, h: u32, rate: f64) -> Mode { Mode { id: String::new(), w, h, rate: round2(rate), exact: rate } }

pub fn detect() -> Option<&'static str> {
    let env = |k: &str| std::env::var(k).unwrap_or_default();
    let desk = env("XDG_CURRENT_DESKTOP").to_uppercase();
    if !env("GAMESCOPE_WAYLAND_DISPLAY").is_empty() { return Some("gamescope"); }
    if desk.contains("KDE") && crate::system::has("kscreen-doctor") { return Some("kde"); }
    if !env("WAYLAND_DISPLAY").is_empty() && crate::system::has("wlr-randr") && !desk.contains("GNOME") { return Some("wlr"); }
    if !env("DISPLAY").is_empty() && env("WAYLAND_DISPLAY").is_empty() && crate::system::has("xrandr") { return Some("xrandr"); }
    None
}

pub fn parse_kscreen(text: &str) -> Option<Screen> {
    let j: Value = serde_json::from_str(text).ok()?;
    let outs: Vec<&Value> = j["outputs"].as_array()?.iter().filter(|o| o["enabled"] == true && o["connected"] != false).collect();
    let o = outs.iter().find(|o| o["priority"] == 1).or(outs.first())?;
    let modes: Vec<Mode> = o["modes"].as_array().cloned().unwrap_or_default().iter().map(|m| Mode {
        id: m["id"].to_string().trim_matches('"').to_string(), w: m["size"]["width"].as_u64().unwrap_or(0) as u32, h: m["size"]["height"].as_u64().unwrap_or(0) as u32,
        rate: round2(m["refreshRate"].as_f64().unwrap_or(0.0)), exact: m["refreshRate"].as_f64().unwrap_or(0.0) }).collect();
    let cur_id = o["currentModeId"].to_string().trim_matches('"').to_string();
    let cur = modes.iter().find(|m| m.id == cur_id).or(modes.first()).map(|m| (m.w, m.h, m.rate));
    Some(Screen { output: o["name"].as_str().unwrap_or("").into(), modes, current: cur, preferred: None, scale: (o["scale"].as_f64().unwrap_or(1.0) * 100.0).round() as u32 })
}

pub fn parse_wlr(text: &str) -> Option<Screen> {
    let j: Value = serde_json::from_str(text).ok()?;
    let o = j.as_array()?.iter().find(|o| o["enabled"] == true)?;
    let list = o["modes"].as_array().cloned().unwrap_or_default();
    let wh = |m: &Value| (m["width"].as_u64().unwrap_or(0) as u32, m["height"].as_u64().unwrap_or(0) as u32);
    let modes = list.iter().map(|m| { let (w, h) = wh(m); mode(w, h, m["refresh"].as_f64().unwrap_or(0.0)) }).collect();
    let cur = list.iter().find(|m| m["current"] == true).map(|m| { let (w, h) = wh(m); (w, h, round2(m["refresh"].as_f64().unwrap_or(0.0))) });
    let pref = list.iter().find(|m| m["preferred"] == true).map(wh);
    Some(Screen { output: o["name"].as_str().unwrap_or("").into(), modes, current: cur, preferred: pref, scale: (o["scale"].as_f64().unwrap_or(1.0) * 100.0).round() as u32 })
}

pub fn parse_xrandr(text: &str) -> Option<Screen> {
    let mut outs: Vec<(Screen, bool)> = Vec::new();
    let mut in_out = false;
    for line in text.lines() {
        if !line.starts_with(char::is_whitespace) {
            in_out = false;
            let mut it = line.split_whitespace();
            if let (Some(name), Some("connected")) = (it.next(), it.next()) {
                outs.push((Screen { output: name.into(), scale: 100, ..Default::default() }, line.contains(" connected primary")));
                in_out = true;
            }
            continue;
        }
        if !in_out { continue; }
        let mut it = line.split_whitespace();
        let Some(res) = it.next() else { continue };
        let Some((w, h)) = res.split_once('x') else { continue };
        let (Ok(w), Ok(h)) = (w.parse::<u32>(), h.trim_end_matches(|c: char| !c.is_ascii_digit()).parse::<u32>()) else { continue };
        let s = &mut outs.last_mut().unwrap().0;
        for tok in it {
            let num: String = tok.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
            let Ok(r) = num.parse::<f64>() else { continue };
            let m = mode(w, h, r);
            if tok.contains('*') { s.current = Some((w, h, m.rate)); }
            if tok.contains('+') { s.preferred = Some((w, h)); }
            s.modes.push(m);
        }
    }
    let pick = outs.iter().position(|(s, p)| *p && !s.modes.is_empty()).or_else(|| outs.iter().position(|(s, _)| !s.modes.is_empty()))?;
    Some(outs.swap_remove(pick).0)
}

/// The connected screen's resolutions from the kernel (/sys/class/drm/*/modes), native first.
pub fn drm_modes(root: &Path) -> Option<(String, Vec<(u32, u32)>)> {
    let mut names: Vec<String> = std::fs::read_dir(root).ok()?.flatten().map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.starts_with("card") && n.contains('-')).collect();
    names.sort();
    for n in names {
        if std::fs::read_to_string(root.join(&n).join("status")).unwrap_or_default().trim() != "connected" { continue; }
        let mut list = Vec::new();
        for l in std::fs::read_to_string(root.join(&n).join("modes")).unwrap_or_default().lines() {
            if let Some((w, h)) = l.split_once('x') {
                let h: String = h.chars().take_while(|c| c.is_ascii_digit()).collect();
                if let (Ok(w), Ok(h)) = (w.parse(), h.parse()) { if !list.contains(&(w, h)) { list.push((w, h)); } }
            }
        }
        if !list.is_empty() { return Some((n.split_once('-').map(|x| x.1).unwrap_or(&n).to_string(), list)); }
    }
    None
}

/// gamescope's list of an external screen's modes: "1920x1080@60 1920x1080@144 3840x2160@60 ...".
pub fn parse_gamescope_modes(text: &str) -> Vec<Mode> {
    text.split_whitespace().filter_map(|t| {
        let (wh, r) = t.split_once('@')?;
        let (w, h) = wh.split_once('x')?;
        Some(mode(w.parse().ok()?, h.parse().ok()?, r.parse::<f64>().ok()?))
    }).filter(|m| m.w > 0 && m.h > 0 && m.rate > 0.0).collect()
}

/// The external screen's modes as gamescope sees them (empty on the built-in screen or outside Game Mode).
fn gamescope_external_modes() -> Vec<Mode> {
    if !crate::gamescope::inside() { return vec![]; }
    if crate::gamescope::cardinals("GAMESCOPE_DISPLAY_IS_EXTERNAL", None).first().copied() != Some(1) { return vec![]; }
    parse_gamescope_modes(&crate::gamescope::root_str("GAMESCOPE_DISPLAY_MODE_LIST_EXTERNAL"))
}

/// NIMBUS_GAMESCOPE_ARGS in session.conf: read it, or set our own options in it (keeping everything else).
pub fn gamescope_args_text(text: &str, set: Option<&str>) -> (String, String) {
    let re = regex::Regex::new(r#"(?m)^NIMBUS_GAMESCOPE_ARGS="([^"]*)""#).unwrap();
    let args = re.captures(text).map(|c| c[1].to_string()).unwrap_or_default();
    let Some(set) = set else { return (args, text.to_string()) };
    let ours = regex::Regex::new(r"(^|\s)-(W|H|r)\s+\S+|(^|\s)--(output-width|output-height|nested-refresh)[= ]\S+").unwrap();
    let kept = ours.replace_all(&args, "").trim().to_string();
    let next = [kept.as_str(), set].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" ");
    let line = format!("NIMBUS_GAMESCOPE_ARGS=\"{next}\"");
    let out = if re.is_match(text) { re.replace(text, regex::NoExpand(&line)).into_owned() }
        else { format!("{}{}{line}\n", text, if !text.is_empty() && !text.ends_with('\n') { "\n" } else { "" }) };
    (next, out)
}
fn gamescope_args(set: Option<&str>) -> String {
    let file = crate::admin::session_conf();
    let text = std::fs::read_to_string(&file).unwrap_or_default();
    let (args, out) = gamescope_args_text(&text, set);
    // All at once: the session reads this file when Game Mode starts, right after a change restarts it.
    if set.is_some() { let _ = crate::paths::write_atomic(&file, out.as_bytes()); }
    args
}
fn flag(args: &str, f: &str) -> Option<u32> { regex::Regex::new(&format!(r"-{f}\s+(\d+)")).unwrap().captures(args).and_then(|c| c[1].parse().ok()) }

pub fn read(backend: &str, zoom: u32) -> Option<Screen> {
    match backend {
        "kde" => parse_kscreen(&crate::system::run("kscreen-doctor", &["-j"]).ok()?),
        "wlr" => parse_wlr(&crate::system::run("wlr-randr", &["--json"]).ok()?),
        "xrandr" => parse_xrandr(&crate::system::run("xrandr", &["--query"]).ok()?),
        "gamescope" => {
            let drm = drm_modes(Path::new("/sys/class/drm"));
            let args = gamescope_args(None);
            let external = gamescope_external_modes();
            if !external.is_empty() {
                // The screen's own modes: only these can be picked. What's showing is the chosen one if it's in the
                // list (gamescope then picked exactly it), else the screen's own first mode.
                let first = (external[0].w, external[0].h, external[0].rate);
                let chosen = match (flag(&args, "W"), flag(&args, "H"), flag(&args, "r")) {
                    (Some(w), Some(h), Some(r)) => external.iter().find(|m| m.w == w && m.h == h && m.rate.round() as u32 == r).map(|m| (m.w, m.h, m.rate)),
                    _ => None,
                };
                return Some(Screen { output: drm.map(|d| d.0).unwrap_or("screen".into()), current: Some(chosen.unwrap_or(first)), preferred: Some((first.0, first.1)), modes: external, scale: zoom });
            }
            let native = drm.as_ref().map(|d| d.1[0]).unwrap_or((1280, 800));
            let (w, h) = (flag(&args, "W").unwrap_or(native.0), flag(&args, "H").unwrap_or(native.1));
            let dynamic = crate::gamescope::cardinals("GAMESCOPE_DYNAMIC_REFRESH", None).first().copied().filter(|r| *r > 0);
            let rate = dynamic.or(flag(&args, "r").map(|r| r as u64)).unwrap_or(60) as f64;
            let sizes = drm.as_ref().map(|d| d.1.clone()).unwrap_or(vec![(w, h)]);
            // The built-in screen: its own size only, at the usual handheld rates (gamescope's display scripts know
            // which the panel can do and use the nearest).
            let modes = sizes.iter().take(1).flat_map(|&(w, h)| COMMON_RATES.iter().filter(|r| **r >= 40.0 && **r <= 120.0).map(move |&r| mode(w, h, r))).collect();
            Some(Screen { output: drm.map(|d| d.0).unwrap_or("screen".into()), modes, current: Some((w, h, rate)), preferred: Some(sizes[0]), scale: zoom })
        }
        _ => None,
    }
}

/// What the UI shows, in the Windows host's shape: the current mode, every resolution (native first) with its
/// refresh rates in whole hertz, and the scale choices.
pub fn get(zoom: u32) -> Value {
    let Some(backend) = detect() else { return json!({}) };
    let Some(d) = read(backend, zoom) else { return json!({}) };
    let (w, h, rate) = d.current.unwrap_or((0, 0, 60.0));
    let mut groups: Vec<(u32, u32, Vec<f64>)> = Vec::new();
    for m in &d.modes {
        match groups.iter_mut().find(|g| g.0 == m.w && g.1 == m.h) {
            Some(g) => if !g.2.iter().any(|r| (r - m.rate).abs() < 0.01) { g.2.push(m.rate) },
            None => groups.push((m.w, m.h, vec![m.rate])),
        }
    }
    groups.sort_by(|a, b| (b.0 * b.1).cmp(&(a.0 * a.1)).then(b.0.cmp(&a.0)));
    if let Some(p) = d.preferred.and_then(|p| groups.iter().position(|g| (g.0, g.1) == p)) { let g = groups.remove(p); groups.insert(0, g); }
    let zoom_scale = backend == "gamescope" || backend == "xrandr";
    let pct = if zoom_scale { zoom } else { d.scale };
    let modes: Vec<Value> = groups.iter().map(|(w, h, rates)| {
        let mut r: Vec<u32> = rates.iter().map(|x| x.round() as u32).collect();
        r.sort_by(|a, b| b.cmp(a)); r.dedup();
        json!({"width": w, "height": h, "rates": r})
    }).collect();
    json!({"resolution": format!("{w} × {h}"), "scale": format!("{pct}%"), "width": w, "height": h, "refreshRate": rate.round() as u32,
           "modes": modes, "scalePercent": pct, "scaleRecommended": 100, "scales": SCALES,
           "backend": backend, "scaleIsInterface": zoom_scale, "appliesOnRestart": backend == "gamescope" && !gamescope_external_modes().is_empty(),
           "pending": backend == "gamescope" && pending()})
}

/// Changes the mode. Returns true when Game Mode has to restart for it (gamescope).
pub fn set_mode(w: u32, h: u32, rate: f64, zoom: u32) -> Result<bool, String> {
    let backend = detect().ok_or("Changing the resolution isn't supported on this desktop")?;
    let d = read(backend, zoom).ok_or("Changing the resolution isn't supported on this desktop")?;
    let m = d.modes.iter().filter(|x| x.w == w && x.h == h).min_by(|a, b| (a.rate - rate).abs().partial_cmp(&(b.rate - rate).abs()).unwrap())
        .ok_or(format!("{w} × {h} isn't available on this screen"))?.clone();
    match backend {
        "kde" => { crate::system::run("kscreen-doctor", &[&format!("output.{}.mode.{}", d.output, m.id)])?; }
        "wlr" => {
            let spec = format!("{}x{}@{}Hz", m.w, m.h, m.exact);
            crate::system::run("wlr-randr", &["--output", &d.output, "--mode", &spec])?;
            save_wlr(&d.output, "mode", &spec);
        }
        "xrandr" => { crate::system::run("xrandr", &["--output", &d.output, "--mode", &format!("{}x{}", m.w, m.h), "--rate", &m.rate.to_string()])?; }
        _ => {
            let external = gamescope_external_modes();
            if external.is_empty() {
                // The built-in screen: the refresh rate changes on the spot, as on SteamOS. No restart, no -r.
                crate::gamescope::set_root("GAMESCOPE_DYNAMIC_REFRESH", m.rate.round() as u64)?;
                gamescope_args(Some(""));
                return Ok(false);
            }
            if !external.iter().any(|x| x.w == m.w && x.h == m.h && x.rate.round() == m.rate.round()) {
                return Err(format!("This screen can't show {} × {} at {} Hz", m.w, m.h, m.rate.round()));
            }
            let before = gamescope_args(None);
            gamescope_args(Some(&format!("-W {} -H {} -r {}", m.w, m.h, m.rate.round())));
            pending_save(&before);
            return Ok(true);
        }
    }
    Ok(false)
}

// ------------------------------------------------------------------ keep or switch back (Game Mode)

fn pending_file() -> std::path::PathBuf { crate::paths::config_dir().join("display-pending.json") }

/// A new Game Mode mode is on trial: the options before it, to go back to.
fn pending_save(before: &str) {
    let _ = std::fs::write(pending_file(), json!({"before": before, "at": crate::library::now_ms()}).to_string());
}

/// Whether a new mode is waiting for "Keep" (display.get shows it; the UI asks).
pub fn pending() -> bool { pending_file().exists() }

/// "Keep these display settings".
pub fn keep() { let _ = std::fs::remove_file(pending_file()); }

/// Back to the options before the trial. Returns true when there was one (Game Mode then restarts).
pub fn revert() -> bool {
    let Some(before) = std::fs::read(pending_file()).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()).and_then(|v| v["before"].as_str().map(str::to_string)) else { return false };
    let file = crate::admin::session_conf();
    let text = std::fs::read_to_string(&file).unwrap_or_default();
    // Put back exactly what was there (our own options removed first, then the old ones of them added back).
    let old_ours: Vec<String> = regex::Regex::new(r"(^|\s)-(W|H|r)\s+\S+").unwrap().find_iter(&before).map(|m| m.as_str().trim().to_string()).collect();
    let (_, out) = gamescope_args_text(&text, Some(&old_ours.join(" ")));
    let _ = crate::paths::write_atomic(&file, out.as_bytes());
    keep();
    crate::bridge::log(&format!("display: the new mode wasn't kept; back to \"{}\"", before));
    true
}

/// Started with Game Mode: a mode on trial that isn't kept within 20 seconds is switched back (`restart` then
/// restarts Game Mode). A screen that can't show the mode stays black, and this brings it back by itself.
pub fn watch_pending(restart: std::sync::Arc<dyn Fn() + Send + Sync>) {
    if !pending() { return; }
    std::thread::spawn(move || {
        for _ in 0..40 { std::thread::sleep(std::time::Duration::from_millis(500)); if !pending() { return; } }
        if revert() { restart(); }
    });
}

/// Sets the scale. Returns Some(percent) when it's Hearth's own interface zoom (the host applies it).
pub fn set_scale(pct: u32, zoom: u32) -> Result<Option<u32>, String> {
    if !SCALES.contains(&pct) { return Err("Unknown scale".into()); }
    match detect() {
        Some("kde") => { let d = read("kde", zoom).ok_or("No screen")?; crate::system::run("kscreen-doctor", &[&format!("output.{}.scale.{}", d.output, pct as f64 / 100.0)])?; Ok(None) }
        Some("wlr") => {
            let d = read("wlr", zoom).ok_or("No screen")?;
            let v = (pct as f64 / 100.0).to_string();
            crate::system::run("wlr-randr", &["--output", &d.output, "--scale", &v])?;
            save_wlr(&d.output, "scale", &v);
            Ok(None)
        }
        _ => Ok(Some(pct)),
    }
}

/// Desktop Mode (labwc) forgets wlr-randr changes on restart: they're kept in a script its autostart runs.
fn save_wlr(output: &str, key: &str, value: &str) {
    let conf = crate::paths::config_dir();
    let file = conf.join("display.json");
    let mut saved: Value = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).filter(Value::is_object).unwrap_or(json!({}));
    if !saved[output].is_object() { saved[output] = json!({}); }
    saved[output][key] = json!(value);
    let _ = std::fs::create_dir_all(&conf);
    let _ = std::fs::write(&file, serde_json::to_string_pretty(&saved).unwrap());
    let lines: Vec<String> = saved.as_object().unwrap().iter().map(|(o, c)| format!("wlr-randr --output '{o}'{}{}",
        c["mode"].as_str().map(|m| format!(" --mode '{m}'")).unwrap_or_default(), c["scale"].as_str().map(|s| format!(" --scale {s}")).unwrap_or_default())).collect();
    let sh = conf.join("display.sh");
    let _ = std::fs::write(&sh, format!("#!/bin/sh\n# Written by Hearth › Settings › Display\n{}\n", lines.join("\n")));
    let _ = crate::system::run("chmod", &["755", &sh.to_string_lossy()]);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gamescope_mode_list() {
        let m = parse_gamescope_modes("3840x2160@60 1920x1080@144 1920x1080@60 junk 0x0@60");
        assert_eq!(m.len(), 3);
        assert_eq!((m[1].w, m[1].h, m[1].rate), (1920, 1080, 144.0));
        assert!(parse_gamescope_modes("").is_empty());
    }
    #[test]
    fn reads_the_tools() {
        let x = parse_xrandr("Screen 0: minimum 8 x 8\nHDMI-1 connected primary 1920x1080+0+0\n   1920x1080     60.00*+  59.94    50.00\n   1280x720      60.00\nDP-1 disconnected\n").unwrap();
        assert_eq!(x.output, "HDMI-1"); assert_eq!(x.current, Some((1920, 1080, 60.0))); assert_eq!(x.preferred, Some((1920, 1080))); assert_eq!(x.modes.len(), 4);
        let w = parse_wlr(r#"[{"name":"eDP-1","enabled":true,"scale":1.5,"modes":[{"width":2560,"height":1600,"refresh":165.0,"preferred":true,"current":true},{"width":1920,"height":1200,"refresh":60.0}]}]"#).unwrap();
        assert_eq!((w.scale, w.current, w.preferred), (150, Some((2560, 1600, 165.0)), Some((2560, 1600))));
        let k = parse_kscreen(r#"{"outputs":[{"name":"DP-2","enabled":true,"priority":1,"scale":1,"currentModeId":"3","modes":[{"id":"3","size":{"width":3840,"height":2160},"refreshRate":59.997}]}]}"#).unwrap();
        assert_eq!(k.current, Some((3840, 2160, 60.0)));
    }
    #[test]
    fn gamescope_options_keep_the_rest() {
        let (args, text) = gamescope_args_text("NIMBUS_START=game\nNIMBUS_GAMESCOPE_ARGS=\"--hdr-enabled -W 1280 -H 720 -r 60\"\n", Some("-W 1920 -H 1080 -r 120"));
        assert_eq!(args, "--hdr-enabled -W 1920 -H 1080 -r 120");
        assert!(text.starts_with("NIMBUS_START=game\n") && text.contains(&format!("\"{args}\"")));
        let (_, t2) = gamescope_args_text("A=1", Some("-W 800 -H 600 -r 60"));
        assert_eq!(t2, "A=1\nNIMBUS_GAMESCOPE_ARGS=\"-W 800 -H 600 -r 60\"\n");
    }
}
