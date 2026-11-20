//! "Sharper graphics" (vkBasalt) for a game (the owner, Nov 20 01:36: vkBasalt in 1.16 on the Fedora edition, a
//! per-game switch). vkBasalt is a Vulkan layer that sharpens the picture (AMD's Contrast Adaptive Sharpening, CAS),
//! can smooth edges (SMAA) and can change colours with a LUT. The Fedora image installs Fedora's `vkBasalt` package
//! (os-bases/anatase/build.sh); where it isn't installed the switch says so and does nothing.
//!
//! The settings are a game's options (game_options.rs, `gameOptions[id].vkbasalt` in Hearth's library file, written
//! with the settings helpers like every other per-game setting): off unless you turn it on. When it's on, Hearth's
//! starts of that game (Play in Hearth Games, Steam Game Mode's `nimbus --play-now <id>`, a desktop icon) carry
//! ENABLE_VKBASALT=1 and VKBASALT_CONFIG_FILE pointing at the game's own vkBasalt.conf in Hearth's config folder
//! (~/.config/nimbus/vkbasalt/<game>.conf), made from those settings. manual::apply adds them, so every kind of
//! start that applies a game's options gets them (games you added, GOG, Epic, Amazon, Battle.net through Faugus
//! Launcher, Recomps, Steam games Hearth starts directly).
//!
//! Games Hearth doesn't start can still use it: Steam's own launch options (`ENABLE_VKBASALT=1 %command%`, copied
//! from Hearth Games; Hearth never edits Steam's files) and Faugus Launcher's own Launch arguments field.

use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

/// What the switch starts as: off, and when it's turned on, sharpening at 40 % with Home turning the effects off and
/// on while you play.
pub fn defaults() -> Value {
    json!({"on": false, "strength": 40, "smaa": false, "vibrance": false, "key": "Home"})
}

/// Keys vkBasalt's toggleKey can be (X11 key names), Home first.
pub const KEYS: &[&str] = &["Home", "End", "Insert", "Pause", "Scroll_Lock", "Print", "F12"];

/// What goes in a game's launch options for Steam (copied from Hearth Games, pasted in Steam › game › Properties).
pub const STEAM_OPTION: &str = "ENABLE_VKBASALT=1 %command%";
/// What goes in Faugus Launcher's own Launch arguments field for a game you added there.
pub const FAUGUS_ARG: &str = "ENABLE_VKBASALT=1";

/// The known parts of a change, each the right kind of value (a missing part stays as it was: set merges).
pub fn clean(v: &Value) -> Map<String, Value> {
    let mut out = Map::new();
    let Some(m) = v.as_object() else { return out };
    for k in ["on", "smaa", "vibrance"] {
        if let Some(x) = m.get(k) {
            let b = x.as_bool().or_else(|| x.as_str().map(|s| s == "true" || s == "1")).or_else(|| x.as_f64().map(|n| n != 0.0));
            if let Some(b) = b { out.insert(k.into(), json!(b)); }
        }
    }
    if let Some(x) = m.get("strength") {
        if let Some(n) = x.as_f64().or_else(|| x.as_str().and_then(|s| s.trim().parse().ok())).filter(|n: &f64| n.is_finite()) {
            out.insert("strength".into(), json!(n.round().clamp(0.0, 100.0) as u64));
        }
    }
    if let Some(k) = m.get("key").and_then(Value::as_str).filter(|k| KEYS.contains(k)) { out.insert("key".into(), json!(k)); }
    out
}

/// A game's settings: the defaults with what's saved on top (anything damaged in the file is left out, not saved over).
pub fn settings(saved: &Value) -> Value {
    let mut o = defaults();
    for (k, v) in clean(saved) { o[k] = v; }
    o
}

/// Where a game's vkBasalt.conf goes: Hearth's config folder, vkbasalt/<game id, made safe>.conf.
pub fn conf_path(config_dir: &Path, id: &str) -> PathBuf {
    let safe: String = id.chars().map(|c| if c.is_ascii_alphanumeric() || "._-".contains(c) { c } else { '_' }).take(120).collect();
    config_dir.join("vkbasalt").join(format!("{}.conf", if safe.is_empty() { "game".into() } else { safe }))
}
/// The colour boost's LUT, shared by every game (vkbasalt/colour-boost.cube).
pub fn lut_path(config_dir: &Path) -> PathBuf { config_dir.join("vkbasalt").join("colour-boost.cube") }
/// The conf for Steam games' launch option with Hearth's sharpening (vkbasalt/steam.conf; the defaults turned on).
pub fn shared_conf_path(config_dir: &Path) -> PathBuf { config_dir.join("vkbasalt").join("steam.conf") }

/// vkBasalt's effects, in the order they run: edges smoothed first, then sharpened, then the colours.
pub fn effects(s: &Value) -> Vec<&'static str> {
    let mut e = vec![];
    if s["smaa"] == true { e.push("smaa"); }
    e.push("cas");
    if s["vibrance"] == true { e.push("lut"); }
    e
}

/// The game's vkBasalt.conf (pure, for the tests). `lut`: the colour boost's file.
pub fn conf_text(s: &Value, lut: &Path) -> String {
    let s = settings(s);
    let strength = s["strength"].as_u64().unwrap_or(40);
    let key = s["key"].as_str().unwrap_or("Home");
    let mut t = String::from("# Made by Hearth from this game's Sharper graphics settings (Hearth Games › the game's page).\n# Hearth writes it again when you change them, so change them there.\n");
    t.push_str(&format!("effects = {}\n", effects(&s).join(":")));
    t.push_str(&format!("casSharpness = {:.2}\n", strength as f64 / 100.0));
    if s["vibrance"] == true { t.push_str(&format!("lutFile = {}\n", lut.display())); }
    t.push_str(&format!("toggleKey = {key}\nenableOnLaunch = True\n"));
    t
}

/// A light colour boost (saturation × 1.15, brightness kept) as a 16×16×16 .cube LUT, vkBasalt's own lut effect.
pub fn lut_text() -> String {
    const N: usize = 16;
    let mut t = String::from("TITLE \"Hearth colour boost\"\nLUT_3D_SIZE 16\nDOMAIN_MIN 0 0 0\nDOMAIN_MAX 1 1 1\n");
    for b in 0..N { for g in 0..N { for r in 0..N {
        let (r, g, b) = (r as f64 / (N - 1) as f64, g as f64 / (N - 1) as f64, b as f64 / (N - 1) as f64);
        let y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        let f = |c: f64| (y + (c - y) * 1.15).clamp(0.0, 1.0);
        t.push_str(&format!("{:.4} {:.4} {:.4}\n", f(r), f(g), f(b)));
    } } }
    t
}

/// Writes the conf (and the LUT when it's used) from the settings, atomically with a .bak (settings_file); an
/// unchanged file isn't written again.
pub fn write_conf(path: &Path, s: &Value) -> Result<(), String> {
    let dir = path.parent().ok_or("No folder for vkBasalt's settings")?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let lut = dir.join("colour-boost.cube");
    if settings(s)["vibrance"] == true { crate::settings_file::write_text(&lut, &lut_text()).map_err(|e| e.to_string())?; }
    crate::settings_file::write_text(path, &conf_text(s, &lut)).map(|_| ()).map_err(|e| e.to_string())
}

/// Whether vkBasalt's Vulkan layer is installed: its layer file in one of Vulkan's implicit layer folders.
pub fn available() -> bool {
    if let Some(v) = std::env::var_os("HEARTH_VKBASALT_FIXTURE") { return v == "1"; }   // CI's pictures
    let home = crate::paths::home();
    let mut dirs: Vec<PathBuf> = ["/usr/share/vulkan/implicit_layer.d", "/etc/vulkan/implicit_layer.d", "/usr/local/share/vulkan/implicit_layer.d"]
        .iter().map(PathBuf::from).collect();
    dirs.push(home.join(".local/share/vulkan/implicit_layer.d"));
    if let Some(x) = std::env::var_os("XDG_DATA_HOME") { dirs.push(PathBuf::from(x).join("vulkan/implicit_layer.d")); }
    dirs.iter().any(|d| std::fs::read_dir(d).map(|rd| rd.flatten().any(|e| e.file_name().to_string_lossy().to_lowercase().starts_with("vkbasalt"))).unwrap_or(false))
}

/// The environment a start of the game gets (manual::apply): nothing while the switch is off; when it's on,
/// ENABLE_VKBASALT=1 and the game's conf (game_options::get puts its path in `vkbasaltConf`), written first if it
/// isn't there or is out of date. A conf that can't be written: vkBasalt's own settings, and the log says why.
pub fn launch_env(opts: &Value) -> Vec<(String, String)> {
    let s = &opts["vkbasalt"];
    if s["on"] != true { return vec![]; }
    let mut env = vec![("ENABLE_VKBASALT".to_string(), "1".to_string())];
    if let Some(p) = opts["vkbasaltConf"].as_str().filter(|p| !p.is_empty()) {
        match write_conf(Path::new(p), s) {
            Ok(()) => env.push(("VKBASALT_CONFIG_FILE".into(), p.into())),
            Err(e) => crate::bridge::log(&format!("vkbasalt: couldn't write {p}: {e}; vkBasalt's own settings this time")),
        }
    }
    env
}

/// What a game's page shows (read only: opening the page writes nothing).
pub fn status(id: &str, saved: &Value, config_dir: &Path) -> Value {
    let s = settings(saved);
    let conf = conf_path(config_dir, id);
    json!({"available": available(), "on": s["on"], "strength": s["strength"], "smaa": s["smaa"], "vibrance": s["vibrance"],
           "key": s["key"], "keys": KEYS, "conf": conf, "env": format!("ENABLE_VKBASALT=1 VKBASALT_CONFIG_FILE={}", conf.display()),
           "steamOption": STEAM_OPTION, "faugusArg": FAUGUS_ARG})
}

/// Steam and Faugus games (Hearth Games › Stores): the launch options to copy. `steamOptionHearth` uses Hearth's
/// sharpening (steam.conf, made when you copy it: vkbasalt.shared).
pub fn info(config_dir: &Path) -> Value {
    let shared = shared_conf_path(config_dir);
    json!({"available": available(), "key": "Home", "steamOption": STEAM_OPTION, "faugusArg": FAUGUS_ARG,
           "steamOptionHearth": format!("ENABLE_VKBASALT=1 VKBASALT_CONFIG_FILE=\"{}\" %command%", shared.display()),
           "faugusArgHearth": format!("ENABLE_VKBASALT=1 VKBASALT_CONFIG_FILE=\"{}\"", shared.display()), "conf": shared})
}

/// Makes steam.conf (sharpening on, the defaults) when you copy the launch option that uses it.
pub fn write_shared(config_dir: &Path) -> Result<Value, String> {
    write_conf(&shared_conf_path(config_dir), &json!({"on": true}))?;
    Ok(info(config_dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_unless_turned_on() {
        assert_eq!(defaults()["on"], false);
        assert!(launch_env(&json!({"vkbasalt": defaults()})).is_empty());
        assert!(launch_env(&json!({})).is_empty());
        assert!(launch_env(&json!({"vkbasalt": {"on": "yes"}})).is_empty());
    }

    #[test]
    fn the_conf_has_the_effects_strength_and_home_key() {
        let lut = Path::new("/h/.config/nimbus/vkbasalt/colour-boost.cube");
        let t = conf_text(&json!({"on": true}), lut);
        assert!(t.contains("effects = cas\n"), "{t}");
        assert!(t.contains("casSharpness = 0.40\n"));
        assert!(t.contains("toggleKey = Home\n"));
        assert!(t.contains("enableOnLaunch = True\n"));
        assert!(!t.contains("lutFile"));
        let t = conf_text(&json!({"on": true, "smaa": true, "vibrance": true, "strength": 85, "key": "End"}), lut);
        assert!(t.contains("effects = smaa:cas:lut\n"), "{t}");
        assert!(t.contains("casSharpness = 0.85\n"));
        assert!(t.contains("lutFile = /h/.config/nimbus/vkbasalt/colour-boost.cube\n"));
        assert!(t.contains("toggleKey = End\n"));
        // Out of range or the wrong kind: kept in range, or the default.
        let t = conf_text(&json!({"strength": 400, "key": "rm -rf"}), lut);
        assert!(t.contains("casSharpness = 1.00\n") && t.contains("toggleKey = Home\n"), "{t}");
    }

    #[test]
    fn clean_keeps_only_known_parts() {
        let c = clean(&json!({"on": true, "strength": "55", "smaa": 1, "bogus": 3, "key": "F12"}));
        assert_eq!(Value::Object(c), json!({"on": true, "strength": 55, "smaa": true, "key": "F12"}));
        assert!(clean(&json!("x")).is_empty());
        assert!(clean(&json!({"key": "Delete"})).is_empty());
        assert_eq!(settings(&json!({"strength": 10}))["on"], false);
    }

    #[test]
    fn the_lut_is_a_16_cube_that_keeps_grey_grey() {
        let t = lut_text();
        assert!(t.contains("LUT_3D_SIZE 16\n"));
        let rows: Vec<&str> = t.lines().filter(|l| l.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)).collect();
        assert_eq!(rows.len(), 16 * 16 * 16);
        assert_eq!(rows[0], "0.0000 0.0000 0.0000");
        assert_eq!(*rows.last().unwrap(), "1.0000 1.0000 1.0000");
        // Pure red gets no less red; a grey stays grey.
        let red: Vec<f64> = rows[15].split(' ').map(|x| x.parse().unwrap()).collect();
        assert!(red[0] >= 0.999 && red[1] == 0.0);
    }

    #[test]
    fn on_the_env_and_conf_are_there() {
        let d = tempfile::tempdir().unwrap();
        let conf = conf_path(d.path(), "gog:1207658924");
        assert!(conf.ends_with("vkbasalt/gog_1207658924.conf"));
        let opts = json!({"vkbasalt": {"on": true, "strength": 60, "vibrance": true}, "vkbasaltConf": conf});
        let env = launch_env(&opts);
        assert!(env.contains(&("ENABLE_VKBASALT".into(), "1".into())));
        assert!(env.contains(&("VKBASALT_CONFIG_FILE".into(), conf.to_string_lossy().to_string())));
        let t = std::fs::read_to_string(&conf).unwrap();
        assert!(t.contains("casSharpness = 0.60") && t.contains("effects = cas:lut"));
        assert!(d.path().join("vkbasalt/colour-boost.cube").exists());
        // Changed: written again, the old one kept as .bak.
        launch_env(&json!({"vkbasalt": {"on": true, "strength": 20}, "vkbasaltConf": conf}));
        assert!(std::fs::read_to_string(&conf).unwrap().contains("casSharpness = 0.20"));
        assert!(crate::settings_file::bak_path(&conf).exists());
    }

    /// CI's before/after picture (vkbasalt.yml, vkcube on lavapipe): the conf Hearth writes, everything turned up.
    #[test]
    fn ci_conf_for_the_lavapipe_picture() {
        let Some(p) = std::env::var_os("HEARTH_VKBASALT_CI_CONF") else { return };
        write_conf(Path::new(&p), &json!({"on": true, "strength": 100, "smaa": true, "vibrance": true})).unwrap();
    }

    #[test]
    fn steam_and_faugus_options() {
        let d = tempfile::tempdir().unwrap();
        let i = info(d.path());
        assert_eq!(i["steamOption"], "ENABLE_VKBASALT=1 %command%");
        assert_eq!(i["faugusArg"], "ENABLE_VKBASALT=1");
        assert!(i["steamOptionHearth"].as_str().unwrap().ends_with("steam.conf\" %command%"));
        assert!(!shared_conf_path(d.path()).exists(), "looking writes nothing");
        write_shared(d.path()).unwrap();
        assert!(std::fs::read_to_string(shared_conf_path(d.path())).unwrap().contains("effects = cas"));
    }
}
