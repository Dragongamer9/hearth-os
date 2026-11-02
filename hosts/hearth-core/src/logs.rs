//! Settings › System › Send logs: everything useful for a bug report in one text file, with personal details
//! taken out, saved in the home folder and on any USB drive or SD card that's plugged in, and (Send) sent to
//! Hearth OS's own bug-report service (the player counter's server), which answers with a short code: the
//! person shares the code or link with the developer, who reads the logs with it. Kept there 14 days.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;


fn tail(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let skip = lines.len().saturating_sub(n);
    let mut out = String::new();
    if skip > 0 { out.push_str(&format!("[… {skip} older lines left out …]\n")); }
    out.push_str(&lines[skip..].join("\n"));
    out
}

/// The newest `max` bytes of a section (whole lines).
fn cap(text: &str, max: usize) -> String {
    if text.len() <= max { return text.to_string(); }
    let mut from = text.len() - max;
    while !text.is_char_boundary(from) { from += 1; }
    let from = text[from..].find('\n').map(|i| from + i + 1).unwrap_or(from);
    format!("[… {} KB of older lines left out …]\n{}", from / 1024, &text[from..])
}

/// The journal without crash dumps' module lists and stack traces (thousands of lines each, which pushed Hearth's
/// own log out of a report): each crash keeps its "Process … dumped core" line and the first frames of its first
/// thread.
pub fn without_dump_details(journal: &str) -> String {
    let mut out = Vec::new();
    let mut frames = 0usize;
    let mut in_dump = false;
    let mut hidden = 0usize;
    for l in journal.lines() {
        let t = l.trim_start();
        let continuation = l.starts_with(' ') || l.starts_with('\t');
        if !continuation { in_dump = l.contains("dumped core"); frames = 0; if hidden > 0 { out.push(format!("    [… {hidden} lines of the crash dump left out …]")); hidden = 0; } out.push(l.to_string()); continue; }
        if in_dump {
            if t.starts_with('#') && frames < 6 { frames += 1; out.push(l.to_string()); continue; }
            if t.starts_with("Stack trace of thread") && frames == 0 { out.push(l.to_string()); continue; }
            if !t.is_empty() { hidden += 1; }
            continue;
        }
        out.push(l.to_string());
    }
    if hidden > 0 { out.push(format!("    [… {hidden} lines of the crash dump left out …]")); }
    out.join("\n")
}

/// The other desktops' logs in ~/.local/share/nimbus, written by an earlier Hearth's Plasma / GNOME / COSMIC (gone now:
/// Mantel is the only desktop); still in a report when an old one is there.
const DESKTOP_LOGS: [(&str, &str); 4] = [("plasma.log", "Hearth Plasma"), ("gnome.log", "Hearth GNOME"), ("cosmic.log", "Hearth COSMIC"),
                                         ("cosmic-companion.log", "Hearth COSMIC companion")];

/// Steam Game Mode (Decky plugin): whether Decky and its service are installed and running, Hearth Sync's state
/// (docs/HEARTH-SYNC.md) and whether the older Hearth plugin is still there, and the newest lines of the plugin's log (Decky writes each plugin's log to ~/homebrew/logs/<plugin>/, one
/// file per start; the plugin logs picture failures by reason and count, never paths) and the parts that turned
/// themselves off (~/homebrew/settings/<plugin>/errors.log). `services` is what systemctl says about the services.
pub fn decky_report(root: &Path, home: &Path, services: &str) -> String {
    let st = crate::steam_mode::decky_status(root, home);
    let mut out = format!("Decky installed: {} (via {}), Hearth Sync: {}, shipped with Hearth: {}, old Hearth plugin still present: {}, Steam remote debugging: {}\n{}\n",
        st["installed"], st["via"].as_str().unwrap_or("none"), st["plugin"], st["shipped"], st["oldPlugin"], st["cefDebugging"], services.trim_end());
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    for dir in ["homebrew/logs/Hearth Sync", "homebrew/logs/decky-hearth-sync", "homebrew/logs/Hearth", "homebrew/logs/decky-hearth"] {
        for e in std::fs::read_dir(home.join(dir)).into_iter().flatten().flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "log").unwrap_or(false) {
                let t = e.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
                if newest.as_ref().map(|(n, _)| t > *n).unwrap_or(true) { newest = Some((t, p)); }
            }
        }
    }
    match newest.and_then(|(_, p)| std::fs::read_to_string(&p).ok().map(|t| (p, t))) {
        Some((p, t)) => out.push_str(&format!("\n--- {} log ({}):\n{}\n", if p.to_string_lossy().contains("ync/") { "Hearth Sync" } else { "Old Hearth plugin" },
            p.file_name().unwrap_or_default().to_string_lossy(), tail(&t, 300))),
        None => out.push_str("\n--- Hearth Sync log: none found (the plugin hasn't run in Steam Game Mode yet)\n"),
    }
    for dir in ["homebrew/settings/Hearth", "homebrew/settings/decky-hearth"] {   // the old plugin's parts (Hearth Sync has none)
        if let Ok(t) = std::fs::read_to_string(home.join(dir).join("errors.log")) {
            if !t.trim().is_empty() { out.push_str(&format!("\n--- Parts that turned themselves off:\n{}\n", tail(&t, 50))); }
        }
    }
    out
}

fn sh(cmd: &str) -> String {
    crate::system::run_timeout("sh", &["-c", &format!("{cmd} 2>&1")], Duration::from_secs(15)).unwrap_or_else(|e| format!("({e})"))
}

/// The screen: gamescope's whole mode list (and the fastest rate at each size), the mode Hearth asked gamescope
/// for, and per kernel connector its status, modes and decoded EDID (the TV's own list: its fastest link, HDMI 2.1
/// FRL rate, 4K rates, VRR range). Serial numbers are left out.
const SCREEN_SH: &str = r##"
echo "--- gamescope: every mode of the external screen"
list=$(xprop -root GAMESCOPE_DISPLAY_MODE_LIST_EXTERNAL 2>/dev/null | sed -n 's/^[^"]*"\(.*\)".*/\1/p')
if [ -n "$list" ]; then
  echo "$list"
  echo "($(echo "$list" | wc -w) modes) fastest at each size: $(echo "$list" | tr ' ' '\n' | awk -F@ 'NF==2 { if (!($1 in m) || $2+0 > m[$1]+0) { if (!($1 in m)) o[++n]=$1; m[$1]=$2 } } END { for (i=1;i<=n;i++) printf "%s@%s ", o[i], m[o[i]] }')"
else
  echo "(none: not in Game Mode, or the built-in screen)"
fi
xprop -root GAMESCOPE_DISPLAY_EDID_PATH GAMESCOPE_DISPLAY_REFRESH_RATE_FEEDBACK 2>/dev/null
pid=$(xprop -root GAMESCOPE_PID 2>/dev/null | awk '{print $3}')
[ -n "$pid" ] && [ -r "/proc/$pid/cmdline" ] && echo "gamescope started as: $(tr '\0' ' ' < /proc/$pid/cmdline)"
echo "gamescope version: $(rpm -q gamescope 2>/dev/null || gamescope --version 2>&1 | head -1)"
conf="${XDG_CONFIG_HOME:-$HOME/.config}/nimbus"
grep -hE '^[[:space:]]*(export[[:space:]]+)?NIMBUS_GAMESCOPE_ARGS=' "$conf/session.conf" 2>/dev/null || echo "session.conf: no gamescope options (gamescope starts at the screen's own mode)"
[ -f "$conf/display-pending.json" ] && echo "a new mode is waiting to be kept: $(cat "$conf/display-pending.json")"
[ -n "$GAMESCOPE_MODE_SAVE_FILE" ] && echo "GAMESCOPE_MODE_SAVE_FILE=$GAMESCOPE_MODE_SAVE_FILE"
for c in /sys/class/drm/card*-*; do
  [ -r "$c/status" ] || continue
  st=$(cat "$c/status")
  echo; echo "--- ${c##*/}: $st, $(cat "$c/enabled" 2>/dev/null), $(cat "$c/dpms" 2>/dev/null)"
  [ "$st" = connected ] || continue
  echo "kernel's modes (sizes only): $(paste -sd' ' "$c/modes" 2>/dev/null)"
  [ -s "$c/edid" ] || { echo "(no EDID)"; continue; }
  d=""
  if command -v edid-decode >/dev/null 2>&1; then d=$(edid-decode "$c/edid" 2>&1)
  elif command -v di-edid-decode >/dev/null 2>&1; then d=$(di-edid-decode "$c/edid" 2>&1)
  fi
  if [ -n "$d" ]; then
    d=$(echo "$d" | grep -viE 'serial')
    echo "EDID summary ($(wc -c < "$c/edid") bytes):"
    echo "$d" | grep -iE 'Display Product Name|Manufacturer:|Model:|Made in|Monitor ranges|Display Range Limits|TMDS|Fixed Rate Link|FRL|DSC|VRR|ALLM|QMS|Max.*Pixel Clock|HDMI Forum|Vendor-Specific Data Block \(HDMI|Colorimetry|HDR Static|3840x2160|4096x2160|2560x1440|1920x1080 .*(1[0-9][0-9])\.' | sed 's/^[[:space:]]*/  /' | head -80
    echo "EDID in full:"
    echo "$d" | sed 's/^/  /' | head -500
  else
    echo "EDID ($(wc -c < "$c/edid") bytes; edid-decode isn't installed):"
    od -An -tx1 -v "$c/edid" | head -48
  fi
done
true
"##;

/// The graphics driver the screen link was made with: Nvidia's version, open or closed kernel modules, its display
/// options (hdmi_deepcolor, disable_hdmi_frl...), modprobe.d, the kernel command line, and the kernel's messages
/// about HDMI links (FRL link training) this boot.
const DRIVER_SH: &str = r##"
if [ -e /proc/driver/nvidia/version ]; then
  head -2 /proc/driver/nvidia/version
  command -v nvidia-smi >/dev/null 2>&1 && nvidia-smi --query-gpu=name,driver_version,vbios_version --format=csv,noheader
  echo "nvidia module: version $(modinfo -F version nvidia 2>/dev/null), license $(modinfo -F license nvidia 2>/dev/null) (Dual MIT/GPL = the open kernel modules)"
  for m in nvidia_modeset nvidia_drm; do
    [ -d "/sys/module/$m/parameters" ] && echo "$m: $(cd "/sys/module/$m/parameters" && grep -s . * | tr '\n' ' ')"
  done
  grep -sE 'EnableGpuFirmware|PreserveVideoMemory|TemporaryFilePath|EnableMSI|DynamicPowerManagement:' /proc/driver/nvidia/params | tr '\n' ' '; echo
else
  for d in /sys/class/drm/card[0-9]; do [ -e "$d/device/driver" ] && echo "${d##*/}: $(basename "$(readlink "$d/device/driver")")"; done
fi
echo "modprobe.d: $(grep -rhsE '^[[:space:]]*(options|blacklist|install)[[:space:]].*(nvidia|nouveau|nova|amdgpu|i915|xe)' /usr/lib/modprobe.d /etc/modprobe.d /run/modprobe.d | tr '\n' ';')"
echo "kernel command line: $(cat /proc/cmdline)"
echo "kernel: $(uname -r)"
echo "--- kernel messages about screens and HDMI links (this boot)"
journalctl -k -b --no-pager 2>/dev/null | grep -iE 'frl|hdmi|nvidia-modeset|nvkms|NVRM|link training|edid|drm.*(connector|mode)' | tail -60
true
"##;

/// Sleep: whether the device really sleeps (and reaches the processor's deepest state, s0i3 on AMD, where the fans
/// stop and the battery barely drops) or only turns the screen off; who handles the power button (logind, Hearth,
/// Handheld Daemon's powerbuttond, Steam's); who holds sleep off; what can wake it and what did; the last sleeps
/// with how long they lasted; and how the previous boot ended (asleep, or stuck waking up).
const SLEEP_SH: &str = r##"
echo "sleep states: $(cat /sys/power/state 2>/dev/null) | mem_sleep: $(cat /sys/power/mem_sleep 2>/dev/null) | disk: $(cat /sys/power/disk 2>/dev/null)"
printf 'suspend counters: '
for f in success fail failed_freeze failed_prepare failed_suspend failed_suspend_late failed_suspend_noirq failed_resume last_failed_dev last_failed_errno last_failed_step last_hw_sleep total_hw_sleep max_hw_sleep; do
  [ -r "/sys/power/suspend_stats/$f" ] && printf '%s=%s ' "$f" "$(cat "/sys/power/suspend_stats/$f" 2>/dev/null)"
done
echo
echo "(hw_sleep: microseconds in the deepest sleep, s0i3; a sleep that leaves total_hw_sleep unchanged never got there: the fans keep running and the battery drains)"
echo "last wake IRQ: $(cat /sys/power/pm_wakeup_irq 2>/dev/null || echo none)"
printf 'modules: '; for m in amd_pmc amdgpu asus_wmi asus_nb_wmi asus_armoury hid_asus hid_asus_ally; do [ -d "/sys/module/$m" ] && printf '%s ' "$m"; done; echo
for f in /sys/devices/platform/asus-nb-wmi/mcu_powersave /sys/class/firmware-attributes/asus-armoury/attributes/mcu_powersave/current_value; do
  [ -r "$f" ] && echo "ASUS MCU powersave ($f): $(cat "$f" 2>/dev/null)"
done
cat /sys/kernel/debug/amd_pmc/s0ix_stats 2>/dev/null || echo "amd_pmc s0ix_stats: needs root (total_hw_sleep above is the same time)"
echo "--- who handles the power button"
for p in HandlePowerKey HandlePowerKeyLongPress HandleSuspendKey HandleLidSwitch HandleLidSwitchExternalPower HandleLidSwitchDocked IdleAction; do
  printf '%s=%s ' "$p" "$(busctl get-property org.freedesktop.login1 /org/freedesktop/login1 org.freedesktop.login1.Manager "$p" 2>/dev/null | cut -d' ' -f2-)"
done
echo
grep -rsE '^[[:space:]]*(Handle|Idle|Inhibit|Suspend|Allow|HibernateDelay|MemorySleepMode)' /etc/systemd/logind.conf /etc/systemd/logind.conf.d /usr/lib/systemd/logind.conf.d /run/systemd/logind.conf.d /etc/systemd/sleep.conf /etc/systemd/sleep.conf.d /usr/lib/systemd/sleep.conf.d /run/systemd/sleep.conf.d
echo "power button helpers running: $(pgrep -af 'hearth-buttons|steamos-powerbuttond|powerbuttond' | grep -v pgrep | sed 's/^[0-9]* //' | paste -sd';' -)"
gp='not running'
for p in $(pgrep -x steam 2>/dev/null); do tr '\0' ' ' <"/proc/$p/cmdline" 2>/dev/null | grep -q -- '-gamepadui' && gp=running; done
echo "Steam's gamepad UI (Handheld Daemon's powerbuttond only acts while it runs): $gp"
journalctl -b --no-pager -u hhd 2>/dev/null | grep -iE 'PBTN|powerbutton|power button|inhibit|Executing (short|long) press|systemctl suspend' | tail -12
echo "when it last woke up (Hearth's sleep hook, /run/hearth-resumed, seconds of uptime): $(cat /run/hearth-resumed 2>/dev/null || echo 'not this boot') | uptime now: $(cut -d' ' -f1 /proc/uptime 2>/dev/null)"
echo "--- inhibitors (block = sleep refused while held; delay = held off a few seconds)"
systemd-inhibit --list --no-pager 2>&1
echo "--- what can wake the device"
awk 'NR>1 && $3 ~ /enabled/ {printf "%s(%s) ", $1, $4} END {print ""}' /proc/acpi/wakeup 2>/dev/null | sed 's/^/ACPI: /'
for d in /sys/bus/usb/devices/*; do
  [ "$(cat "$d/power/wakeup" 2>/dev/null)" = enabled ] && [ -r "$d/idVendor" ] && echo "USB ${d##*/}: $(cat "$d/idVendor"):$(cat "$d/idProduct") $(cat "$d/product" 2>/dev/null)"
done
for d in /sys/bus/serio/devices/*; do [ -r "$d/power/wakeup" ] && echo "serio ${d##*/} ($(cat "$d/description" 2>/dev/null)): wakeup $(cat "$d/power/wakeup")"; done
echo "wake sources that fired (count, name, device):"
for w in /sys/class/wakeup/wakeup*; do
  [ -r "$w/event_count" ] && echo "$(cat "$w/event_count") $(cat "$w/name") $(readlink -f "$w/device" 2>/dev/null | sed 's|^/sys/devices/||')"
done | sort -rn | awk '$1 > 0' | head -12
echo "--- fans and battery now"
for h in /sys/class/hwmon/hwmon*; do for f in "$h"/fan*_input; do [ -r "$f" ] && echo "$(cat "$h/name") ${f##*/}: $(cat "$f") RPM"; done; done
for b in /sys/class/power_supply/*; do
  [ "$(cat "$b/type" 2>/dev/null)" = Battery ] && echo "${b##*/}: $(cat "$b/capacity" 2>/dev/null)% $(cat "$b/status" 2>/dev/null), power_now=$(cat "$b/power_now" 2>/dev/null) energy_now=$(cat "$b/energy_now" 2>/dev/null)"
done
echo "--- sleeps this boot: when, and how long"
journalctl -b -k -o short-unix --no-pager 2>/dev/null | grep -E 'PM: suspend (entry|exit)' | while read -r t rest; do
  case "$rest" in
    *entry*) s=${t%.*}; echo "$(date -d "@$s" '+%F %T') went to sleep (${rest##*entry })";;
    *exit*) [ -n "$s" ] && echo "$(date -d "@${t%.*}" '+%F %T') woke up after $(( ${t%.*} - s )) s"; s="";;
  esac
done
echo "--- who asked for each sleep, power presses ignored after waking, kernel and logind around sleep (this boot, oldest first)"
echo "(hearth-sleep: Hearth Game Mode, Desktop Mode, hearth-buttons and Hearth's sleep hook; hhd: Handheld Daemon's powerbuttond)"
# One journalctl each: -k with -u matched only kernel lines from those units (none), so this part came out empty.
{
  journalctl -b --no-pager -o short-iso-precise -k 2>/dev/null | grep -iE 'PM: |s2idle|ACPI: PM|Low-level resume|amd_pmc|deepest|suspend|wakeup|freez|restarting tasks|hardware sleep|resume'
  journalctl -b --no-pager -o short-iso-precise -u systemd-logind -u systemd-suspend.service -u systemd-suspend-then-hibernate.service -u systemd-hibernate.service 2>/dev/null | grep -iE 'power key|suspend|sleep|lid|hibernat|wake|resume|inhibit'
  journalctl -b --no-pager -o short-iso-precise -t hearth-sleep 2>/dev/null
  journalctl -b --no-pager -o short-iso-precise -u hhd 2>/dev/null | grep -E 'Executing (short|long) press|systemctl suspend|woke up from sleep'
} | grep -v '^-- ' | sort -s -k1,1 | tail -100
echo "--- how the previous boot ended (asleep and never woke, or stuck waking up?)"
journalctl --list-boots --no-pager 2>/dev/null | tail -5
journalctl -b -1 -k --no-pager -o short-precise 2>/dev/null | grep -iE 'PM: |s2idle|ACPI: PM|amd_pmc|amdgpu|deepest|wakeup|resume' | tail -20
echo "(its last kernel lines)"
journalctl -b -1 -k --no-pager -o short-precise 2>/dev/null | tail -12
journalctl -b -1 --no-pager -o short-precise -u systemd-logind -u systemd-suspend.service 2>/dev/null | tail -10
true
"##;

/// Personal details out: the user name and home folder, sign-in tokens, e-mail addresses, Steam account ids,
/// Wi-Fi passwords.
pub fn redact(text: &str, user: &str, home: &str) -> String {
    let mut t = text.to_string();
    if home.len() > 1 { t = t.replace(home, "~"); }
    // The same home folder by its other name (/home/me and /var/home/me are one folder on the image-based
    // editions): for short user names the word rule below doesn't apply, and the name was left in the paths.
    if !user.is_empty() {
        if let Ok(r) = regex::Regex::new(&format!(r"(?:/var)?/home/{}\b", regex::escape(user))) { t = r.replace_all(&t, "~").to_string(); }
    }
    let rules: [(&str, &str); 6] = [
        (r#"(?i)("?(?:access_token|refresh_token|id_token|token|password|passwd|psk|secret|cookie|authorization|session_?id|code)"?\s*[:=]\s*"?)[^\s",&}]{4,}"#, "$1[removed]"),
        (r"(?i)(bearer\s+)[A-Za-z0-9._~+/=-]{8,}", "$1[removed]"),
        (r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}", "[email]"),
        (r"\b7656119\d{10}\b", "[steam-id]"),
        (r"(?i)([?&](?:code|token|key|sig|signature)=)[^&\s]+", "$1[removed]"),
        (r"(?i)(ssid[\s:=]+)\S+", "$1[wifi]"),
    ];
    for (re, to) in rules { if let Ok(r) = regex::Regex::new(re) { t = r.replace_all(&t, to).to_string(); } }
    if user.len() >= 3 {
        if let Ok(r) = regex::Regex::new(&format!(r"\b{}\b", regex::escape(user))) { t = r.replace_all(&t, "[user]").to_string(); }
    }
    t
}

/// The report: about the device, Hearth's logs, and what the system says about the parts that usually go wrong
/// (controller service, power, gamescope, memory).
pub fn report(version: &str) -> String {
    let data = crate::paths::data_dir();
    let run = PathBuf::from(std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into()));
    let mut info = crate::sysinfo::info(Path::new("/")); info["version"] = json!(version); info["host"] = json!("rust");
    let mut out = format!("Hearth logs · {} · Hearth {version}\n\n", chrono::Local::now().format("%Y-%m-%d %H:%M"));
    let mut section = |title: &str, body: String| { out.push_str(&format!("\n===== {title} =====\n{}\n", body.trim_end())); };
    section("About this device", serde_json::to_string_pretty(&info).unwrap_or_default());
    section("OS", std::fs::read_to_string("/etc/os-release").unwrap_or_default());
    // Hearth app updates (appchannel.rs): which Hearth runs (an update or the built-in one), its marks, the last check
    // and why Hearth last went back to the built-in version.
    section("Hearth app updates", crate::appchannel::report());
    section("System version (bootc)", sh("{ bootc status --format humanreadable || bootc status || rpm-ostree status; } 2>&1; ls /ostree/deploy/*/deploy 2>&1 | head -8"));
    // Hearth's log: the controller input lines first (the jumping-selection reports), then the newest lines.
    // Thousands of identical "library.downloads ok" lines from the old Electron app are left out.
    if let Ok(t) = std::fs::read_to_string(data.join("bridge.log")) {
        let spam = regex::Regex::new(r"^\d\d:\d\d:\d\d library\.(downloads|get) ok \d+ms$").unwrap();
        let lines: Vec<&str> = t.lines().filter(|l| !spam.is_match(l)).collect();
        let input: Vec<&str> = lines.iter().copied().filter(|l| l.contains(" input ") && !l.contains("art cache")).collect();
        let keep = |v: &[&str], n: usize| -> String { let skip = v.len().saturating_sub(n); let mut o = if skip > 0 { format!("[… {skip} older lines left out …]\n") } else { String::new() }; o.push_str(&v[skip..].join("\n")); o };
        if !input.is_empty() { section("Controller input (newest last)", keep(&input, 400)); }
        section("Hearth log", keep(&lines, 1500));
    }
    for (f, name) in [(data.join("session.log"), "Session log"), (run.join("nimbus-shell.log"), "Desktop shell log"), (data.join("pad.log"), "Desktop controller helper (hearth-pad)"),
                      (data.join("labwc.log"), "Desktop Mode compositor (labwc)")] {
        if let Ok(t) = std::fs::read_to_string(&f) { section(name, tail(&t, 400)); }
    }
    // Hearth Plasma, Hearth GNOME and Hearth COSMIC: each desktop's own log (the desktop's messages, and what
    // hearth-companion decided about the controller and why), and whether Quickshell is there (it runs the controller
    // in Plasma and COSMIC).
    {
        let mut body = format!("quickshell: {}\n", sh("command -v quickshell || echo 'not installed'").trim());
        for (f, name) in DESKTOP_LOGS {
            if let Ok(t) = std::fs::read_to_string(data.join(f)) { body.push_str(&format!("\n--- {name} ({f}):\n{}\n", tail(&t, 150))); }
        }
        section("Desktop Mode: Hearth Plasma, GNOME and COSMIC", body);
    }
    // Hearth Theater (docs/THEATER.md): its own log (start, the renderer and graphics it got, pages that loaded or
    // failed, the controller pointer), when it has been opened on this device.
    {
        let f = crate::theater::log_file();
        if crate::theater::find().is_some() || f.exists() {
            let body = match std::fs::read_to_string(&f) {
                Ok(t) => tail(&t, 400),
                Err(_) => "(no theater.log: Theater hasn't been opened since it started keeping one)".into(),
            };
            let chromium = if crate::theater::electron_in(Path::new("/")).is_some() { "there" } else { "missing (Theater can't open)" };
            section("Hearth Theater", format!("Hearth's Chromium (draws Theater): {chromium}\n\n{body}"));
        }
    }
    // Which engine draws Hearth's screens (docs/RENDERERS.md), and the Qt renderer's own log (Experimental).
    {
        let qt = std::fs::read_to_string(data.join("hearth-qml.log")).map(|t| tail(&t, 300)).unwrap_or_else(|_| "(no hearth-qml.log: the Qt renderer hasn't run)".into());
        section("Renderer", format!("{}\n\nQt renderer log (hearth-qml.log):\n{}", serde_json::to_string_pretty(&crate::renderer::status()).unwrap_or_default(), qt));
    }
    // Hearth Input (controller layouts for keyboard-and-mouse games): what the mapper did, and what it's doing now
    // (its controllers, whether it holds them, the layout running, its last error).
    {
        let log = std::fs::read_to_string(data.join("input.log")).map(|t| tail(&t, 300)).unwrap_or_else(|_| "(no input.log: Hearth Input hasn't run in Game Mode)".into());
        let status = sh(&format!("python3 '{}' status", crate::paths::desktop_dir().join("shell/hearth-input").display()));
        let layouts = std::fs::read_dir(crate::input_map::dir()).map(|d| d.filter_map(Result::ok).filter_map(|e| {
            let v: Value = std::fs::read(e.path()).ok().and_then(|b| serde_json::from_slice(&b).ok())?;
            Some(format!("{}: {}", e.file_name().to_string_lossy().trim_end_matches(".json"), v["layout"].as_str().unwrap_or("?")))
        }).collect::<Vec<_>>().join("\n")).unwrap_or_default();
        section("Hearth Input (controller layouts)", format!("Now: {}\n\nGames with a layout chosen:\n{}\n\nIts log:\n{}", status.trim(), if layouts.is_empty() { "(none)".into() } else { layouts }, log));
    }
    // The installer USB: the install screens' own log and the installer session's log (a report sent from there may
    // come from another account, so the installer user's files are read directly).
    if Path::new("/etc/hearth-live").exists() {
        for (f, name) in [("/var/home/liveuser/.local/share/nimbus/bridge.log", "Installer: Hearth log"), ("/var/home/liveuser/hearth-installer.log", "Installer: session log"),
                          ("/home/liveuser/.local/share/nimbus/bridge.log", "Installer: Hearth log"), ("/home/liveuser/hearth-installer.log", "Installer: session log")] {
            if f.starts_with("/home/") && Path::new("/var/home/liveuser").exists() { continue; }
            if let Ok(t) = std::fs::read_to_string(f) { section(name, tail(&t, 600)); }
        }
    }
    // Multi-seat: how it's set up and what the split-screen host did (labwc's version and warnings, its rules).
    if Path::new(crate::multiplay::SEATS).exists() && (data.join("seat-host.log").exists() || Path::new("/etc/hearth/seats.json").exists() || run.join("hearth-seat-host").exists()) {
        let host_log = std::fs::read_to_string(data.join("seat-host.log")).map(|t| tail(&t, 200)).unwrap_or_else(|_| "(none yet)".into());
        let rules = std::fs::read_to_string(run.join("hearth-seat-host/rc.xml")).unwrap_or_else(|_| "(not running)".into());
        section("Multi-seat", format!("{}\n\nSplit-screen host log:\n{}\n\nHost window rules:\n{}",
            cap(&sh(&format!("{} status", crate::multiplay::SEATS)), 8000), host_log, cap(&rules, 6000)));
    }
    // Couch Co-op and Multi-seat's "Player N, press A": the controllers each helper can see (and why it leaves the
    // others out), and the listener's own log (multiplay.log: what it saw at each try, which controller pressed).
    let pads_log = data.join("multiplay.log");
    if pads_log.exists() || data.join("coop").exists() || Path::new("/etc/hearth/seats.json").exists() {
        let log = std::fs::read_to_string(&pads_log).map(|t| tail(&t, 150)).unwrap_or_else(|_| "(none yet)".into());
        let mut seen = String::new();
        if Path::new(crate::multiplay::COOP).exists() {
            seen.push_str(&format!("Couch Co-op sees: {}\n\n", cap(&sh(&format!("python3 {} pads", crate::multiplay::COOP)), 6000).trim()));
        }
        if Path::new(crate::multiplay::SEATS).exists() {
            seen.push_str(&format!("Multi-seat sees: {}\n\n", cap(&sh(&format!("python3 {} pads", crate::multiplay::SEATS)), 6000).trim()));
        }
        section("Player controllers (press A to join)", format!("{seen}Listener log:\n{log}"));
    }
    // Couch Co-op: the session (or how the last one failed), the split screen's labwc log and each player's log (their
    // nested gamescope and game: a player whose window closed at once says why here).
    if Path::new(crate::multiplay::COOP).exists() && (data.join("coop").exists() || run.join("hearth-coop").exists() || run.join("hearth-coop-last.json").exists()) {
        let labwc = std::fs::read_to_string(run.join("hearth-coop/labwc.log")).map(|t| tail(&t, 80)).unwrap_or_else(|_| "(none)".into());
        let last = std::fs::read_to_string(run.join("hearth-coop-last.json")).unwrap_or_else(|_| "(none)".into());
        let mut players = String::new();
        for n in 1..=4 {
            let f = data.join("coop/players").join(n.to_string()).join("last-run.log");
            if let Ok(t) = std::fs::read_to_string(&f) { players.push_str(&format!("--- player {n}\n{}\n", tail(&t, 80))); }
        }
        section("Couch Co-op", format!("Now: {}\n\nLast failed start: {}\n\nSplit screen (labwc) log:\n{}\n\nPlayers' logs (gamescope and the game):\n{}",
            cap(&sh(&format!("python3 {} status", crate::multiplay::COOP)), 4000).trim(), cap(&last, 6000), labwc,
            if players.is_empty() { "(none yet)".to_string() } else { cap(&players, 24000) }));
    }
    // What's busy: the processor (top processes) and, on Nvidia, the graphics card (for "it feels sluggish").
    section("Processor and graphics load", sh("top -b -n 1 -w 160 2>/dev/null | head -24 || ps -eo pcpu=,comm= --sort=-pcpu | head -15; \
        [ -e /proc/driver/nvidia/version ] && command -v nvidia-smi >/dev/null && { echo; nvidia-smi --query-gpu=name,utilization.gpu,utilization.memory,memory.used,memory.total,clocks.gr,clocks.max.gr,pstate,power.draw --format=csv; echo; nvidia-smi --query-compute-apps=pid,process_name,used_memory --format=csv; } || true"));
    section("Memory", sh("free -m; echo; ps -eo rss=,comm= --sort=-rss | head -25 | awk '{printf \"%7.0f MB  %s\\n\", $1/1024, $2}'"));
    section("Games and processes", sh("ps -eo pid,ppid,stat,etime,comm --sort=start_time | tail -60"));
    // Steam: the folders Hearth looks in, the one it uses, its account file's counts (no names or ids), the games'
    // records per library, the owned-games list and Steam's processes (steam::report).
    let steam_on = std::fs::read(data.join("library.json")).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .map(|v| v["sources"]["steam"] != Value::Bool(false)).unwrap_or(true);
    section("Steam", crate::steam::report(&crate::paths::home(), steam_on));
    section("Steam Game Mode (Decky plugin)", decky_report(Path::new("/"), &crate::paths::home(),
        &sh("for u in \"hearth-decky@$(id -un).service\" hearth-decky.service plugin_loader.service; do printf '%s: ' \"$u\"; systemctl is-active \"$u\" 2>/dev/null || true; done")));
    // Steam's own logs (native and Flathub Steam): game launches and what stopped them ("games won't start in
    // Steam Game Mode"), only the launch, compatibility and error lines, no account lines.
    section("Steam's own logs (game launches)", sh("for d in \"$HOME/.local/share/Steam/logs\" \"$HOME/.var/app/com.valvesoftware.Steam/.local/share/Steam/logs\"; do \
        for f in console_log.txt console-linux.txt compat_log.txt; do [ -s \"$d/$f\" ] || continue; echo \"--- $d/$f\" | sed \"s|$HOME|~|\"; \
        grep -aiE 'gameaction|launch|appid|compat|pressure|xwayland|display|exited|error|fail|proton|reaper' \"$d/$f\" | grep -aivE 'account|login|password|token|steamid|persona' | tail -120; done; done"));
    section("Storage (drives and space)", crate::diskspace::report());
    section("Power (hearth-power)", sh(&format!("python3 '{}' status", crate::paths::desktop_dir().join("shell/hearth-power").display())));
    // "Pressing power turns the screen off but the fans keep running": whether it really slept, how deeply, who
    // handles the power button and who holds sleep off (SLEEP_SH).
    section("Sleep", sh(&format!("{{ {SLEEP_SH} }}")));
    // Flatpak and Bazaar (Hearth OS's app store; Hearth's own store is archived): what Settings › Apps sees (can your
    // own Flatpak folder be used, is Bazaar installed, hearth-flatpak-setup's last run), the Flathub remotes (yours and
    // the system's) and who owns the folders flatpak writes to (a folder owned by root or an old account makes
    // `flatpak --user` fail with "permission denied").
    section("App store (Bazaar and Flatpak)", sh(&format!("{{ python3 '{}' status; echo; echo '--- flatpak'; flatpak --version; \
        echo '--- remotes (yours)'; flatpak remotes --user --show-details; echo '--- remotes (system)'; flatpak remotes --system --show-details; \
        echo '--- who owns the folders'; for d in \"$HOME\" \"$HOME/.local\" \"$HOME/.local/share\" \"$HOME/.local/share/flatpak\" \"$HOME/.local/share/flatpak/repo\" \"$HOME/.local/share/flatpak/appstream\" \"$HOME/.cache\" \"$HOME/.config\"; do stat -c '%U (%u):%G %a %F %n' \"$d\"; done; \
        echo '--- not yours inside ~/.local/share/flatpak (first 20)'; find \"$HOME/.local/share/flatpak\" -maxdepth 4 ! -user \"$(id -u)\" 2>/dev/null | head -20; \
        echo '--- Bazaar'; flatpak info io.github.kolunmi.Bazaar 2>&1 | head -12; flatpak override --system --show io.github.kolunmi.Bazaar 2>&1; \
        echo '--- installing Bazaar (hearth-flatpak-setup)'; journalctl -b --no-pager -n 25 -u hearth-flatpak-setup; \
        echo '--- the boot-time repair'; journalctl -b --no-pager -n 15 -u hearth-home-repair; }} 2>&1",
        crate::paths::desktop_dir().join("shell/hearth-flatpak").display())));
    // Hearth's Debian box (.deb files through Distrobox): whether it's made, what's installed through Hearth.
    section("Debian box (.deb packages)", sh(&format!("{{ python3 '{0}' status --quick; python3 '{0}' list; distrobox --version; podman --version; grep \"^$(id -un):\" /etc/subuid /etc/subgid; }} 2>&1",
        crate::paths::desktop_dir().join("shell/hearth-debbox").display())));
    // Keeping Quick Resume games through turning off and restarting (hibernation): whether it's available and why
    // not (power.hibernateStatus, with what it looked at), Nvidia's sleep helpers, and the last hibernations.
    section("Quick Resume through turning off (hibernation)", format!("{}\n\n{}",
        serde_json::to_string_pretty(&crate::power::hibernate_status()).unwrap_or_default(),
        sh("cat /proc/swaps; echo; tr ' ' '\\n' </proc/cmdline | grep -E '^(resume|resume_offset|nvidia)'; echo; ls /run/systemd/sleep.conf.d 2>/dev/null; \
            systemctl is-enabled nvidia-suspend.service nvidia-hibernate.service nvidia-resume.service 2>&1 | paste -sd' '; \
            journalctl -b -1 --no-pager -n 30 -u systemd-hibernate.service -u nvidia-hibernate.service 2>/dev/null | tail -30; \
            journalctl -b --no-pager -n 40 -u systemd-hibernate.service -u nvidia-hibernate.service -u nvidia-resume.service -u systemd-hibernate-resume.service 2>/dev/null | tail -40")));
    section("Controllers (InputPlumber)", sh("busctl --system --list tree org.shadowblip.InputPlumber | head -40; for d in $(busctl --system --list tree org.shadowblip.InputPlumber | grep -o '/org/shadowblip/InputPlumber/CompositeDevice[0-9]*' | sort -u); do echo \"$d\"; busctl --system get-property org.shadowblip.InputPlumber $d org.shadowblip.Input.CompositeDevice Name InterceptMode ProfileName 2>&1 | sed 's/^/  /'; done"));
    section("Controllers and power (Handheld Daemon)", crate::hhd::report());
    // Quick Access over games (Handheld Daemon's overlay app, hearth-overlay): its own log (the fonts it was given,
    // which page it loaded and whether that page's text shows), and fontconfig as the system sees it. Quick Access
    // came up with no text at all on an Nvidia PC (0.6.5): Chromium's own fontconfig found no font.
    if Path::new("/usr/share/nimbus/desktop/shell/hearth-overlay").exists() || run.join("hearth-overlay.log").exists() {
        let log = std::fs::read_to_string(run.join("hearth-overlay.log")).map(|t| tail(&t, 120)).unwrap_or_else(|_| "(no hearth-overlay.log: Handheld Daemon hasn't started Quick Access in Game Mode this boot)".into());
        let fc = sh("{ rpm -q fontconfig 2>/dev/null || fc-match --version; echo \"sans-serif: $(fc-match sans-serif)\"; echo \"Sans: $(fc-match Sans)\"; echo \"fonts: $(fc-list | wc -l)\"; \
            for f in 48-guessfamily 48-spacing 49-sansserif; do [ -e /etc/fonts/conf.d/$f.conf ] && echo \"conf.d/$f.conf -> $(readlink -f /etc/fonts/conf.d/$f.conf) ($(rpm -qf /etc/fonts/conf.d/$f.conf 2>/dev/null || dpkg -S /etc/fonts/conf.d/$f.conf 2>/dev/null | cut -d: -f1))\"; done; \
            echo \"overlay's fonts.conf: $(ls -l \"${XDG_RUNTIME_DIR:-/tmp}/hearth-overlay-fonts/fonts.conf\" 2>&1)\"; echo \"caches: $(ls -d ~/.cache/hearth-overlay/* ~/.cache/fontconfig 2>&1 | tr '\\n' ' ')\"; } 2>&1");
        section("Quick Access over games (overlay app)", format!("{}\n\nFonts (the system's fontconfig):\n{}", log, fc.trim_end()));
    }
    // Long enough for an external screen's whole mode list (it was cut at 300 characters, in the middle of it).
    section("gamescope", sh("xprop -root | grep -E '^GAMESCOPE|^GAMESCOPECTRL' | cut -c1-4000"));
    // "My TV does 144 Hz but Game Mode shows 60": what gamescope, the kernel and the TV itself (its EDID) say, and
    // which driver and options the link was made with. Enough to tell a TV that only offered HDMI 2.0 rates from a
    // driver that didn't make the HDMI 2.1 fast link (FRL), or a mode Hearth chose.
    section("Screen: modes and what it says it can do (EDID)", sh(&format!("{{ {SCREEN_SH} }}")));
    section("Graphics driver and the screen link", sh(&format!("{{ {DRIVER_SH} }}")));
    section("Bluetooth", sh("bluetoothctl devices; bluetoothctl show | grep -E 'Powered|Discoverable|Pairable'"));
    section("System journal: controllers, power, display (this boot)", cap(&sh("journalctl -b --no-pager -n 400 -u inputplumber -u hhd -u steamos-manager -u powerstation -u bluetooth -u tuned -u tuned-ppd"), 80_000));
    section("System journal: errors (this boot)", cap(&without_dump_details(&sh("journalctl -b --no-pager -p err -n 400")), 80_000));
    section("Your session's journal (this boot)", cap(&without_dump_details(&sh("journalctl --user -b --no-pager -n 800")), 150_000));
    let user = std::env::var("USER").unwrap_or_default();
    redact(&out, &user, &crate::paths::home().to_string_lossy())
}

/// Saves the report in the home folder: Hearth-logs-<date>.txt
pub fn save(version: &str) -> Result<(PathBuf, String), String> {
    let text = report(version);
    let file = crate::paths::home().join(format!("Hearth-logs-{}.txt", chrono::Local::now().format("%Y-%m-%d-%H-%M")));
    std::fs::write(&file, &text).map_err(|e| format!("Couldn't save the logs: {e}"))?;
    Ok((file, text))
}

/// Saves it in the home folder, and a copy on each USB drive or SD card plugged in: {path, copies: [...]}.
pub fn send(version: &str) -> Result<Value, String> {
    let (file, _) = save(version)?;
    let name = file.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    let user = std::env::var("USER").unwrap_or_default();
    let mut copies = vec![];
    for base in [PathBuf::from("/run/media").join(&user), PathBuf::from("/media").join(&user)] {
        for d in std::fs::read_dir(&base).into_iter().flatten().flatten() {
            let to = d.path().join(&name);
            if std::fs::copy(&file, &to).is_ok() { copies.push(to.to_string_lossy().to_string()); }
        }
    }
    Ok(json!({"path": file.to_string_lossy(), "copies": copies}))
}

/// A report made to fit in `max` bytes: the biggest section other than Hearth's own (device, Hearth log, session
/// log) loses its oldest lines first, again and again; Hearth's log only as a last resort. (It used to cut from
/// the start of Hearth's log, and a journal full of crash dumps left no Hearth log at all.)
pub fn fit(text: &str, max: usize) -> String {
    if text.len() <= max { return text.to_string(); }
    let mut parts: Vec<(String, String)> = Vec::new();   // (header line, body)
    let mut head = String::new();
    for (i, chunk) in text.split("\n===== ").enumerate() {
        if i == 0 { head = chunk.to_string(); continue; }
        let (title, body) = chunk.split_once('\n').unwrap_or((chunk, ""));
        parts.push((title.to_string(), body.to_string()));
    }
    let own = |t: &str| t.starts_with("About this device") || t.starts_with("Hearth log") || t.starts_with("Session log") || t.starts_with("OS ");
    let total = |p: &[(String, String)]| head.len() + p.iter().map(|(t, b)| t.len() + b.len() + 8).sum::<usize>();
    for round in 0..40 {
        let size = total(&parts);
        if size <= max { break; }
        let over = size - max;
        let pick = parts.iter().enumerate().filter(|(_, (t, b))| (round >= 20 || !own(t)) && b.len() > 2100).max_by_key(|(_, (_, b))| b.len()).map(|(i, _)| i);
        let Some(i) = pick else { break };
        let body = &parts[i].1;
        let keep = body.len().saturating_sub(over.max(body.len() / 4)).max(2000.min(body.len()));
        parts[i].1 = cap(body, keep);
    }
    let mut out = head;
    for (t, b) in parts { out.push_str("\n===== "); out.push_str(&t); out.push('\n'); out.push_str(&b); }
    out
}

/// Hearth OS's bug-report service (the same server as the player counter).
pub const SERVICE: &str = "https://hearth-counter.game305-cl.workers.dev";
const MAX_SEND: usize = 600_000;

/// Saves (as above) and sends: {path, copies, code, url}. Too-long logs lose your session's journal first, then
/// their oldest lines.
/// A note for an HTTP header: printable ASCII kept, everything else percent-encoded (UTF-8), at most 1000 bytes in.
pub fn header_note(note: &str) -> String {
    let mut n = note.trim().to_string();
    if n.len() > 1000 { let mut c = 1000; while !n.is_char_boundary(c) { c -= 1; } n.truncate(c); }
    let mut out = String::new();
    for b in n.bytes() {
        if b.is_ascii_alphanumeric() || b" -_.~!*'()".contains(&b) { out.push(b as char); } else { out.push_str(&format!("%{b:02X}")); }
    }
    out
}

pub fn send_to_developer(version: &str, kind: &str) -> Result<Value, String> { send_to_developer_note(version, kind, "") }
/// The same, with the person's note ("What happened?"): it goes with the logs as X-Hearth-Note.
pub fn send_to_developer_note(version: &str, kind: &str, note: &str) -> Result<Value, String> {
    crate::watchdog::long_task();   // asked for from the page: up to a minute on purpose
    send_report(version, kind, note)
}
/// The same, from the fail-safe's own thread (which mustn't mark the stuck request as slow on purpose).
pub fn send_report(version: &str, kind: &str, note: &str) -> Result<Value, String> {
    let mut out = send(version)?;
    let path = out["path"].as_str().unwrap_or("").to_string();
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let text = fit(&text, MAX_SEND);
    let tmp = std::env::temp_dir().join(format!("hearth-send-{}.txt", std::process::id()));
    std::fs::write(&tmp, &text).map_err(|e| e.to_string())?;
    let note_h = format!("X-Hearth-Note: {}", header_note(note));
    let ver_h = format!("X-Hearth-Version: {version}");
    let kind_h = format!("X-Hearth-Kind: {kind}");
    let data = format!("@{}", tmp.to_string_lossy());
    let url = format!("{SERVICE}/v1/logs");
    let mut args: Vec<&str> = vec!["-sS", "-m", "60", "-X", "POST", "-H", "Content-Type: text/plain; charset=utf-8", "-H", &ver_h, "-H", &kind_h];
    if !note.trim().is_empty() { args.push("-H"); args.push(&note_h); }
    args.extend(["--data-binary", &data, &url]);
    let r = crate::system::run_timeout("curl", &args, Duration::from_secs(70));
    let _ = std::fs::remove_file(&tmp);
    let reply: Value = r.ok().and_then(|t| serde_json::from_str(t.trim()).ok()).unwrap_or(Value::Null);
    let code = reply["code"].as_str().ok_or_else(|| format!("Saved to {path}, but sending didn't work{}. Check the internet connection and try again.",
        reply["error"].as_str().map(|e| format!(" ({e})")).unwrap_or_default()))?;
    out["code"] = json!(code);
    out["url"] = json!(format!("{SERVICE}/v1/logs/{code}"));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn notes_fit_in_a_header() {
        assert_eq!(header_note("  Game froze  "), "Game froze");
        assert_eq!(header_note("a\nb"), "a%0Ab");
        assert_eq!(header_note("é"), "%C3%A9");
        assert_eq!(header_note("x: y"), "x%3A y");
        assert!(header_note(&"é".repeat(800)).len() <= 3000);
    }
    #[test]
    fn takes_out_personal_details() {
        let t = redact("home /home/sam/x refresh_token=abcdef123456 mail sam.smith@gmail.com id 76561198012345678 Bearer abcdefghijkl sam logged in, ssid: CasaWifi",
                       "sam", "/home/sam");
        assert!(!t.contains("/home/sam"), "{t}");
        assert!(!t.contains("abcdef123456") && !t.contains("gmail") && !t.contains("76561198012345678") && !t.contains("abcdefghijkl") && !t.contains("CasaWifi"), "{t}");
        assert!(t.contains("[user] logged in"), "{t}");
        assert_eq!(tail("a\nb\nc", 2), "[… 1 older lines left out …]\nb\nc");
        let t = redact("steam at /home/jo/.steam and /var/home/jo/Games, not /home/joan", "jo", "/var/home/jo");
        assert_eq!(t, "steam at ~/.steam and ~/Games, not /home/joan");
    }
    #[test]
    fn screen_sections_keep_their_output() {
        // sh() gives only an error line when the script's last command fails (grep finding nothing): both end in true.
        for s in [SCREEN_SH, DRIVER_SH, SLEEP_SH] { assert!(s.trim_end().ends_with("\ntrue"), "{s}"); }
        // The Sleep section: the states, the deep-sleep counters, who handles the power button, the inhibitors, the
        // wake sources, and the last sleeps with their lengths.
        for k in ["/sys/power/mem_sleep", "total_hw_sleep", "HandlePowerKey", "systemd-inhibit --list", "/proc/acpi/wakeup", "PM: suspend (entry|exit)", "journalctl -b -1 -k", "amd_pmc", "-t hearth-sleep", "/run/hearth-resumed"] {
            assert!(SLEEP_SH.contains(k), "{k}");
        }
        assert!(SCREEN_SH.contains("GAMESCOPE_DISPLAY_MODE_LIST_EXTERNAL") && SCREEN_SH.contains("edid-decode"));
        assert!(DRIVER_SH.contains("/proc/cmdline") && DRIVER_SH.contains("modinfo -F version nvidia") && DRIVER_SH.contains("driver_version"));
    }
    #[test]
    fn desktop_logs_are_the_ones_the_session_writes() {
        let session = include_str!("../../linux/session/nimbus-session");
        // Mantel is the only desktop: the session doesn't write these any more (a report still shows old ones).
        for (f, _) in DESKTOP_LOGS.iter().take(3) { assert!(!session.contains(&format!("/{f}\"")), "{f}"); }
        assert!(include_str!("../../linux/desktop/cosmic/hearth-cosmic").contains("cosmic-companion.log"));
    }
    #[test]
    fn decky_plugin_log_is_the_newest_and_says_what_is_installed() {
        let t = std::env::temp_dir().join(format!("hearth-logs-decky-{}", std::process::id()));
        let (root, home) = (t.join("root"), t.join("home"));
        let _ = std::fs::remove_dir_all(&t);
        let none = decky_report(&root, &home, "hearth-decky.service: inactive");
        assert!(none.contains("Decky installed: false") && none.contains("none found") && none.contains("hearth-decky.service: inactive"), "{none}");
        std::fs::create_dir_all(root.join("etc/systemd/system")).unwrap();
        std::fs::write(root.join(crate::steam_mode::HEARTH_UNIT), "").unwrap();
        let logs = home.join("homebrew/logs/Hearth");
        std::fs::create_dir_all(&logs).unwrap();
        std::fs::write(logs.join("2026-10-01.log"), "old start\n").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(logs.join("2026-11-01.log"), "Hearth plugin ready\nart: not shown (page art: not a picture), 1 so far\n").unwrap();
        std::fs::create_dir_all(home.join("homebrew/settings/decky-hearth")).unwrap();
        std::fs::write(home.join("homebrew/settings/decky-hearth/errors.log"), "tkMenu: not found\n").unwrap();
        let r = decky_report(&root, &home, "");
        assert!(r.contains("Decky installed: true (via hearth)") && r.contains("Hearth Sync: false") && r.contains("old Hearth plugin still present: false"), "{r}");
        assert!(r.contains("art: not shown") && !r.contains("old start"), "the newest start's log: {r}");
        assert!(r.contains("tkMenu: not found"), "{r}");
        std::thread::sleep(std::time::Duration::from_millis(20));
        let sync = home.join("homebrew/logs/Hearth Sync");
        std::fs::create_dir_all(&sync).unwrap();
        std::fs::write(sync.join("2026-11-02.log"), "sync: 3 in Steam; added 3\n").unwrap();
        let r = decky_report(&root, &home, "");
        assert!(r.contains("--- Hearth Sync log") && r.contains("sync: 3 in Steam"), "{r}");
        let _ = std::fs::remove_dir_all(&t);
    }
    #[test]
    fn crash_dumps_shortened() {
        let j = "Oct 21 x systemd-coredump[1]: Process 5 (mangoapp) of user 1000 dumped core.\n    \n    Module libz.so.1 from rpm zlib\n    Module libx.so\n    Stack trace of thread 5:\n    #0 a\n    #1 b\nOct 21 y kernel: something else";
        let o = without_dump_details(j);
        assert!(o.contains("dumped core") && o.contains("#0 a") && o.contains("something else"));
        assert!(!o.contains("Module libz") && o.contains("2 lines of the crash dump left out"));
    }
    #[test]
    fn fits_without_losing_hearths_log() {
        let big = "x\n".repeat(400_000);
        let text = format!("Hearth logs\n\n===== About this device =====\n{{}}\n\n===== Hearth log =====\nnewest hearth line\n\n===== System journal: errors (this boot) =====\n{big}\n===== Your session's journal (this boot) =====\n{big}");
        let out = fit(&text, 100_000);
        assert!(out.len() <= 100_000 + 200, "{}", out.len());
        assert!(out.contains("newest hearth line") && out.contains("===== System journal: errors") && out.contains("===== Your session's journal"));
    }
}
