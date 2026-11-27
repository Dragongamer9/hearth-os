//! "Warn when hot" (board 03, Look): MangoHud has no temperature warning, so Hearth watches the temperature itself.
//! `hearth-overlay --watch` (the user service hearth-overlay-hot.service, made and turned on only when you pick a
//! temperature in Warn when hot, off again with Off; never part of starting the PC or Game Mode) reads the graphics
//! card's temperature every 2 s (the kernel's hwmon: amdgpu, i915, xe, nouveau; nvidia-smi for Nvidia's own driver)
//! and, from the temperature you picked, writes a warning line to $XDG_RUNTIME_DIR/hearth-overlay-hot, which MangoHud
//! shows in the game (MangoHud's own `exec=` line, which Hearth Overlay adds to its file when you turn it on: MangoHud
//! reads the line twice a second); it's emptied again once the card is 3 °C cooler. The first time it gets hot it
//! also says so in a notification (for games where the overlay is hidden). It never writes your settings.
//! $HEARTH_OVERLAY_TEMP: a file with the temperature in millidegrees (CI's pictures).

use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;

pub const UNIT: &str = "hearth-overlay-hot.service";

pub fn warn_file() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("hearth-overlay-hot")
}

/// MangoHud's line that shows the warning (Hearth Overlay's own; taken out with Off).
pub fn exec_line() -> String {
    format!("cat {}", warn_file().display())
}

/// The graphics card's temperature in °C, from the kernel's hwmon (the hottest card), else nvidia-smi.
pub fn gpu_temp() -> Option<f32> {
    if let Ok(f) = std::env::var("HEARTH_OVERLAY_TEMP") {
        return std::fs::read_to_string(f).ok()?.trim().parse::<f32>().ok().map(|m| m / 1000.0);
    }
    let mut best: Option<f32> = None;
    if let Ok(rd) = std::fs::read_dir("/sys/class/hwmon") {
        for e in rd.flatten() {
            let name = std::fs::read_to_string(e.path().join("name")).unwrap_or_default();
            if !matches!(name.trim(), "amdgpu" | "i915" | "xe" | "nouveau" | "radeon") {
                continue;
            }
            // the edge (or the first) sensor; the junction runs hotter by design
            let v = std::fs::read_to_string(e.path().join("temp1_input")).ok().and_then(|s| s.trim().parse::<f32>().ok()).map(|m| m / 1000.0);
            if let Some(v) = v {
                best = Some(best.map_or(v, |b: f32| b.max(v)));
            }
        }
    }
    if best.is_none() && std::path::Path::new("/proc/driver/nvidia").exists() {
        let o = hearth_ui::run::blocking(&["nvidia-smi".to_string(), "--query-gpu=temperature.gpu".into(), "--format=csv,noheader,nounits".into()]);
        if o.ok {
            best = o.out.lines().filter_map(|l| l.trim().parse::<f32>().ok()).fold(None, |b, v| Some(b.map_or(v, |b: f32| b.max(v))));
        }
    }
    best
}

/// What the line says at `temp` against the limit, with the 3 °C step back (`was`: it said hot last time).
pub fn words(temp: Option<f32>, limit: f32, was: bool) -> (bool, String) {
    match temp {
        Some(t) if t >= limit || (was && t > limit - 3.0) => (true, format!("⚠ GPU {} °C · hot", t.round())),
        _ => (false, String::new()),
    }
}

fn write_atomic(f: &PathBuf, s: &str) {
    let tmp = f.with_extension("part");
    if std::fs::write(&tmp, s).is_ok() {
        let _ = std::fs::rename(&tmp, f);
    }
}

/// The service: until warnings are turned off (then it ends, and the line is emptied).
pub fn watch() -> i32 {
    let f = warn_file();
    let mut hot = false;
    let mut told = false;
    loop {
        let set: Value = hearth_oss_kit_a::read_settings("overlay");
        if !set.get("warnOn").and_then(Value::as_bool).unwrap_or(false) {
            let _ = std::fs::remove_file(&f);
            return 0;
        }
        let limit = set.get("warnGpu").and_then(Value::as_u64).unwrap_or(83) as f32;
        let t = gpu_temp();
        let (now_hot, line) = words(t, limit, hot);
        if now_hot != hot || !f.exists() {
            write_atomic(&f, &format!("{line}\n"));
        } else if now_hot {
            write_atomic(&f, &format!("{line}\n"));
        }
        if now_hot && !told {
            told = true;
            hearth_ui::run::detached(&["notify-send", "-a", "Hearth Overlay", "-u", "critical", "Your graphics card is hot", &format!("{} °C (you asked to be warned from {} °C). Let it cool, or turn the frame-rate limit down.", t.unwrap_or(0.0).round(), limit)]);
        }
        if !now_hot && t.map(|t| t < limit - 8.0).unwrap_or(true) {
            told = false;
        }
        hot = now_hot;
        std::thread::sleep(Duration::from_secs(2));
    }
}

/// The service's file (in your own systemd folder), started through hearth-part so the Store's copy runs when it has one.
pub fn unit_text(me: &str) -> String {
    let start = if std::path::Path::new("/usr/libexec/hearth/hearth-part").exists() { "/usr/libexec/hearth/hearth-part run hearth-overlay /usr/bin/hearth-overlay --watch".to_string() } else { format!("{me} --watch") };
    format!(
        "# Hearth Overlay: warn when the graphics card is hot (made by Hearth Overlay when you turned it on; Off removes it)\n[Unit]\nDescription=Hearth Overlay: warn when the GPU is hot\nStartLimitIntervalSec=300\nStartLimitBurst=3\n\n[Service]\nExecStart={start}\nRestart=on-failure\nRestartSec=15\nNice=10\n\n[Install]\nWantedBy=default.target\n"
    )
}

/// On: the service made and started (from your next sign-in too). Off: stopped and its file removed.
pub fn set_on(on: bool) -> Result<(), String> {
    let base = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| hearth_oss_kit_a::home().join(".config"));
    let unit = base.join("systemd/user").join(UNIT);
    if on {
        let me = std::env::current_exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|_| "hearth-overlay".into());
        std::fs::create_dir_all(unit.parent().unwrap_or(&base)).map_err(|e| e.to_string())?;
        write_atomic(&unit, &unit_text(&me));
        let _ = hearth_ui::run::blocking(&["systemctl".to_string(), "--user".into(), "daemon-reload".into()]);
        let o = hearth_ui::run::blocking(&["systemctl".to_string(), "--user".into(), "enable".into(), "--now".into(), UNIT.into()]);
        if !o.ok {
            return Err(o.why());
        }
    } else {
        let _ = hearth_ui::run::blocking(&["systemctl".to_string(), "--user".into(), "disable".into(), "--now".into(), UNIT.into()]);
        let _ = std::fs::remove_file(&unit);
        let _ = std::fs::remove_file(warn_file());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hot_and_cool_again() {
        assert_eq!(words(Some(84.2), 83.0, false), (true, "⚠ GPU 84 °C · hot".into()));
        assert_eq!(words(Some(81.0), 83.0, true).0, true, "3 °C back before it stops");
        assert_eq!(words(Some(79.5), 83.0, true).0, false);
        assert_eq!(words(None, 83.0, true).0, false);
        assert!(unit_text("/x/hearth-overlay").contains("--watch"));
    }
}
