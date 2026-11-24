//! System › Notifications, Memory, Storage, Drives & folders, Your desktop, Boot splash and Remote desktop (Mantel's
//! SetNotifications.qml, SetMemory.qml + MemorySettings.qml, SetStorage.qml, SetDrives.qml, SetYourDesktop.qml,
//! SetBoot.qml's "splash" part, SetRemoteDesktop.qml), with the same helpers: hearth-notify (each app's
//! notification settings), notify.json (quiet by itself), hearth-memory, Hearth's core (`nimbus --settings-call
//! diskspace.*`), hearth-files (prefs, drive letters), hearth-boot-look and hearth-admin boot-splash, hearth-vnc.
//!
//! Notifications on KDE Plasma: Plasma shows the notifications there, so Do not disturb, quiet in full screen or while
//! sharing the screen, and each app's pop-ups are Plasma's own (plasmanotifyrc, written with kwriteconfig6 so Plasma
//! takes them at once); Hearth's notify.json keeps its copy for Mantel.

use super::*;
use iced::widget::{column, row};
use hearth_ui::ui::{self, ts, Kind};
use crate::widgets::btn;
use iced::widget::{container, Row};
use serde_json::json;

pub fn view<'a>(a: &'a App, page: &'static str) -> El<'a> {
    match page {
        "notifications" => notifications(a),
        "memory" => memory(a),
        "storage" => storage(a),
        "drives" => drives(a),
        "your-desktop" => your_desktop(a),
        "boot" => boot(a),
        _ => remote(a),
    }
}

// ------------------------------------------------------------------ Notifications

/// "2026,11,18,22,0,0" (KConfig's date) as numbers to compare.
fn kdate(s: &str) -> Vec<i64> {
    s.split(',').map(|x| x.trim().split('.').next().unwrap_or("0").parse().unwrap_or(0)).collect()
}

fn notifications(a: &App) -> El<'_> {
    let p = a.sh.p;
    let st = &a.system;
    let k = st.j("k-notify");
    let pl = plasma();
    let hub = st.file("notify.json");
    let pref = |key: &str, d: bool| -> bool { at(hub, key).as_bool().unwrap_or(d) };
    let mut c = Column::new().spacing(10.0);
    c = c.push(head(&p, "Pop-ups and the list"));
    let dnd = if pl {
        let u = vs(k, "until");
        !u.is_empty() && kdate(&u) > kdate(&vs(k, "now"))
    } else {
        vb(st.j("desk"), "dnd")
    };
    let dnd_msg = if pl {
        Msg::Exec(Ex::new(if dnd { kdel("plasmanotifyrc", &["DoNotDisturb"], "Until") } else { kw("plasmanotifyrc", &["DoNotDisturb"], "Until", "2099,12,31,23,59,59") }).re("k-notify").ok(if dnd { "Do not disturb is off" } else { "Do not disturb is on" }))
    } else {
        Msg::Exec(Ex::new(py("hearth-desk", &["set", "dnd", if dnd { "off" } else { "on" }])).re("desk"))
    };
    c = c.push(srow(&p, G::Moon, "Do not disturb", if pl { "Notifications go straight to the list without popping up (urgent ones still do). The bell in the panel switches it too" } else { "Notifications go straight to the list without popping up (urgent ones still do). Right-click the bell on the taskbar to switch it, or press Super+N for the list" }, sw(&p, dnd, dnd_msg)));
    if pl {
        c = c.push(info_row(&p, G::List, "The notification list", "Plasma keeps it in the bell in the panel: open it there to read or clear it"));
    }
    c = c.push(head(&p, "Quiet by itself"));
    // Plasma's own: WhenFullscreen, WhenScreenSharing (and WhenScreensMirrored, the same idea), on unless turned off.
    let kb = |key: &str, d: bool| -> bool { let v = vs(k, key); if v.is_empty() { d } else { v == "true" } };
    let full = if pl { kb("fullscreen", true) } else { pref("autoFullscreen", true) };
    let share = if pl { kb("sharing", true) } else { pref("autoSharing", true) };
    let quiet = |hkey: &'static str, on: bool, kde: &[&'static str]| -> Msg {
        let mut ex = Ex::none().re("k-notify");
        if pl {
            for kk in kde {
                ex = ex.and(kw("plasmanotifyrc", &["DoNotDisturb"], kk, if on { "true" } else { "false" }));
            }
        }
        let mut o = serde_json::Map::new();
        o.insert(hkey.into(), json!(on));
        Msg::Json("notify.json", Value::Object(o), ex)
    };
    c = c.push(srow(&p, G::Pad, "While a full-screen app is in front", "Games and videos: pop-ups wait in the list, and you're told how many came in when you leave full screen", sw(&p, full, quiet("autoFullscreen", !full, &["WhenFullscreen"]))));
    c = c.push(srow(&p, G::Display, "While you share or record your screen", "Screen sharing in a call, Hearth's recorder, Remote desktop, or the same picture on two screens: nobody else sees your notifications", sw(&p, share, quiet("autoSharing", !share, &["WhenScreenSharing", "WhenScreensMirrored"]))));
    let sched = pref("schedule", false);
    c = c.push(srow(&p, G::Clock, "On a schedule", if pl { "Every day between the times below, in Mantel (Hearth's desktop). KDE Plasma has no schedule of its own" } else { "Every day between the times below. Press Super+Alt+N any time for Do not disturb" }, sw(&p, sched, quiet("schedule", !sched, &[]))));
    if sched {
        let times: Vec<(String, String)> = (0..48).map(|i| { let v = format!("{:02}:{}", i / 2, if i % 2 == 0 { "00" } else { "30" }); (v.clone(), v) }).collect();
        let from = { let v = vs(hub, "scheduleFrom"); if v.is_empty() { "22:00".to_string() } else { v } };
        let to = { let v = vs(hub, "scheduleTo"); if v.is_empty() { "07:00".to_string() } else { v } };
        c = c.push(chooser(a, "sfrom", G::Moon, "From", "", times.clone(), &from, |v| Msg::Json("notify.json", json!({"scheduleFrom": v}), Ex::none())));
        c = c.push(chooser(a, "sto", G::Display, "Until", "", times, &to, |v| Msg::Json("notify.json", json!({"scheduleTo": v}), Ex::none())));
    }
    c = c.push(info_row(&p, G::Info, "Hearth's own warnings always show", "Low space, a low battery, a settings file put back: they pop up even when it's quiet, and can't be muted"));
    c = c.push(head(&p, "History"));
    let keep = pref("keepHistory", true);
    c = c.push(srow(&p, G::List, "Keep the list after a restart", if pl { "In Mantel's list: the last 100, with their buttons while their app can still answer (Plasma keeps its own)" } else { "The last 100, with their buttons while their app can still answer. Off: the list starts empty each time (what's kept now is cleared)" }, sw(&p, keep, Msg::Json("notify.json", json!({"keepHistory": !keep}), Ex::none()))));
    c = c.push(link_row(&p, G::Sound, "Sounds", "Hearth's own short sounds, the freedesktop set, or none, and which events play one", Msg::Go("sounds")));

    // ---- each app: Hearth's (hearth-notify) and, on Plasma, the apps Plasma has settings for
    c = c.push(head(&p, "Each app"));
    let rules = at(st.j("notify"), "apps");
    let mut apps: Vec<(String, bool)> = rules.as_object().map(|m| m.keys().map(|n| (n.clone(), false)).collect()).unwrap_or_default();
    if pl {
        if let Some(m) = k.as_object() {
            for key in m.keys() {
                if let Some(rest) = key.strip_prefix("app.") {
                    if let Some((name, _)) = rest.rsplit_once('.') {
                        if !apps.iter().any(|x| x.0 == name) {
                            apps.push((name.to_string(), true));
                        }
                    }
                }
            }
        }
    }
    apps.sort_by(|x, y| x.0.to_lowercase().cmp(&y.0.to_lowercase()));
    if apps.is_empty() {
        c = c.push(line(&p, "Apps show here once they've sent a notification."));
    }
    for (name, kde) in apps {
        let r = rules.get(&name).unwrap_or(&NULL);
        let allow = if kde { kg(k, &format!("app.{name}.ShowInHistory")) != "false" || kg(k, &format!("app.{name}.ShowPopups")) != "false" } else { at(r, "allow").as_bool().unwrap_or(true) };
        let popups = if kde { kg(k, &format!("app.{name}.ShowPopups")) != "false" } else { !va(st.j("desk"), "muted").iter().any(|m| m.as_str() == Some(name.as_str())) };
        let sound = vb(r, "sound");
        let lock = vb(r, "lockScreen");
        let prio = if kde { if kg(k, &format!("app.{name}.ShowPopupsInDndMode")) == "true" { "high".to_string() } else { "normal".to_string() } } else { let v = vs(r, "priority"); if v.is_empty() { "normal".to_string() } else { v } };
        let summary = if !allow { "Off: nothing from it is shown or kept".to_string() } else { [if popups { "Pop-ups" } else { "No pop-ups" }, if sound { "sound" } else { "" }, if lock { "on the lock screen" } else { "" }, if prio == "high" { "high priority" } else if prio == "low" { "list only" } else { "" }].iter().filter(|x| !x.is_empty()).copied().collect::<Vec<_>>().join(" · ") };
        let key = format!("app:{name}");
        let open = st.open == key;
        c = c.push(iced::widget::mouse_area(srow(&p, if allow { G::Info } else { G::Stop }, name.clone(), summary, pill(&p, if open { "Close" } else { "Change" }, true, Some(Msg::Open(key.clone()))))).on_press(Msg::Open(key.clone())));
        if !open {
            continue;
        }
        let set = |kk: &str, v: &str| -> Ex { Ex::new(py_s("hearth-notify", vec!["set".into(), name.clone(), kk.into(), v.into()])).re("notify").fail("Couldn't change that") };
        let kset = |kk: &str, v: bool| -> Vec<String> { kw("plasmanotifyrc", &["Applications", &name], kk, if v { "true" } else { "false" }) };
        let mut inner = Column::new().spacing(12.0);
        let allow_msg = if kde { Msg::Exec(Ex::new(kset("ShowPopups", !allow)).and(kset("ShowInHistory", !allow)).re("k-notify")) } else { Msg::Exec(set("allow", if allow { "off" } else { "on" })) };
        inner = inner.push(inner_row(&p, G::Info, "Notifications", "Off: nothing from this app is shown or kept, urgent ones neither", sw(&p, allow, allow_msg)));
        if allow {
            let pop_msg = if kde { Msg::Exec(Ex::new(kset("ShowPopups", !popups)).re("k-notify")) } else { Msg::Exec(Ex::new(py_s("hearth-desk", vec!["mute-app".into(), name.clone(), if popups { "on".into() } else { "off".into() }])).re("desk")) };
            inner = inner.push(inner_row(&p, G::List, "Pop-ups", "Off: they only go to the list", sw(&p, popups, pop_msg)));
            if !kde {
                inner = inner.push(inner_row(&p, G::Sound, "Play a sound", "", sw(&p, sound, Msg::Exec(set("sound", if sound { "off" } else { "on" })))));
                inner = inner.push(inner_row(&p, G::Shield, "Show on the lock screen", "Its name and the title, nothing more", sw(&p, lock, Msg::Exec(set("lockScreen", if lock { "off" } else { "on" })))));
            }
            let pr = |v: &str| -> Msg {
                if kde {
                    Msg::Exec(Ex::new(kset("ShowPopupsInDndMode", v == "high")).and(kset("ShowPopups", v != "low")).re("k-notify"))
                } else {
                    Msg::Exec(set("priority", v))
                }
            };
            inner = inner.push(inner_row(&p, G::Update, "Priority", "High pops up even with Do not disturb on; Low goes straight to the list", choice(&p, vec![("Low".into(), prio == "low", pr("low")), ("Normal".into(), prio == "normal", pr("normal")), ("High".into(), prio == "high", pr("high"))])));
        }
        c = c.push(card(&p, inner));
    }
    c.into()
}

// ------------------------------------------------------------------ Memory (MemorySettings.qml)

fn mval<'v>(st: &'v State, group: &str, key: &str) -> &'v Value {
    let d = at(st.j("memdraft"), &format!("{group}.{key}"));
    if !d.is_null() {
        return d;
    }
    at(st.j("memory"), &format!("settings.{group}.{key}"))
}

fn draft(path: &str, v: Value) -> Msg {
    Msg::Draft("memdraft", path.to_string(), v)
}

fn chips<'a>(p: &Pal, opts: Vec<(String, String)>, cur: &str, on: impl Fn(String) -> Msg) -> El<'a> {
    choice(p, opts.into_iter().map(|(v, l)| (l, v == cur, on(v))).collect())
}

fn memory(a: &App) -> El<'_> {
    let p = a.sh.p;
    let st = &a.system;
    let m = st.j("memory");
    if let Some(w) = waiting(a, "memory", "the memory settings") {
        return w;
    }
    let gib = 1073741824.0f64;
    let gb = |b: f64| if b >= 10.0 * gib { format!("{:.0}", b / gib) } else { format!("{:.1}", b / gib) };
    let mem_gb = (vf(m, "memory.total", 0.0) / gib).round().max(1.0);
    let d = st.j("memdraft");
    let dirty = d.as_object().map(|o| !o.is_empty()).unwrap_or(false);
    let mut c = Column::new().spacing(12.0);
    // The pills along the top.
    let mut pills = Row::new().spacing(8.0);
    let pw = vs(m, "pressure.word");
    pills = pills.push(ui::chip(format!("Pressure: {pw}"), Pal::a(if pw == "none" || pw == "light" { p.good } else { p.flame }, 0.18), if pw == "none" || pw == "light" { p.good } else { p.flame }));
    if vb(m, "zram.active") {
        let ratio = vf(m, "zram.ratio", 0.0);
        pills = pills.push(ui::chip(format!("Zram {} / {} GB{}", gb(vf(m, "zram.used", 0.0)), gb(vf(m, "zram.disksize", 0.0)), if ratio > 0.0 { format!(" · {ratio}:1") } else { String::new() }), p.panel3, p.text));
    }
    if vb(m, "swapfile.exists") {
        let path = vs(m, "swapfile.path");
        let used = va(m, "swaps").iter().find(|s| vs(s, "path") == path).map(|s| vf(s, "used", 0.0)).unwrap_or(0.0);
        pills = pills.push(ui::chip(format!("Drive swap {} / {} GB", gb(used), vf(m, "swapfile.gb", 0.0)), p.panel3, p.text));
    }
    c = c.push(pills);
    if let Some(tf) = at(m, "testFiles").as_object().filter(|o| !o.is_empty()) {
        let names: Vec<String> = tf.values().map(|v| v.as_str().unwrap_or("").to_string()).collect();
        c = c.push(card(&p, row![t(format!("Your test settings are in use ({}). Hearth shows their values here and leaves them as they are. Use Hearth's settings to hand them over: the same values, kept by Hearth, with the files kept as .bak.", names.join(", ")), sz(13.0), p.text).width(Length::Fill), btn(&p, "Use Hearth's settings", Kind::Normal, false, Some(Msg::Exec(Ex::new(adm("hearth-memory", sv(&["adopt"]))).re("memory").ok("Your settings are in use now ✓"))))].spacing(12.0).align_y(iced::Center)));
    }
    let mcard = |title: &str, sub: &str, badge: bool, body: Column<'static, Msg>| -> El<'static> {
        let mut top = Row::new().spacing(8.0).align_y(iced::Center).push(ts(title.to_string(), sz(16.0), p.text));
        if badge {
            top = top.push(ui::chip("NEW", Pal::a(p.flame, 0.15), p.flame));
        }
        card(&p, column![top, t(sub.to_string(), sz(12.0), p.muted), body.spacing(12.0)].spacing(6.0))
    };
    let mline = |title: &str, sub: &str, ctl: El<'static>| -> El<'static> { inner_row(&p, G::Drive, title.to_string(), sub.to_string(), ctl) };

    // Compressed memory (zram)
    let z_on = mval(st, "zram", "on").as_bool().unwrap_or(true);
    let z_gb = vf(mval(st, "zram", "gb"), "", 8.0);
    let algo = { let v = mval(st, "zram", "algo").as_str().unwrap_or("zstd").to_string(); v };
    let algos: Vec<(String, String)> = va(m, "algos").iter().take(4).map(|x| { let s = x.as_str().unwrap_or("").to_string(); (s.clone(), s) }).collect();
    let ratio = vf(m, "zram.ratio", 3.0).max(1.0);
    let rec_gb = vf(m, "recommended.zram.gb", 0.0);
    let mut zb = Column::new();
    zb = zb.push(mline("On", "", sw(&p, z_on, draft("zram.on", json!(!z_on)))));
    zb = zb.push(mline("Size", &format!("{} GB · how much overflow it can hold (up to twice your memory{})", z_gb, if rec_gb > 0.0 { format!("; {rec_gb} GB is recommended") } else { String::new() }), slider_f32(z_gb as f32, 1.0..=(mem_gb as f32 * 2.0).max(2.0), 1.0, |v| draft("zram.gb", json!(v.round() as i64)), Msg::Text("noop", String::new()))));
    zb = zb.push(mline("Compression", "zstd squeezes most · lz4 is quickest (only what this kernel offers)", chips(&p, algos, &algo, |v| draft("zram.algo", json!(v)))));
    zb = zb.push(line(&p, format!("When full, holds {} GB in about {} GB of real memory ({}{}:1){}.", z_gb, (z_gb / ratio).round().max(1.0), if present(m, "zram.ratio") { "your usual " } else { "about " }, ratio, if z_gb == rec_gb { format!(" · recommended for {mem_gb} GB") } else { String::new() })));
    c = c.push(mcard("Compressed memory (zram)", "Squeezes idle memory inside RAM instead of writing it to the drive. Fast, and it uses nothing until it's needed.", false, zb));

    // Keep memory free
    let keep_mode = { let v = vs(d, "keep.mode"); if !v.is_empty() { v } else { let v = vs(m, "settings.keep.mode"); if v.is_empty() { "auto".into() } else { v } } };
    let swappiness = if present(d, "swappiness") { vf(d, "swappiness", 200.0) } else { vf(m, "settings.swappiness", 200.0) };
    let mut kb = Column::new();
    let keep_opts: Vec<(String, String)> = [("auto", "Auto"), ("off", "Off"), ("2gb", "2 GB"), ("25%", "25%"), ("50%", "50%"), ("custom", "Custom")].iter().map(|(v, l)| (v.to_string(), l.to_string())).collect();
    kb = kb.push(mline("Target", "", chips(&p, keep_opts, &keep_mode, |v| if v == "custom" { draft("keep", json!({"mode": "custom", "factor": 500})) } else { draft("keep", json!({"mode": v})) })));
    if keep_mode == "custom" {
        let factor = if present(d, "keep.factor") { vf(d, "keep.factor", 500.0) } else { vf(m, "settings.keep.factor", 500.0) };
        kb = kb.push(mline("Keep free", &format!("{}% · a share of your memory", (factor / 50.0).round()), slider_f32((factor / 50.0) as f32, 1.0..=60.0, 1.0, |v| draft("keep", json!({"mode": "custom", "factor": ((v * 50.0) as i64).clamp(10, 3000)})), Msg::Text("noop", String::new()))));
    }
    if vf(m, "watermarks.low", 0.0) > 0.0 {
        let low = (100.0 * vf(m, "watermarks.low", 0.0) / vf(m, "memory.total", 1.0).max(1.0)).round().max(1.0);
        kb = kb.push(mline("Starts freeing below", "then frees up to the target", t1(format!("{low}% free"), sz(13.0), p.text).into()));
    }
    kb = kb.push(mline("Swappiness", &format!("{} · how readily idle memory is compressed instead of dropping cached files", swappiness), slider_f32(swappiness as f32, 0.0..=200.0, 1.0, |v| draft("swappiness", json!(v.round() as i64)), Msg::Text("noop", String::new()))));
    if let Some(over) = at(m, "overridden").as_object().filter(|o| !o.is_empty()) {
        let words: Vec<String> = over.keys().map(|k| format!("vm.{k} = {}", vs(m, &format!("vm.{k}")))).collect();
        kb = kb.push(row![t(format!("Not in use: {}", words.join(", ")), sz(12.0), p.warn).width(Length::Fill), btn(&p, "Use these settings", Kind::Normal, false, Some(Msg::Exec(Ex::new(adm("hearth-memory", sv(&["handover"]))).re("memory"))))].spacing(10.0).align_y(iced::Center));
    }
    if keep_mode != "auto" && !present(d, "keep") {
        let mb = ((vf(m, "memory.total", 0.0) * 0.025 / 1048576.0 / 10.0).round() * 10.0).max(1.0);
        kb = kb.push(row![t(format!("Recommended: Auto. It keeps a small share of memory free (about {mb} MB here) instead of a fixed amount, so the PC isn't busy freeing memory all the time. Your choice stays until you change it."), sz(12.0), p.muted).width(Length::Fill), btn(&p, "Use Auto", Kind::Normal, false, Some(draft("keep", json!({"mode": "auto"}))))].spacing(10.0).align_y(iced::Center));
    }
    if keep_mode == "50%" || keep_mode == "25%" {
        kb = kb.push(line(&p, format!("{keep_mode} works your memory harder: more compressing, a little more battery. Fine for browsing and AI work; if a game stutters, try Auto.")));
    }
    c = c.push(mcard("Keep memory free", "How much the system keeps empty, ready for the next heavy moment. It moves idle memory into zram to keep it.", false, kb));

    // Drive swap file
    let everyday = mval(st, "swapfile", "everyday").as_bool().unwrap_or(true);
    let cur_gb = vf(m, "swapfile.gb", 0.0) as i64;
    let swap_gb = if present(d, "swapGb") { vf(d, "swapGb", 0.0) as i64 } else { cur_gb };
    let max = vf(m, "limits.swapfileMaxGb", 0.0) as i64;
    let mut sizes: Vec<i64> = vec![0, 4, 8, 16, 32, mem_gb as i64, 64, cur_gb];
    sizes.retain(|v| *v <= max.max(cur_gb));
    sizes.sort();
    sizes.dedup();
    let custom = st.on("swapcustom");
    let mut sb = Column::new();
    sb = sb.push(mline("Use for everyday overflow", "off = only for hibernation", sw(&p, everyday, draft("swapfile.everyday", json!(!everyday)))));
    let mut size_opts: Vec<(String, bool, Msg)> = sizes.iter().map(|v| (if *v == 0 { "Off".to_string() } else { format!("{v} GB") }, !custom && swap_gb == *v, draft("swapGb", json!(v)))).collect();
    size_opts.push(("Custom".into(), custom, Msg::Flag("swapcustom", !custom)));
    sb = sb.push(mline("Size", &format!("on C: (/var/swap) · hibernation needs at least your memory size · C: always keeps 20 GB free · at most {max} GB here"), choice(&p, size_opts)));
    if custom {
        sb = sb.push(mline("Custom size", &format!("{} GB · 1 GB up to {max} GB", swap_gb.max(1)), slider_f32(swap_gb.max(1) as f32, 1.0..=(max.max(2) as f32), 1.0, |v| draft("swapGb", json!(v.round() as i64)), Msg::Text("noop", String::new()))));
    }
    sb = sb.push(line(&p, if vb(m, "swapfile.hibernate") { "Hibernation has its own space (/var/swap/hearth-hibernate), left as it is." } else { "Changing the size makes the file again when you press Save (it can take a minute)." }));
    let hib = vf(m, "hibernateChecked", 0.0);
    sb = sb.push(mline("Hibernation", if hib > 0.0 { "works on this laptop" } else { "not tried on this PC yet" }, t1(if hib > 0.0 { "ready" } else { "" }, sz(12.0), p.good).into()));
    c = c.push(mcard("Drive swap file", "The slow overflow on your drive (C:, /var/swap). Used only when zram is full, and for hibernation.", false, sb));

    // When memory runs out (the memory guard)
    let level = mval(st, "guard", "level").as_str().unwrap_or("gentle").to_string();
    let never: Vec<String> = va(if present(d, "guard.never") { d } else { m }, if present(d, "guard.never") { "guard.never" } else { "settings.guard.never" }).iter().map(|x| x.as_str().unwrap_or("").to_string()).collect();
    let extra: Vec<String> = va(if present(d, "guard.extra") { d } else { m }, if present(d, "guard.extra") { "guard.extra" } else { "settings.guard.extra" }).iter().map(|x| x.as_str().unwrap_or("").to_string()).collect();
    let checkpoint: Vec<String> = va(if present(d, "guard.checkpoint") { d } else { m }, if present(d, "guard.checkpoint") { "guard.checkpoint" } else { "settings.guard.checkpoint" }).iter().map(|x| x.as_str().unwrap_or("").to_string()).collect();
    let mut gbx = Column::new();
    gbx = gbx.push(mline("Protection", "", chips(&p, vec![("off".into(), "Off".into()), ("gentle".into(), "Gentle".into()), ("strong".into(), "Strong".into())], &level, |v| draft("guard.level", json!(v)))));
    let mut nv = Row::new().spacing(6.0);
    if let Some(labels) = at(m, "neverLabels").as_object() {
        for (key, label) in labels {
            let on = never.contains(key);
            let mut l = never.clone();
            if on { l.retain(|x| x != key); } else { l.push(key.clone()); }
            nv = nv.push(pill(&p, label.as_str().unwrap_or(key).to_string(), on, Some(draft("guard.never", json!(l)))));
        }
    }
    for e in &extra {
        let l: Vec<String> = extra.iter().filter(|x| *x != e).cloned().collect();
        nv = nv.push(pill(&p, format!("{e}  ✕"), true, Some(draft("guard.extra", json!(l)))));
    }
    gbx = gbx.push(mline("Never close", "always kept running (press one to take it off the list)", nv.into()));
    let typed = st.txt("never").trim().to_string();
    let ok_name = !typed.is_empty() && typed.len() <= 64 && typed.chars().all(|ch| ch.is_alphanumeric() || "_.+-".contains(ch)) && !extra.contains(&typed);
    let mut add = extra.clone();
    add.push(typed.clone());
    gbx = gbx.push(mline("Add a program", "its program name, then Enter", field(&p, "program name", st.txt("never"), |s| Msg::Text("never", s)).on_submit(if ok_name { draft("guard.extra", json!(add)) } else { Msg::Text("never", typed.clone()) }).width(Length::Fixed(sz(220.0))).into()));
    if !checkpoint.is_empty() {
        let mut cp = Row::new().spacing(6.0);
        for e in &checkpoint {
            let l: Vec<String> = checkpoint.iter().filter(|x| *x != e).cloned().collect();
            cp = cp.push(pill(&p, format!("{e}  ✕"), true, Some(draft("guard.checkpoint", json!(l)))));
        }
        gbx = gbx.push(mline("Pause instead of closing", "frozen, its memory moved to the drive; Resume in Task Manager", cp.into()));
    }
    gbx = gbx.push(mline("Close first", "the program using the most memory and swap", t1("automatic", sz(12.0), p.good).into()));
    let last = va(m, "lastClosed");
    if !last.is_empty() {
        let words: Vec<String> = last.iter().rev().take(3).map(|x| format!("{} ({}{:.1} GB)", vs(x, "name"), if vb(x, "paused") { "paused, " } else { "" }, vf(x, "mb", 0.0) / 1024.0)).collect();
        gbx = gbx.push(line(&p, format!("Last closed: {}", words.join(", "))));
    }
    c = c.push(mcard("When memory runs out", "Instead of the whole PC freezing, Hearth closes the one program eating memory. It tells you which, and you can open it again.", true, gbx));

    // Save
    let mut changes = serde_json::Map::new();
    if let Some(o) = d.as_object() {
        for (k, v) in o {
            if k != "swapGb" {
                changes.insert(k.clone(), v.clone());
            }
        }
    }
    let sw_new = if present(d, "swapGb") && swap_gb != cur_gb { Some(swap_gb) } else { None };
    let set_args = |try_: bool| -> Vec<String> {
        let mut v = vec!["set".to_string()];
        if try_ {
            v.push("--try".into());
        }
        v.push(Value::Object(changes.clone()).to_string());
        v
    };
    let mut save = Ex::none().re("memory").ok("Saved ✓").fail("Couldn't save the memory settings").secs(300);
    if let Some(n) = sw_new {
        save = save.and(adm("hearth-memory", vec!["swapfile".into(), n.to_string()]));
    }
    if !changes.is_empty() {
        save = save.and(adm("hearth-memory", set_args(false)));
    }
    let try_ex = Ex::new(adm("hearth-memory", set_args(true))).re("memory").ok("Tried until the next restart").secs(120);
    let words = if dirty { "Not saved yet." } else { "Changes are saved and kept until you change them. Nothing here changes by itself." };
    let mut foot = Row::new().spacing(8.0).align_y(iced::Center).push(t(words, sz(13.0), p.muted).width(Length::Fill));
    foot = foot.push(btn(&p, "Back to recommended", Kind::Normal, false, Some(Msg::Seq(vec![Msg::Draft("memdraft", String::new(), Value::Null), Msg::Flag("swapcustom", false), Msg::Exec(Ex::new(adm("hearth-memory", sv(&["recommended"]))).re("memory").ok("Back to the recommended settings ✓").secs(120))]))));
    if dirty && !changes.is_empty() {
        foot = foot.push(btn(&p, "Try until restart", Kind::Normal, false, Some(Msg::Exec(try_ex))));
    }
    foot = foot.push(btn(&p, "Save", Kind::Primary, false, if dirty { Some(Msg::Seq(vec![Msg::Exec(save), Msg::Draft("memdraft", String::new(), Value::Null)])) } else { None }));
    c = c.push(card(&p, foot));
    c.into()
}

// ------------------------------------------------------------------ Storage

fn gbs(b: f64) -> String {
    if b >= 1e12 {
        format!("{:.1} TB", b / 1e12)
    } else if b > 0.0 && b < 1e9 {
        format!("{} MB", (b / 1e6).round().max(1.0))
    } else {
        format!("{} GB", (b / 1e9).round())
    }
}

fn core(method: &str, args: Value) -> Vec<String> {
    sv(&["nimbus", "--settings-call", method, &args.to_string()])
}

fn storage(a: &App) -> El<'_> {
    let p = a.sh.p;
    let st = &a.system;
    let v = at(st.j("disk"), "value");
    let mut c = Column::new().spacing(10.0);
    if v.is_null() {
        // Hearth's core couldn't be asked (the Hearth app isn't here, or it's still reading): the drives from df.
        if !st.read("disk") {
            c = c.push(note(&p, "Looking at your drives…", false));
        }
        let drives = data::storage(st.raw("df"));
        if !drives.is_empty() {
            c = c.push(head(&p, "Drives"));
            c = c.push(card(&p, drive_rows(&p, &drives)));
        }
        if st.read("disk") {
            c = c.push(line(&p, format!("Games by size and Clean up come from the Hearth app, which didn't answer{}.", if vs(st.j("disk"), "error").is_empty() { String::new() } else { format!(": {}", vs(st.j("disk"), "error")) })));
        }
        c = c.push(link_row(&p, G::Drive, "Disk Usage", "What's using the space, folder by folder", Msg::App(sv(&["hearth-disk-usage"]))));
        return c.into();
    }
    for l in va(v, "low") {
        c = c.push(note(&p, format!("Low on space: {} has {} left", if vs(l, "name").is_empty() { "a drive".to_string() } else { vs(l, "name") }, gbs(vf(l, "free", 0.0))), true));
    }
    c = c.push(head(&p, "Drives"));
    for d in va(v, "drives") {
        let size = vf(d, "size", 0.0);
        let used = vf(d, "used", 0.0);
        let sub = format!("{} free of {}{}{}", gbs(vf(d, "free", 0.0)), gbs(size), if vb(d, "games") { " · games go here" } else { "" }, if vb(d, "readOnly") { " · read only" } else { "" });
        let title = if vs(d, "name").is_empty() { vs(d, "mount") } else { vs(d, "name") };
        c = c.push(srow(&p, G::Folder, title, sub, column![t1(if size > 0.0 { format!("{}% used", (100.0 * used / size).round()) } else { String::new() }, sz(13.0), p.muted), container(bar_of(&p, used, size)).width(Length::Fixed(sz(200.0)))].spacing(6.0).align_x(iced::Alignment::End).into()));
    }
    c = c.push(head(&p, "Games by size"));
    let mut games: Vec<&Value> = va(v, "games").iter().filter(|g| vf(g, "size", 0.0) > 0.0).collect();
    games.sort_by(|x, y| vf(y, "size", 0.0).partial_cmp(&vf(x, "size", 0.0)).unwrap_or(std::cmp::Ordering::Equal));
    for g in games.iter().take(12) {
        let sub: Vec<String> = ["store", "drive"].iter().map(|k| vs(g, k)).filter(|x| !x.is_empty()).collect();
        c = c.push(value_row(&p, G::Pad, vs(g, "title"), sub.join(" · "), gbs(vf(g, "size", 0.0)), false));
    }
    if games.is_empty() {
        c = c.push(line(&p, "Game sizes haven't been added up yet."));
    }
    c = c.push(link_row(&p, G::Store, "Move games and add up sizes", "Moves a game to another drive, or adds up every game's size again. Opens in Hearth Game Mode (Desktop Mode closes first).", Msg::App(sv(&["nimbus-mode", "hearth-page", "diskspace"]))));
    c = c.push(head(&p, "Clean up"));
    let clean = va(st.j("clean"), "value.items");
    let have = st.has("clean");
    c = c.push(act_row(&p, G::Search, if have { "Look again" } else { "Find things to clean up" }, "Shader caches, Proton prefixes of games that are gone, and leftover downloads. Nothing is deleted until you say so.", if have { "Look again" } else { "Find" }, Some(Msg::Exec(Ex::new(core("diskspace.cleanList", json!({}))).store("clean").fail("Couldn't look for things to clean up").secs(120)))));
    let picked = st.j("cleanpick");
    let is_on = |c: &Value| -> bool { let k = vs(c, "key"); match picked.get(&k) { Some(Value::Bool(b)) => *b, _ => vb(c, "selectable") && vb(c, "on") } };
    for it in clean {
        let saves: Vec<String> = va(it, "saves").iter().map(|x| x.as_str().unwrap_or("").to_string()).collect();
        let mut parts = vec![vs(it, "where"), gbs(vf(it, "bytes", 0.0))];
        if !saves.is_empty() {
            parts.push(format!("has saves: {}{}", saves.join(", "), if vb(it, "cloud") { " (Steam Cloud has a copy)" } else { "" }));
        }
        if !vb(it, "selectable") {
            parts.push(if vs(it, "why").is_empty() { "kept".into() } else { vs(it, "why") });
        }
        let on = is_on(it);
        c = c.push(srow(&p, G::Drive, vs(it, "title"), parts.into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" · "), switch(&p, on, if vb(it, "selectable") { Some(Msg::Draft("cleanpick", vs(it, "key"), json!(!on))) } else { None })));
    }
    if have && clean.is_empty() {
        c = c.push(line(&p, "Nothing to clean up."));
    }
    if !clean.is_empty() {
        let chosen: Vec<&Value> = clean.iter().filter(|x| vb(x, "selectable") && is_on(x)).collect();
        let bytes: f64 = chosen.iter().map(|x| vf(x, "bytes", 0.0)).sum();
        let keys: Vec<String> = chosen.iter().map(|x| vs(x, "key")).collect();
        let confirmed: Vec<String> = chosen.iter().filter(|x| vb(x, "confirm")).map(|x| vs(x, "key")).collect();
        let asking = st.on("cleanask");
        c = c.push(srow(&p, G::Close, format!("Delete {} item{}", chosen.len(), if chosen.len() == 1 { "" } else { "s" }), format!("Frees about {}. This can't be undone.", gbs(bytes)), btn(&p, "Delete…", Kind::Danger, false, if chosen.is_empty() { None } else { Some(Msg::Flag("cleanask", !asking)) })));
        if asking {
            let run = Msg::Seq(vec![
                Msg::Flag("cleanask", false),
                Msg::Exec(Ex::new(core("diskspace.clean", json!({"keys": keys, "confirmed": confirmed}))).store("cleaned").re("disk").ok("Cleaned up").fail("Couldn't clean up").secs(600)),
                Msg::Draft("clean", String::new(), Value::Null),
            ]);
            c = c.push(srow(&p, G::Check, "Yes, delete them", if !confirmed.is_empty() { "Some of these have saves on this device: they go too." } else { "Press to delete." }, btn(&p, "Yes, delete them", Kind::Danger, false, Some(run))));
        }
    }
    let freed = at(st.j("cleaned"), "value");
    if !freed.is_null() {
        let failed = va(freed, "failed");
        c = c.push(line(&p, format!("Freed {}{}", gbs(vf(freed, "freed", 0.0)), if failed.is_empty() { String::new() } else { format!(" · {} kept: {}", failed.len(), vs(&failed[0], "why")) })));
    }
    c = c.push(head(&p, "Low space warning"));
    let warn_on = vb(v, "warn.on");
    let warn_gb = vf(v, "warn.gb", 15.0) as i64;
    let set_warn = |on: bool, g: i64| Msg::Exec(Ex::new(core("diskspace.setWarn", json!({"on": on, "gb": g}))).re("disk").fail("Couldn't change the warning"));
    c = c.push(srow(&p, G::Drive, "Warn when a games drive is low on space", "Before an install and in notifications", sw(&p, warn_on, set_warn(!warn_on, warn_gb))));
    if warn_on {
        c = c.push(srow(&p, G::Drive, "Low means less than", "", choice(&p, [5, 10, 15, 20, 30, 50, 100].iter().map(|g| (format!("{g} GB"), warn_gb == *g, set_warn(true, *g))).collect())));
    }
    c = c.push(link_row(&p, G::Drive, "Disk Usage", "What's using the space, folder by folder", Msg::App(sv(&["hearth-disk-usage"]))));
    c.into()
}

// ------------------------------------------------------------------ Drives & folders (Hearth Files' own settings)

fn drives(a: &App) -> El<'_> {
    let p = a.sh.p;
    let st = &a.system;
    let pr = st.j("prefs");
    let pc = st.j("thispc");
    let mut c = Column::new().spacing(10.0);
    if let Some(w) = waiting(a, "prefs", "Hearth Files' settings") {
        c = c.push(w);
    }
    let cdrive = at(pr, "cdrive").as_bool().unwrap_or(true);
    let linux = vb(pr, "linuxFolders");
    let addr = if vs(pr, "addressBar") == "linux" { "linux" } else { "windows" };
    let copy = if vs(pr, "copyPath") == "linux" { "linux" } else { "windows" };
    let term = at(pr, "terminalWindows").as_bool().unwrap_or(true);
    let save = |k: &str, v: Value| -> Option<Msg> {
        if !st.has("prefs") {
            return None;
        }
        let mut o = serde_json::Map::new();
        o.insert(k.into(), v);
        Some(Msg::Exec(Ex::new(py_s("hearth-files", vec!["prefs-set".into(), Value::Object(o).to_string()])).re("prefs").fail("Couldn't save that")))
    };
    let user = { let u = vs(pc, "user"); if u.is_empty() { a.sh.info.user.clone() } else { u } };
    let home = hearth_ui::paths::home().to_string_lossy().to_string();
    c = c.push(head(&p, "How Files shows Hearth's drive"));
    c = c.push(srow(&p, G::Drive, "Show Hearth (C:) with Windows folder names", "Program Files, Users, Windows… Only names: nothing on your drive moves.", switch(&p, cdrive, save("cdrive", json!(!cdrive)))));
    c = c.push(srow(&p, G::Folder, "Show system folders", "/usr, /etc and the other Linux folders in Files and every app's Open and Save window (hidden by default, like macOS)", switch(&p, linux, if cdrive { save("linuxFolders", json!(!linux)) } else { None })));
    c = c.push(srow(&p, G::Grid, "The address bar shows", "Typing either kind always works", seg(&p, vec![
        ("Windows names (C:\\…)".into(), addr == "windows", if cdrive && addr != "windows" { save("addressBar", json!("windows")) } else { None }),
        ("Linux paths (/…)".into(), addr == "linux", if cdrive && addr != "linux" { save("addressBar", json!("linux")) } else { None }),
    ])));
    c = c.push(srow(&p, G::Grid, "Apps you install", "Apps from Flatpak use Hearth's Open and Save window, with Hearth (C:). Text an app draws itself keeps its own paths.", ui::chip("on", Pal::a(p.good, 0.18), p.good)));
    c = c.push(srow(&p, G::Grid, "Terminal speaks C:\\", "dir, copy, del, cd C:\\Users… in every terminal (Linux commands still work). Applies to new terminal windows", switch(&p, term, save("terminalWindows", json!(!term)))));
    c = c.push(srow(&p, G::Link, "Copy path gives", "The other one is always in the right-click menu", seg(&p, vec![
        (format!("C:\\Users\\{}\\Documents", if user.is_empty() { "you".to_string() } else { user.clone() }), copy == "windows", if copy != "windows" { save("copyPath", json!("windows")) } else { None }),
        (format!("{home}/Documents"), copy == "linux", if copy != "linux" { save("copyPath", json!("linux")) } else { None }),
    ])));
    c = c.push(head(&p, "Drive letters"));
    c = c.push(value_row(&p, G::Drive, "C:  Hearth", "The drive Hearth runs from · always C:", "Always C:", false));
    let mut all: Vec<&Value> = va(pc, "drives").iter().chain(va(pc, "network").iter()).filter(|d| !vs(d, "letter").is_empty() && vs(d, "letter") != "C").collect();
    all.sort_by_key(|d| vs(d, "letter"));
    let taken = at(pc, "taken");
    for d in &all {
        let letter = vs(d, "letter");
        let key = vs(d, "key");
        let info = if at(d, "mounted").as_bool() == Some(false) {
            format!("{} · keeps {letter}: for when it's back", if vs(d, "sub").is_empty() { "Not plugged in".to_string() } else { vs(d, "sub") })
        } else if vf(d, "total", 0.0) > 0.0 && present(d, "free") {
            format!("{} free of {}", ui::bytes(vf(d, "free", 0.0)), ui::bytes(vf(d, "total", 0.0)))
        } else {
            vs(d, "sub")
        };
        let mut used: Vec<String> = taken.as_object().map(|m| m.keys().cloned().collect()).unwrap_or_default();
        used.extend(all.iter().filter(|q| vs(q, "key") != key).map(|q| vs(q, "letter")));
        let free: Vec<(String, String)> = "DEFGHIJKLMNOPQRSTUVWXY".chars().map(|l| l.to_string()).filter(|l| *l == letter || !used.contains(l)).map(|l| (l.clone(), format!("{l}:"))).collect();
        let name = vs(d, "name");
        let k2 = key.clone();
        let n2 = name.clone();
        c = c.push(chooser(a, &format!("letter:{key}"), if vs(d, "kind") == "network" { G::Server } else { G::Drive }, format!("{letter}:  {name}"), info, free, &letter, move |l| {
            Msg::Exec(Ex::new(py_s("hearth-files", vec!["letter-set".into(), k2.clone(), l.clone()])).re("thispc").ok(format!("\u{201c}{n2}\u{201d} is {l}: now (Windows apps see it there from their next start)")).fail("Couldn't change the letter"))
        }));
    }
    if all.is_empty() {
        c = c.push(line(&p, "Other drives get the next free letter (D: to Y:) the first time they're plugged in, and keep it."));
    } else {
        c = c.push(line(&p, "Only free letters. Z: is how Windows apps see the whole system, so it's left alone."));
    }
    c = c.push(info_row(&p, G::Info, "Only names", "These only change how Hearth names things. Your files, apps and games stay exactly where they are, and Windows apps keep their own C: inside them."));
    c.into()
}

// ------------------------------------------------------------------ Your desktop (Fedora edition)

/// Your desktop on the Fedora edition: Desktop Mode is KDE Plasma, with Firehold on Plasma (a test) when it's in the
/// image (Firehold (Rust, test) was archived on Nov 19) (Mantel isn't in this image, so its window manager choice isn't offered: hearth-your-desktop is Mantel's).
fn your_desktop(a: &App) -> El<'_> {
    let p = a.sh.p;
    let st = &a.system;
    let d = &st.desk;
    // 1.17 (the owner, Nov 21): "Put the default Desktop enviroment as Hearth Clasic labwc", and "hide Fire hold and KDE,
    // dont have to uninstall them just hide them": Mantel is the one choice shown where labwc is in the image. KDE
    // Plasma and Firehold stay installed (and a device that picked one keeps it); KDE Plasma is still the safe start.
    // 1.18 (the owner, Nov 23: "only 2 versions i want are Mantel and the Rust version of it, add a switcher in the
    // Setting"): on Debian, Mantel or Mantel in Rust (marked Rust), both on labwc.
    if !d.mantel.is_empty() {
        let mut c = Column::new().spacing(10.0).push(t("What Desktop Mode opens. Nothing changes until you switch to Desktop Mode again.", sz(14.0), p.muted));
        for (id, name, sub) in [("mantel", "Mantel", "Hearth's own desktop with all of Hearth's apps, built into Hearth OS"), ("rust", "Mantel in Rust", "The same desktop written in Rust, from the Hearth Store")] {
            let on = d.mantel == id;
            let here = id == "mantel" || d.rust_here;
            let label = if id == "rust" { format!("{name}  ·  Rust") } else { name.to_string() };
            let right: El<'_> = if on { t1("Picked", sz(13.0), p.accent).into() }
                else if !here { t1("Get it in the Hearth Store", sz(13.0), p.muted).into() }
                else { btn(&p, "Use", Kind::Normal, false, Some(Msg::Desktop(id))) };
            c = c.push(srow(&p, G::Grid, label, sub, right));
        }
        if st.desk_saved {
            c = c.push(card(&p, row![
                column![ts("Saved", sz(15.0), p.text), t("It opens the next time you switch to Desktop Mode. Switching now closes open apps.", sz(13.0), p.muted)].spacing(2.0).width(Length::Fill),
                btn(&p, "Switch now", Kind::Primary, false, Some(Msg::SwitchNow))
            ].spacing(10.0).align_y(iced::Center)));
        }
        c = c.push(value_row(&p, G::Shield, "Safe start", "If Mantel in Rust closes right after it starts, Mantel opens instead for that sign-in. Your choice stays saved.", "always on", true));
        return c.into();
    }
    let mut opts: Vec<(&'static str, &'static str, &'static str)> = Vec::new();
    if d.labwc {
        opts.push(("classic", "Mantel (Hearth Classic)", "Hearth's own desktop: the bar, the dock, Quick Access and all of Hearth's apps (the default)"));
    } else {
        opts.push(("plasma", "KDE Plasma", "The desktop, with Hearth's look"));
        if d.plasma_fh {
            opts.push(("firehold-plasma", "Firehold (Plasma, test)", "Firehold's Couch and Desk on KDE Plasma"));
        }
    }
    let mut c = Column::new().spacing(10.0).push(t("What Desktop Mode opens. Nothing changes until you switch to Desktop Mode again.", sz(14.0), p.muted));
    // A device that picked a desktop that's now hidden keeps it (red flag 9), and says so here.
    let hidden_now = match d.chosen.as_str() { "plasma" if d.labwc => Some("KDE Plasma"), "firehold-plasma" if d.labwc => Some("Firehold on Plasma"), "firehold-labwc" => Some("Firehold on labwc"), _ => None };
    if let Some(n) = hidden_now {
        c = c.push(info_row(&p, G::Info, "Right now", format!("This PC opens {n}, your earlier pick. Use Mantel to switch to Hearth Classic.")));
    }
    for (id, name, sub) in opts {
        let on = d.chosen == id;
        c = c.push(srow(&p, G::Grid, name, sub, if on { t1("Picked", sz(13.0), p.accent).into() } else { btn(&p, "Use", Kind::Normal, false, Some(Msg::Desktop(id))) }));
    }
    if st.desk_saved {
        c = c.push(card(&p, row![
            column![ts("Saved", sz(15.0), p.text), t("It opens the next time you switch to Desktop Mode. Switching now closes open apps.", sz(13.0), p.muted)].spacing(2.0).width(Length::Fill),
            btn(&p, "Switch now", Kind::Primary, false, Some(Msg::SwitchNow))
        ].spacing(10.0).align_y(iced::Center)));
    }
    c = c.push(value_row(&p, G::Shield, "Safe start", "If Mantel doesn't start, KDE Plasma opens for that sign-in and Hearth tells you why. Your choice stays saved.", "always on", true));
    // >>> labwc on the Fedora edition
    if d.labwc && !d.labwc_missing.is_empty() {
        c = c.push(info_row(&p, G::Info, "Mantel can't start on this PC yet", format!("This Hearth OS is missing {}. If you pick it, KDE Plasma opens instead and tells you why.", d.labwc_missing)));
    }
    if !d.labwc {
        c = c.push(value_row(&p, G::Grid, "Mantel (Hearth Classic)", "Hearth's own desktop is on the Debian edition; this edition's desktop is KDE Plasma", "Debian edition", false));
    }
    // <<< labwc on the Fedora edition
    c.into()
}

// ------------------------------------------------------------------ Boot splash (SetBoot.qml's "splash")

/// Each splash's card: its name and its preview sheet (bootpics.rs).
pub fn boot_cards(st: &State) -> Vec<(String, crate::bootpics::Sheet)> {
    va(st.j("boot"), "splash.presets").iter().filter_map(|s| crate::bootpics::sheet(s).map(|sh| (vs(s, "label"), sh))).collect()
}

/// The sheet that plays now on Boot splash (the preview's, or the card under the mouse or the focus).
pub fn boot_active(st: &State) -> Option<crate::bootpics::Sheet> {
    let pv = st.boot_pv.as_ref().and_then(crate::bootpics::sheet);
    crate::bootpics::active(pv, &st.flip, &boot_cards(st))
}

/// The posters of the splashes, read once the list is.
pub fn boot_posters(st: &State) -> Task<Msg> {
    let Some(g) = crate::bootpics::geo(st.j("boot")) else { return Task::none() };
    match crate::bootpics::load_posters(boot_cards(st).into_iter().map(|c| c.1).collect(), g) {
        Some(f) => Task::perform(f, |_| Msg::BootPics),
        None => Task::none(),
    }
}

/// Boot splash (SetBoot.qml's "splash" part with SetBootCard.qml): each splash as a card with its picture (playing
/// while the mouse or the focus is on it); a card opens its preview (boot_preview), and only Use this there chooses.
fn boot(a: &App) -> El<'_> {
    let p = a.sh.p;
    let st = &a.system;
    let b = st.j("boot");
    let mut c = Column::new().spacing(10.0);
    c = c.push(head(&p, if st.busy.is_empty() { "Boot splash" } else { "Boot splash · saving…" }));
    c = c.push(t("What the screen shows while Hearth OS starts. Pick one to see it play; it changes only when you choose Use this, and takes effect from the next start. If a splash can't be shown, Hearth OS starts in text as before.", sz(12.0), p.muted));
    if let Some(w) = waiting(a, "boot", "the boot splashes") {
        c = c.push(w);
        return c.into();
    }
    let current = vs(b, "splash.current");
    let movies = va(b, "movie.list");
    for s in va(b, "splash.presets") {
        let pair = vs(s, "pair");
        let pair_label = movies.iter().find(|m| vs(m, "id") == pair).map(|m| format!("Goes with the {} startup animation", vs(m, "label"))).unwrap_or_default();
        c = c.push(crate::cats::gaming::boot::boot_card(&p, &st.flip, s, vs(s, "id") == current, pair_label, Msg::BootPreview(Some(s.clone())), Msg::BootHover, Msg::BootUnhover));
    }
    c = c.push(link_row(&p, G::Pad, "Startup animation (Steam Game Mode)", "Gaming › Startup animation", Msg::Go("startup-animation")));
    c.into()
}

/// A boot splash's preview (SetBootPreview.qml) over the page: it plays; Use this, Use with its startup animation
/// (when that one is there), Close. Looking changes nothing.
pub fn boot_preview(a: &App) -> Option<(El<'_>, Option<Msg>)> {
    let st = &a.system;
    let item = st.boot_pv.as_ref()?;
    let p = a.sh.p;
    let b = st.j("boot");
    let id = vs(item, "id");
    let label = vs(item, "label");
    let in_use = vs(b, "splash.current") == id;
    let can_change = vb(b, "splash.canChange");
    let can_use = st.busy.is_empty() && can_change && !matches!(item.get("available"), Some(Value::Bool(false)));
    let pair = vs(item, "pair");
    let pair_ok = !pair.is_empty() && va(b, "movie.list").iter().any(|m| vs(m, "id") == pair);
    let splash_cmd = adm("hearth-admin", vec!["boot-splash".into(), id.clone()]);
    let mut bs: Vec<El> = Vec::new();
    if !in_use {
        bs.push(btn(&p, "Use this", Kind::Primary, false, if can_use { Some(Msg::Seq(vec![Msg::BootPreview(None), Msg::Exec(Ex::new(splash_cmd.clone()).re("boot").ok(format!("{label} shows from the next start.")).fail("Couldn't change the boot splash"))])) } else { None }));
    }
    if can_use && pair_ok {
        let movie = lx("hearth-boot-look", &["movie", &pair]);
        let ex = if in_use { Ex::new(movie) } else { Ex::new(splash_cmd.clone()).and(movie) };
        bs.push(btn(&p, "Use with its startup animation", Kind::Normal, false, Some(Msg::Seq(vec![Msg::BootPreview(None), Msg::Exec(ex.re("boot").ok(format!("{label}: the splash shows from the next start, and the startup animation plays the next time Steam Game Mode starts.")).fail("Couldn't use them as a set"))]))));
    }
    bs.push(btn(&p, "Close", Kind::Normal, false, Some(Msg::BootPreview(None))));
    let note_line = if in_use {
        ("This is the one in use.", p.flame)
    } else if !can_change {
        ("The boot splash can't be changed on this edition: this is only a look.", p.muted)
    } else {
        ("Only a preview: nothing changes until you choose Use this.", p.muted)
    };
    let body = column![
        ui::tb(format!("Boot splash: {label}"), sz(20.0), p.text),
        container(crate::cats::gaming::boot::preview_screen(&p, &st.flip, item, 640.0)).center_x(Length::Fill),
        t(format!("{}. On your screen it fills the screen and keeps going until Hearth OS has started; drawn here from the splash's own script.", vs(item, "about").trim_end_matches('.')), sz(13.0), p.muted),
        t(note_line.0, sz(12.0), note_line.1),
        Row::with_children(bs).spacing(10.0),
    ]
    .spacing(14.0);
    Some((ui::dialog_box(&p, 680.0, body), Some(Msg::BootPreview(None))))
}

// ------------------------------------------------------------------ Remote desktop (hearth-vnc)

fn remote(a: &App) -> El<'_> {
    let p = a.sh.p;
    let st = &a.system;
    let v = st.j("vnc");
    let mut c = Column::new().spacing(10.0);
    if let Some(w) = waiting(a, "vnc", "Remote desktop") {
        c = c.push(w);
        if st.read("vnc") {
            return c.into();
        }
    }
    if !st.has("vnc") {
        return c.into();
    }
    let on = vb(v, "on");
    let set = |k: &str, val: &str, said: &str| Msg::Exec(Ex::new(py_s("hearth-vnc", vec!["set".into(), k.into(), val.into()])).re("vnc").ok(said).fail("Couldn't change Remote desktop").secs(60));
    let sub = if on { if vb(v, "running") { "On: open a VNC app on your other device and connect to this PC" } else { "On, but the server isn't running. Turn it off and on again" } } else { "Use this PC's desktop from a VNC app on your laptop, tablet or phone, through Tailscale. Off until you turn it on" };
    c = c.push(srow(&p, G::Display, "Remote desktop", sub, sw(&p, on, if on { set("on", "off", "Off. Nobody can connect.") } else { set("on", "on", "On. The first time someone connects, this PC asks you to allow sharing the screen.") })));
    if !on {
        c = c.push(link_row(&p, G::Play, "Games and low-latency streaming", "For playing this PC's games on a TV, phone or another PC, Remote play (Moonlight) is faster", Msg::Go("remote")));
        return c.into();
    }
    for tip in va(v, "tips") {
        c = c.push(note(&p, format!("{}.", tip.as_str().unwrap_or("")), true));
    }
    let clients = va(v, "clients");
    if !clients.is_empty() {
        c = c.push(head(&p, "Connected now"));
        for cl in clients {
            let since = vf(cl, "since", 0.0);
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0);
            let mins = ((now - since) / 60.0).round().max(0.0) as i64;
            let dur = if mins < 1 { "just now".to_string() } else if mins < 60 { format!("{mins} min") } else { format!("{} h {} min", mins / 60, mins % 60) };
            let via = match vs(cl, "via").as_str() { "tailscale" => "Through Tailscale", "home" => "On your home network", _ => "On this PC" };
            let who = if vs(cl, "name").is_empty() { vs(cl, "ip") } else { vs(cl, "name") };
            c = c.push(act_row(&p, G::Users, format!("{who}{}", if vb(cl, "viewOnly") { " (view only)" } else { "" }), format!("{via} · for {dur}"), "Disconnect", Some(Msg::Exec(Ex::new(py_s("hearth-vnc", vec!["disconnect".into(), vs(cl, "id")])).re("vnc").ok("Disconnected.")))));
        }
    }
    c = c.push(head(&p, "Connect from another device"));
    let ts_name = { let n = vs(v, "addresses.tailscale.name"); if n.is_empty() { vs(v, "addresses.tailscale.ip") } else { n } };
    let port = vf(v, "port", 5900.0) as i64;
    if !ts_name.is_empty() {
        c = c.push(info_row(&p, G::Wifi, format!("In your VNC app, connect to {ts_name}{}", if port != 5900 { format!(":{port}") } else { String::new() }), "Sign in to Tailscale on that device with the same account first. Pick VeNCrypt or TLS if the app asks, and use the password below"));
    } else {
        c = c.push(link_row(&p, G::Wifi, "Tailscale isn't connected", "Your other devices reach this PC through Tailscale. Sign in here and on the other device", Msg::Go("hs-tailscale")));
    }
    let home: Vec<String> = va(v, "addresses.home").iter().map(|x| x.as_str().unwrap_or("").to_string()).collect();
    if vs(v, "reach") == "home" && !home.is_empty() {
        c = c.push(info_row(&p, G::Home, format!("At home: {}", home.join(" or ")), "Devices on your home network can connect there too"));
    }
    let fp = vs(v, "fingerprint");
    if !fp.is_empty() {
        c = c.push(act_row(&p, G::Shield, "This PC's certificate (SHA-256)", format!("If your VNC app asks whether to trust this PC, check it shows: {fp}"), "Copy", Some(Msg::App(sv(&["wl-copy", "--", &fp])))));
    }
    c = c.push(head(&p, "Password"));
    let keyring = vs(v, "keyring");
    let has_pw = vb(v, "password");
    let pw_sub = if keyring == "locked" { "Your keyring is locked: sign out and in again, then set it" } else if keyring == "missing" { "The keyring tool (secret-tool) isn't installed, so a password can't be kept" } else if has_pw { "Kept in your keyring, never in a file. Every connection needs it" } else { "Nobody can connect until there is one: at least 8 characters" };
    let pw_open = st.on("pwopen");
    c = c.push(act_row(&p, G::Shield, if has_pw { "Change the password" } else { "Set a password" }, pw_sub, if pw_open { "Close" } else { "Change" }, Some(Msg::Flag("pwopen", !pw_open))));
    if pw_open {
        let typed = st.txt("vncpw").to_string();
        let submit = if typed.chars().count() >= 8 { Msg::Exec(Ex::new(py("hearth-vnc", &["password"])).input(typed.clone()).re("vnc").ok("The password is set.").fail("Couldn't set the password")) } else { Msg::Text("vncpw", typed.clone()) };
        c = c.push(row![field(&p, "The new password (at least 8 characters), then Enter", &typed, |s| Msg::Text("vncpw", s)).secure(true).on_submit(submit).width(Length::Fixed(sz(380.0))), t1(if !typed.is_empty() && typed.chars().count() < 8 { "Use at least 8 characters" } else { "" }, sz(12.0), p.danger)].spacing(12.0).align_y(iced::Center));
    }
    c = c.push(head(&p, "Who can connect and what they can do"));
    let reach = vs(v, "reach");
    c = c.push(srow(&p, G::Wifi, "Reachable", if reach == "home" { "Never from the internet. Anyone on your home network or Wi-Fi can reach its sign-in (they still need the password)" } else { "Never from the internet. Tailscale only keeps it closed to your home network too" }, choice(&p, vec![
        ("Tailscale only".into(), reach != "home", set("reach", "tailscale", "Only through Tailscale now: anyone connected from the home network was disconnected.")),
        ("Tailscale and home network".into(), reach == "home", set("reach", "home", "Your home network can connect too.")),
    ])));
    let view_only = vb(v, "viewOnly");
    let input_line = if view_only { "View only: they see the screen but can't type or click".to_string() } else {
        match vs(v, "input").as_str() {
            "portal" => "They can use the keyboard and mouse, through the desktop's own remote control".into(),
            "uinput" => "They can use the keyboard and mouse (a virtual keyboard and mouse: this desktop has no remote control of its own)".into(),
            _ => format!("View only for now: {}", if vs(v, "inputWhy").is_empty() { "this desktop can't take typing from another device".to_string() } else { vs(v, "inputWhy") }),
        }
    };
    c = c.push(srow(&p, G::Display, "View only", input_line, sw(&p, view_only, set("viewOnly", if view_only { "off" } else { "on" }, if view_only { "They can use the keyboard and mouse again." } else { "View only: nobody can type or click from another device." }))));
    if clients.len() > 1 {
        c = c.push(act_row(&p, G::Close, "Disconnect everyone", "", "Disconnect", Some(Msg::Exec(Ex::new(py("hearth-vnc", &["disconnect", "all"])).re("vnc").ok("Everyone is disconnected.")))));
    }
    c = c.push(link_row(&p, G::Play, "Games and low-latency streaming", "For playing this PC's games on a TV, phone or another PC, Remote play (Moonlight) is faster", Msg::Go("remote")));
    c = c.push(line(&p, "Every connection is encrypted (TLS) and shows a notification while it lasts. Five wrong passwords lock that device out for five minutes, and many from anywhere pause new connections for ten. The first time, this PC asks you to allow sharing its screen; on KDE's desktop that answer is remembered."));
    c.into()
}
