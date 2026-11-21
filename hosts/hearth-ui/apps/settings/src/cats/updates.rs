//! Updates & recovery (SetUpdates.qml) and Backup and help (SetAbout.qml's "backup" part). hearth-admin through
//! pkexec (Hearth's polkit rule lets the person at the PC do it without a password) for the status, looking for an
//! update, updating (its progress line by line) and going back; a restore point first (hearth_restore.py, in the
//! background so it can never stop or slow the update); "Shut down when it's done" (only for this update, nothing
//! saved); the writable system where hearth-unlock is (Debian's Peppermint; the Fedora edition has none, so that part
//! doesn't show); Hearth's Decky plugins (hearth-decky as you; plugin-updates.json through hearth_settings_io.py);
//! System Restore and Reliability history (the pure-Rust apps). The running is cats/hs/kit.rs's.

use crate::cats::hs::kit::{self, *};
use crate::shared::Shared;
use crate::widgets::sz;
use crate::App;
use hearth_ui::run;
use hearth_ui::ui::{self, glyph, t, ts, G};
use hearth_ui::Pal;
use iced::widget::{column, container, Row};
use iced::{Element, Length, Subscription, Task};
use serde_json::{json, Value};

pub const PAGES: &[&str] = &["updates", "backup"];

pub type State = St;
pub type Msg = kit::Msg;

fn reads(page: &str, _st: &St) -> Vec<Rd> {
    if page == "backup" {
        return vec![rd("backup", H::Nimbus, &["--settings-call", "backup.status", "{}"])];
    }
    let decky = format!("[ -f {DECKY} ] && exec python3 {DECKY} plugins-status || echo null");
    let adm = format!("[ -x {ADMIN} ] && echo '{{\"ok\": true}}' || echo '{{\"ok\": false}}'");
    vec![
        rd("adm", H::Sh, &[adm.as_str()]),
        rd("unlock", H::Sh, &["[ -x /usr/bin/hearth-unlock ] && exec /usr/bin/hearth-unlock --status --json || echo null"]),
        rd("plugins", H::Sh, &[decky.as_str()]),
    ]
}

fn follow(_page: &str, slot: &str, st: &St) -> Vec<Rd> {
    if slot == "adm" && b(st.get("adm"), "ok") {
        return vec![rd("u", H::Admin, &["update-status"])];
    }
    Vec::new()
}

fn live(_page: &str, st: &St) -> bool {
    st.prog.running || st.l.contains_key("offAt")
}

pub const HOOKS: Hooks = Hooks { reads, follow, submit: no_submit, live };

pub fn open(sh: &mut Shared, st: &mut State, page: &'static str) -> Task<Msg> {
    let part = std::mem::take(&mut sh.part);
    kit::open(st, page, &part, &HOOKS)
}

pub fn update(sh: &mut Shared, st: &mut State, m: Msg) -> Task<Msg> {
    kit::update(sh, st, m, &HOOKS)
}

/// From 1.0 on Hearth OS is MAJOR.MINOR: 1.0.0 shows as "1.0".
fn ver(i: &Value) -> String {
    let v = s(i, "version");
    if !v.is_empty() {
        return crate::data::shown_version(&v);
    }
    s(i, "timestamp").chars().take(10).collect()
}

/// Something that ran: its restore point first (when updating), then hearth-admin, line by line.
fn admin(cmd: &str, st: &St) -> Act {
    let u = st.get("u");
    let a = act(H::Admin, &[cmd]).stream().secs(if cmd == "update" { 4 * 3600 } else { 600 });
    let a = if cmd == "rollback" { a } else { a.store("u") };
    if cmd != "update" {
        return a;
    }
    let a = if st.ls("after") == "shutdown" { a.set("offAt", json!("@now+60")) } else { a };
    let to = ver(at(u, "available"));
    let rp = run::shell_helper("hearth_restore.py");
    let script = format!("python3 {} snapshot --kind update{} >/dev/null 2>&1 </dev/null & echo '{{}}'", crate::data::q(&rp), if to.is_empty() { String::new() } else { format!(" --to {}", crate::data::q(&to)) });
    act_v(H::Sh, vec![script]).no_reload().then(a)
}

fn unlock_set(action: &str) -> Act {
    let a = act(H::Admin, &["unlock", action]).store("unlock").secs(600);
    if action == "on" || action == "off" {
        let rp = run::shell_helper("hearth_restore.py");
        let title = if action == "on" { "Before unlocking the system" } else { "Before locking the system" };
        let script = format!("python3 {} snapshot --kind automatic --title {} >/dev/null 2>&1 </dev/null & echo '{{}}'", crate::data::q(&rp), crate::data::q(title));
        return act_v(H::Sh, vec![script]).no_reload().then(a);
    }
    a
}

fn plugins_run(cmd: &str) -> Msg {
    let wait = match cmd {
        "plugins-check" => "Looking for plugin updates…",
        "plugins-update" => "Updating the plugins…",
        _ => "Going back to the built-in plugins…",
    };
    act(H::Decky, &[cmd]).store("pluginsAns").set("pcmd", json!(cmd)).wait(wait).secs(900).msg()
}

fn plugin_name(k: &str) -> &str {
    match k {
        "decky-hearth-sync" => "Hearth Sync",
        "decky-hearth-link" => "Hearth Link",
        "decky-hearth-music" => "Hearth Music",
        _ => "",
    }
}

fn updates_page<'a>(a: &'a App) -> E<'a, Msg> {
    let p = a.sh.p;
    let st = &a.updates;
    let can = b(st.get("adm"), "ok");
    let u = st.get("u");
    let pr = &st.prog;
    let phase = if pr.running { if pr.what == "update" { "updating" } else if pr.what == "update-check" { "checking" } else { "busy" } } else { "idle" };
    let staged = b(u, "staged");
    let available = b(u, "available") && !staged;
    let mut rs = Rows::new();

    // The status card: where Hearth stands and the one thing to do next.
    let action: Option<(&str, Msg)> = if !can || phase != "idle" {
        None
    } else if staged {
        Some(("Restart now", Msg::Detached(sv(&["systemctl", "reboot"]))))
    } else if available {
        Some(("Download", admin("update", st).msg()))
    } else {
        Some(("Look for updates", admin("update-check", st).msg()))
    };
    let title = if !can {
        "Updates come with Hearth OS".to_string()
    } else if phase == "updating" {
        "Downloading the update…".into()
    } else if phase == "checking" {
        "Looking for updates…".into()
    } else if staged {
        format!("Hearth {} is ready. Restart to finish updating.", ver(at(u, "staged")))
    } else if available {
        format!("Hearth {} is ready to download", ver(at(u, "available")))
    } else if u.is_object() && !b(u, "error") {
        "You're up to date".into()
    } else {
        "Hearth".into()
    };
    let sub = if !can {
        "System updates are made here on Hearth, the OS.".to_string()
    } else if available && phase == "idle" {
        "Keeps working while it downloads; finishes when you restart".into()
    } else {
        let img = s(u, "booted.image");
        kit::join(&[format!("Hearth OS {}", ver(at(u, "booted"))), if img.is_empty() { "Running now".into() } else { img }], " · ")
    };
    let ic = if staged || available || phase != "idle" { G::Update } else if can { G::Check } else { G::Info };
    let mut hero = Row::new().spacing(18.0).align_y(iced::Center).push(container(glyph(ic, p.accent, 28.0, 2.2)).center_x(Length::Fixed(56.0)).center_y(Length::Fixed(56.0)).style(ui::boxed(Pal::mix(p.panel, p.accent, 0.18), p.r(12.0), None))).push(column![ts(title, sz(19.0), p.text), t(sub, sz(13.0), p.muted)].spacing(3.0).width(Length::Fill));
    if let Some((label, m)) = action.clone() {
        hero = hero.push(crate::widgets::btn(&p, label.to_string(), ui::Kind::Primary, false, Some(m)));
    }
    let hero_c = container(hero).padding(20.0).width(Length::Fill).style(ui::boxed(p.panel, p.r(12.0), None));
    rs.add(match action {
        Some((_, m)) => iced::widget::mouse_area(hero_c).on_press(m).interaction(iced::mouse::Interaction::Pointer).into(),
        None => hero_c.into(),
    });

    // Progress.
    if phase != "idle" {
        let line = if pr.line.is_empty() { if phase == "updating" { "Starting…".to_string() } else if phase == "checking" { "Looking for updates…".into() } else { String::new() } } else { pr.line.clone() };
        let mut top = Row::new().push(t(line, sz(14.0), p.text).width(Length::Fill));
        if pr.pct >= 0.0 {
            top = top.push(ts(format!("{}%", (pr.pct * 100.0).floor()), sz(14.0), p.accent));
        }
        // No moving bar while it isn't known how far it is: a third of the bar, still (nothing redraws at rest).
        let frac = if pr.pct >= 0.0 { pr.pct as f64 } else { 0.3 };
        rs.add(column![top, meter(&p, frac, false)].spacing(8.0).into());
    }
    if !pr.error.is_empty() {
        rs.add(words_c(pr.error.clone(), p.danger));
    } else if b(u, "error") && can {
        rs.add(words_c(s(u, "error"), p.danger));
    }

    // Shut down after updating (only for this update: nothing is saved).
    if can && (phase == "updating" || (available && phase == "idle")) {
        let on = st.ls("after") == "shutdown";
        rs.add(r("power", "Shut down when it's done").sub("Downloads the update, then turns the PC off; it starts on the new version next time").switch(&p, on, Some(lv("after", if on { Value::Null } else { json!("shutdown") }))).el(&p));
    }
    let off_in = if st.l.contains_key("offAt") { (st.ln("offAt") - hearth_ui::time::now() as f64).max(0.0) } else { -1.0 };
    if can && staged && phase == "idle" && off_in < 0.0 {
        rs.add(r("power", "Shut down and finish updating").sub(format!("Turns the PC off now; it starts on Hearth {} next time", ver(at(u, "staged")))).on(Msg::Detached(sv(&["systemctl", "poweroff"]))).el(&p));
    }
    if off_in >= 0.0 {
        rs.add(banner(&p, "info", "power", format!("Updated. Shutting down in {off_in} s"), "Save anything open. Cancel keeps the PC on; the update finishes whenever it next turns off or restarts", "Cancel", Some(Msg::Locals(vec![("offAt".into(), Value::Null), ("after".into(), Value::Null)]))));
    }
    let plugins = st.get("plugins");
    let idle_up = can && phase == "idle" && !available && !staged;
    // Where updates come from (hearth-admin's "channel", 1.16): Hearth OS releases (:stable), or a test tag. A PC on
    // a test tag whose version is out as a release moves to the releases with its next update, by itself.
    let ch = at(u, "channel");
    let kind = s(ch, "channel");
    let show_ch = can && (kind == "stable" || kind == "test");
    if idle_up || plugins.is_object() || show_ch {
        rs.add(h(&p, "Updates"));
    }
    if idle_up {
        rs.add(r("update", "Update anyway").sub("Downloads anything new for Hearth and its apps, even small fixes").on(admin("update", st).msg()).el(&p));
    }
    if show_ch {
        let tag = s(ch, "following").rsplit(':').next().unwrap_or("").to_string();
        let rel = s(ch, "release");
        let sub = if kind == "stable" {
            "Hearth OS releases: every update is a tested release, and the version before always stays to go back to".to_string()
        } else if !s(ch, "moveTo").is_empty() && !rel.is_empty() {
            format!("Test builds ({tag}). Hearth {} is out: the next update moves this PC to Hearth OS releases, and the version you're on stays in the boot menu to go back to", crate::data::shown_version(&rel))
        } else {
            format!("Test builds ({tag}). Once this version is released, the next update moves this PC to Hearth OS releases by itself")
        };
        rs.add(r("update", "Updates from").sub(sub).value(if kind == "stable" { "Releases" } else { "Test builds" }).el(&p));
    }

    // Decky plugins.
    if plugins.is_object() {
        let avail = keys(plugins, "available").len();
        let news = keys(plugins, "new");
        let ans = st.get("pluginsAns");
        let pcmd = st.ls("pcmd");
        let pline = if st.busy > 0 && !st.msg.is_empty() && pcmd.is_empty() {
            String::new()
        } else if ans.is_object() && !b(ans, "error") && !pcmd.is_empty() {
            if b(ans, "restarted") {
                "Done. Decky restarted with the new plugins.".to_string()
            } else if b(ans, "applies") {
                format!("Ready. Put in place {}.", s(ans, "applies"))
            } else if pcmd == "plugins-check" {
                if !keys(ans, "available").is_empty() { "Plugin updates are ready to install.".into() } else if !keys(ans, "new").is_empty() { "The plugins are up to date. New plugins are listed below.".into() } else { "The plugins are up to date.".into() }
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        let mut inner = Vec::new();
        for k in keys(plugins, "plugins") {
            let pl = at(plugins, &format!("plugins.{k}"));
            let name = if !plugin_name(&k).is_empty() { plugin_name(&k).to_string() } else if !s(pl, "name").is_empty() { s(pl, "name") } else { k.clone() };
            let src = s(pl, "source");
            let mut sub = if src == "downloaded" { if b(pl, "builtin") { format!("Updated (built-in: {})", s(pl, "builtin")) } else { "Added from Hearth".into() } } else if src == "built-in" { "Built in".to_string() } else { "Not installed yet".into() };
            let av = s(plugins, &format!("available.{k}"));
            if !av.is_empty() {
                sub.push_str(&format!(" · {av} available"));
            }
            if !s(pl, "downloaded").is_empty() && s(pl, "downloaded") != s(pl, "installed") {
                sub.push_str(&format!(" · {} ready for the next Steam Game Mode start", s(pl, "downloaded")));
            }
            inner.push(r("update", format!("{name} {}", if s(pl, "installed").is_empty() { s(pl, "builtin") } else { s(pl, "installed") })).sub(sub).el(&p));
        }
        if !news.is_empty() {
            inner.push(h(&p, "New from Hearth"));
            for k in &news {
                let nw = at(plugins, &format!("new.{k}"));
                inner.push(r("update", format!("{} {}", if s(nw, "name").is_empty() { k.clone() } else { s(nw, "name") }, s(nw, "version"))).sub(format!("{}{}", if s(nw, "description").is_empty() { String::new() } else { format!("{} · ", s(nw, "description")) }, if b(plugins, "installNew") { "Installed with the next update" } else { "Tick \"Also install new Hearth plugins\" to get it" })).el(&p));
            }
        }
        let writer = |k: &str, v: bool| {
            let path = hearth_ui::paths::home().join(".config/nimbus/plugin-updates.json").to_string_lossy().to_string();
            let mut m = serde_json::Map::new();
            m.insert(k.to_string(), Value::Bool(v));
            act_v(H::Io, vec!["merge".into(), path, Value::Object(m).to_string(), "--history".into()]).msg()
        };
        let inst_new = b(plugins, "installNew");
        inner.push(r("update", "Also install new Hearth plugins").sub("Update (and automatic updates, when on) also installs new plugins from Hearth, checked the same way. Off: they're only listed").switch(&p, inst_new, Some(writer("installNew", !inst_new))).el(&p));
        let checked = n(plugins, "lastCheck");
        let csub = if !pline.is_empty() { pline } else if checked > 0.0 { format!("Last checked {}", when(checked)) } else { "Not checked yet".into() };
        inner.push(r("restart", "Check now").sub(csub).on(plugins_run("plugins-check")).el(&p));
        inner.push(r("update", if avail > 0 { "Update the plugins" } else { "Update" }).sub("Downloads and checks newer plugins; Decky restarts with them if Steam Game Mode is on (Steam keeps running)").on(plugins_run("plugins-update")).el(&p));
        let auto = b(plugins, "auto");
        inner.push(r("update", "Update plugins automatically").sub("Checks once a day while online; a new plugin is put in place at the next Steam Game Mode start, or right away when no game is running").switch(&p, auto, Some(writer("auto", !auto))).el(&p));
        inner.push(r("restart", "Use the built-in plugins").sub("The plugins that came with this Hearth OS version come back (those downloads aren't fetched again by themselves)").on(plugins_run("plugins-builtin")).el(&p));
        if !s(plugins, "lastError").is_empty() {
            inner.push(words_c(s(plugins, "lastError"), p.danger));
        }
        let value = if avail > 0 { format!("{avail} {}", if avail == 1 { "update" } else { "updates" }) } else if checked > 0.0 { "Up to date".into() } else { String::new() };
        // `hearth-settings updates:x:decky` opens it.
        rs.add(expander(&p, st, "x:decky", "box", "Decky plugins", "Hearth's plugins for Steam Game Mode, updated without a system update", &value, None, inner));
    }

    // Go back to the version before.
    let can_back = can && phase == "idle" && !ver(at(u, "rollback")).is_empty() && !staged;
    let queued = b(u, "rollbackQueued");
    rs.add(h(&p, "If something broke"));
    if can_back && !queued {
        let ask = st.lb("askBack");
        rs.add(r("restart", format!("Go back to {}", ver(at(u, "rollback")))).sub("If something broke after the last update: the version before this one starts at the next restart. Your games and files stay.").on(toggle(st, "askBack")).el(&p));
        if ask {
            rs.add(r("check", "Yes, go back at the next restart").sub(format!("Hearth updates again by itself later. Changed your mind before restarting? Stay on {} shows up here.", ver(at(u, "booted")))).on(Act { set: vec![("askBack".into(), Value::Null)], ..admin("rollback", st) }.msg()).el(&p));
        }
    }
    if can_back && queued {
        rs.add(r("restart", format!("Restart to go back to {}", ver(at(u, "rollback")))).sub("It starts at the next restart. Your games and files stay.").on(Msg::Detached(sv(&["systemctl", "reboot"]))).el(&p));
        rs.add(r("close", format!("Stay on {}", ver(at(u, "booted")))).sub("Don't go back after all").on(admin("rollback", st).msg()).el(&p));
    }
    rs.add(r("restart", "System Restore").sub("Hearth and its settings back to an earlier restore point. Made before each update, weekly, or by you").chev().on(Msg::Detached(sv(&["hearth-system-restore"]))).el(&p));
    rs.add(r("info", "Reliability history").sub("How Hearth has been running: apps that stopped, Hearth problems, warnings and updates, day by day").chev().on(Msg::Detached(sv(&["hearth-reliability-history"]))).el(&p));
    if can {
        rs.add(r("shield", "The previous version always stays startable").sub("Every update keeps the version before it in the boot menu").chip("Kept", Ck::Ok).el(&p));
    }

    // The writable system (Peppermint's hearth-unlock; nothing on the Fedora edition).
    let un = st.get("unlock");
    if b(un, "available") {
        rs.add(h(&p, "Writable system"));
        if b(un, "notice") {
            rs.add(r("info", s(un, "notice.title")).sub(s(un, "notice.text")).el(&p));
            rs.add(r("check", "OK, hide this").on(unlock_set("dismiss").msg()).el(&p));
        }
        let unlocked = b(un, "unlocked");
        let sub = match s(un, "state").as_str() {
            "unlocked" => "apt and .deb files install anywhere, kept across restarts. What you add comes back by itself after an update.",
            "locks-at-restart" => "Locked from the next restart: the system as the image made it. Your packages stay saved.",
            "unlocks-at-restart" => "Writable from the next restart.",
            "suspended" => "This start is without your changes (safe start).",
            "repairing" => "This start is without your changes while an install that was cut off is finished.",
            _ => "The system is as the image made it. Unlock to install with apt again; your saved packages come back.",
        };
        rs.add(r("lock", "Unlocked system").sub(sub).switch(&p, unlocked, Some(unlock_set(if unlocked { "off" } else { "on" }).msg())).el(&p));
        if b(un, "suspended") && unlocked {
            rs.add(r("restart", "Turn my changes back on").sub("Uses the packages you added again, now. If Hearth then doesn't start, it starts without them again by itself.").on(unlock_set("resume").msg()).el(&p));
        }
        let pk: Vec<String> = arr(un, "packages").iter().map(vs).collect();
        rs.add(r("update", "Your packages").sub(if pk.is_empty() { "Nothing added yet".to_string() } else { pk.join(", ") }).el(&p));
        if b(un, "writable") && (!pk.is_empty() || !arr(un, "removed").is_empty()) {
            rs.add(r("restart", "Put my packages back again").sub("Reinstalls what you added on this version, in the background").on(unlock_set("reapply").msg()).el(&p));
        }
        rs.add(if b(un, "error") { words_c(s(un, "error"), p.danger) } else { words(&p, "If something you install stops Hearth from starting, the next start leaves your changes off by itself, and the previous version in the boot menu keeps its own packages. Want a system that can't be changed at all? That's Hearth OS, Fedora edition.") });
    }
    rs.add(message(&p, st));
    rs.el()
}

fn backup_page<'a>(a: &'a App) -> E<'a, Msg> {
    let p = a.sh.p;
    let st = &a.updates;
    let bk = at(st.get("backup"), "value");
    let mut rs = Rows::new();
    rs.add(h(&p, "Help and logs"));
    rs.add(r("file", "Logs").sub("Read the system's and Hearth's logs, search them, and save or send them").on(Msg::Detached(sv(&["hearth-tools", "--app", "logs"]))).el(&p));
    let sending = st.busy > 0 && st.msg == "Sending…";
    let send = act(H::Nimbus, &["--send-logs", "Sent from Settings › About"]).store("sendtext").wait("Sending…").secs(300).no_reload().fail_set("sendtext", Value::Null);
    rs.add(r("update", if sending { "Sending…" } else { "Send logs to the developer" }).sub("Hearth's and the system's logs, personal details taken out, sent to Hearth OS's bug-report service. You get a short code to share; they're kept 14 days.").on_maybe((!sending).then(|| send.msg())).el(&p));
    let sent = st.get("sendtext");
    if let Some(out) = sent.as_str() {
        let code = out.lines().find_map(|l| l.split("code: ").nth(1)).map(|c| c.split_whitespace().next().unwrap_or("").to_string()).unwrap_or_default();
        rs.add(words(&p, if code.is_empty() { "Couldn't send them: the Hearth app didn't give a code".to_string() } else { format!("Sent. Share this code with the developer: {code}") }));
    }
    rs.add(h(&p, "Backup"));
    for d in arr(bk, "destinations").iter().take(4) {
        let label = s(d, "label");
        let req = json!({ "dest": s(d, "path"), "signins": false }).to_string();
        let a_ = act_v(H::Nimbus, vec!["--settings-call".into(), "backup.create".into(), req]).wait("Backing up…").ok(format!("Backed up to {label}")).secs(1800);
        rs.add(r("folder", format!("Back up Hearth to {label}")).sub("Settings, profiles, favorites, collections, play time and your own games. Store sign-ins stay out.").on_maybe((st.busy == 0).then(|| a_.msg())).el(&p));
    }
    let found = arr(bk, "backups").len();
    rs.add(r("restart", "Restore from a backup").sub(format!("{}On a new device, or after a reinstall. Opens in Hearth Game Mode (Desktop Mode closes first).", if found > 0 { format!("{found} found. ") } else { String::new() })).chev().on(Msg::Detached(sv(&["nimbus-mode", "hearth-page", "system"]))).el(&p));
    let bv = st.get("backup");
    if b(bv, "error") || at(bv, "ok") == &Value::Bool(false) {
        rs.add(words(&p, if b(bv, "error") { s(bv, "error") } else { "The Hearth app couldn't read the backups".into() }));
    }
    rs.add(message(&p, st));
    rs.el()
}

pub fn view<'a>(a: &'a App, page: &'static str) -> Option<Element<'a, Msg>> {
    match page {
        "updates" => Some(updates_page(a)),
        "backup" => Some(backup_page(a)),
        _ => None,
    }
}

pub fn subscription(st: &State) -> Subscription<Msg> {
    kit::subscription(st, &HOOKS)
}

pub fn dialog<'a>(_a: &'a App) -> Option<(Element<'a, Msg>, Option<Msg>)> {
    None
}

pub fn back(_st: &mut State) -> bool {
    false
}
