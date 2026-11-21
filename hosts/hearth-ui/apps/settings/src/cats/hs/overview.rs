//! Home server (SetHomeServer.qml), Set up your home server (SetHomeSetup.qml), Server mode (SetServerMode.qml) and
//! AI engine (SetAIEngine.qml).

use super::kit::*;
use crate::widgets::sz;
use hearth_ui::ui::{self, t, tb, ts};
use hearth_ui::Pal;
use iced::widget::{column, container, row, Column, Row};
use iced::{Length, Padding};
use serde_json::{json, Value};

pub fn reads(page: &str, st: &St) -> Vec<Rd> {
    match page {
        "hs-overview" => vec![rd("status", H::Hs, &["status"]), rd("drives", H::Hs, &["drives"]), rd("appsq", H::Apps, &["quick"])],
        "hs-setup" => {
            let mut v = vec![rd("status", H::Hs, &["status"])];
            if !st.d.contains_key("cands") {
                v.push(rd("cands", H::Hs, &["candidates"]));
            }
            v
        }
        "hs-server" => vec![rd("sm", H::ServerMode, &["status"])],
        _ => vec![rd("col", H::Colibri, &["status"])],
    }
}

pub fn follow(page: &str, slot: &str, st: &St) -> Vec<Rd> {
    let c = st.get("col");
    if page == "hs-setup" {
        // The sign-in QR code, drawn here from qrencode (the same tool the helper makes its picture with).
        let l = st.get("login");
        return if slot == "login" && !s(l, "qr").is_empty() && !s(l, "authUrl").is_empty() {
            vec![Rd { slot: "qrtext".into(), h: H::Sh, args: vec!["qrencode -t ASCII -m 1 -- \"$1\"".into(), "sh".into(), s(l, "authUrl")] }]
        } else {
            Vec::new()
        };
    }
    match slot {
        "col" if b(c, "installed") && !st.d.contains_key("cat") => vec![rd("cat", H::Colibri, &["catalog"])],
        "cat" if b(c, "modelDownloaded") && !s(c, "model").is_empty() => vec![Rd { slot: "plan".into(), h: H::Colibri, args: vec!["plan".into(), s(c, "model")] }],
        _ => Vec::new(),
    }
}

pub fn live(page: &str, st: &St) -> bool {
    match page {
        "hs-setup" => st.ln("step") as i64 == 1 && st.d.contains_key("login") && s(st.get("status"), "tailscale.state") != "Running",
        _ => {
            let c = st.get("col");
            b(c, "installing") || b(c, "download.running")
        }
    }
}

pub fn submit(st: &mut St, key: &str, text: String) -> Result<Vec<Act>, String> {
    match key {
        "name" => {
            let n = text.trim().to_string();
            let ok = !n.is_empty() && n.len() <= 63 && n.chars().next().map(|c| c.is_ascii_alphanumeric()).unwrap_or(false) && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
            if !ok {
                return Err("Use letters, digits and - (no spaces)".into());
            }
            Ok(vec![act_v(H::Admin, vec!["hostname".into(), n.clone()]).ok(format!("Renamed to {n}"))])
        }
        "pin" => {
            let pin = text.trim().to_string();
            if pin.len() < 4 || pin.len() > 12 || !pin.chars().all(|c| c.is_ascii_digit()) {
                st.setl("pinMsg", json!("Use 4 to 12 digits (numbers only)"));
                return Err("Use 4 to 12 digits (numbers only)".into());
            }
            let sm = st.get("sm");
            let on = b(sm, "settings.web.on");
            let done = if on {
                format!("PIN set ✓ Type it on {} on your phone or laptop (over Tailscale)", if s(sm, "url").is_empty() { "the web page".into() } else { s(sm, "url") })
            } else {
                "PIN set ✓ Turn on \"Dashboard web page\" above to use it".to_string()
            };
            Ok(vec![act(H::ServerMode, &["pin-set"]).input(pin).wait("Saving the PIN…").set("pinOpen", Value::Null).set("pinMsg", json!(done.clone())).then(act(H::ServerMode, &["web-apply"]).ok(done))])
        }
        _ => Ok(Vec::new()),
    }
}

// ------------------------------------------------------------------ Home server (the landing)
fn health(drv: &Value) -> (String, String, bool) {
    if drv.is_null() || !has(drv, "drives") {
        return ("—".into(), "Open Drives & undo to check".into(), false);
    }
    let all = arr(drv, "drives");
    let ds: Vec<&Value> = all.iter().filter(|d| has(d, "smart")).collect();
    if ds.is_empty() {
        return ("—".into(), format!("{} drives · no health data", all.len()), false);
    }
    let failing = ds.iter().filter(|d| at(d, "smart.passed") == &Value::Bool(false)).count();
    let temps: Vec<f64> = ds.iter().map(|d| n(d, "smart.temp")).filter(|t| *t > 0.0).collect();
    let mut sub = format!("{} drives", all.len());
    if !temps.is_empty() {
        let lo = temps.iter().cloned().fold(f64::MAX, f64::min);
        let hi = temps.iter().cloned().fold(f64::MIN, f64::max);
        sub.push_str(&format!(" · {lo}–{hi} °C"));
    }
    (if failing > 0 { "Check".into() } else { "Good".into() }, sub, failing > 0)
}

fn failed_backups(stv: &Value) -> Vec<&Value> {
    arr(stv, "backups").iter().filter(|j| at(j, "state.last.ok") == &Value::Bool(false)).collect()
}

fn backup_big(stv: &Value) -> (String, String) {
    let jobs = arr(stv, "backups");
    if jobs.is_empty() {
        return ("None yet".into(), "a second copy somewhere else".into());
    }
    let failed = failed_backups(stv);
    if !failed.is_empty() {
        return ("Didn't finish".into(), failed.iter().map(|j| s(j, "name")).collect::<Vec<_>>().join(", "));
    }
    if jobs.iter().any(|j| b(j, "state.running")) {
        return ("Running".into(), "backing up now".into());
    }
    let last = jobs.iter().map(|j| n(j, "state.lastOk")).fold(0.0, f64::max);
    if last <= 0.0 {
        let sub = if jobs.iter().any(|j| b(j, "state.waiting")) { "waiting for your game".to_string() } else { format!("{} set up", jobs.len()) };
        return ("Not yet".into(), sub);
    }
    let h = ((hearth_ui::time::now() as f64 - last) / 3600.0).round();
    let big = if h < 1.0 { "Just now".to_string() } else if h < 48.0 { format!("{h} h ago") } else { format!("{} days ago", (h / 24.0).round()) };
    (big, plural(jobs.len() as f64, "backup", "backups"))
}

fn tile<'a>(p: &Pal, label: &str, big: String, of: String, sub: String, frac: Option<f64>, tone: Option<iced::Color>) -> E<'a, Msg> {
    let p = *p;
    let mut c = Column::new().spacing(3.0).push(t(label.to_string(), sz(13.0), p.muted));
    let mut r = Row::new().spacing(6.0).align_y(iced::alignment::Vertical::Bottom).push(ts(big, sz(21.0), tone.unwrap_or(p.text)));
    if !of.is_empty() {
        r = r.push(t(of, sz(14.0), p.muted));
    }
    c = c.push(r);
    if let Some(f) = frac {
        c = c.push(container(meter(&p, f, false)).padding(Padding { top: 4.0, right: 0.0, bottom: 0.0, left: 0.0 }));
    }
    if !sub.is_empty() {
        c = c.push(t(sub, sz(13.0), p.muted));
    }
    container(c).padding(Padding { top: 14.0, right: 16.0, bottom: 14.0, left: 16.0 }).width(Length::FillPortion(1)).height(Length::Fixed(sz(104.0))).style(ui::boxed(p.panel, p.r(12.0), None)).into()
}

pub fn overview<'a>(p: &Pal, st: &'a St) -> E<'a, Msg> {
    let stv = st.get("status");
    let cfg = at(stv, "settings");
    let has_cfg = cfg.is_object();
    let ts_ = at(stv, "tailscale");
    let svc = at(stv, "services");
    let drv = st.get("drives");
    let (hbig, hsub, hbad) = health(drv);
    let alerts = arr(stv, "alerts.active");
    let mut rs = Rows::new();
    let host = s(stv, "host");
    rs.add(words(p, if !host.is_empty() {
        format!("This PC (\"{host}\") shares folders, media and AI with your other devices: Windows PCs, phones, TVs and Hearth devices, at home and anywhere through Tailscale.")
    } else {
        "This PC shares folders, media and AI with your other devices: Windows PCs, phones, TVs and Hearth devices, at home and through Tailscale.".into()
    }));
    if b(stv, "error") || at(stv, "helper") == &Value::Bool(false) {
        rs.add(words(p, if b(stv, "error") { s(stv, "error") } else { "Parts of Home server come with the Hearth OS update that brings it (Samba, Tailscale): what you choose here is kept.".into() }));
    }
    // What needs a look.
    let mut attention: Vec<(String, &'static str, &'static str)> = Vec::new();
    if has_cfg {
        let failed = failed_backups(stv);
        if !failed.is_empty() {
            let names = failed.iter().map(|j| format!("“{}”", s(j, "name"))).collect::<Vec<_>>().join(", ");
            let err = s(failed[0], "state.last.error");
            attention.push((format!("The backup {names} didn't finish{}", if err.is_empty() { String::new() } else { format!(": {err}") }), "hs-backups", "Open Backups"));
        }
        if b(svc, "guardTripped") {
            attention.push(("The ransomware guard stopped writes to your shares".into(), "hs-safety", "Open Safety"));
        }
        for a in alerts {
            attention.push((if s(a, "text").is_empty() { "An alert".into() } else { s(a, "text") }, "hs-alerts", "Open Alerts"));
        }
        if hbad {
            attention.push(("A drive's health check failed".into(), "hs-drives", "Open Drives"));
        }
    }
    if let Some(first) = attention.first() {
        let n_ = attention.len();
        rs.add(banner(p, "warn", "warn", if n_ == 1 { "1 thing needs a look".into() } else { format!("{n_} things need a look") }, first.0.clone(), first.2, Some(Msg::Go(first.1))));
    }
    if has_cfg && !b(cfg, "setupDone") {
        rs.add(r("check", "Set up your home server").sub("A few steps, all with the controller: this PC's name, Tailscale, your first shared folder and your first backup").chev().on(Msg::Go("hs-setup")).el(p));
    }
    if has_cfg {
        let shares_on: Vec<&Value> = arr(stv, "shares").iter().filter(|x| nf(x, "on")).collect();
        let total = n(stv, "usage.total");
        let used = n(stv, "usage.used");
        let ts_state = s(ts_, "state");
        let ts_big = if ts_state == "Running" {
            "Connected".to_string()
        } else if at(ts_, "installed") == &Value::Bool(false) {
            "Not installed".into()
        } else if at(ts_, "running") == &Value::Bool(false) {
            "Off".into()
        } else if ts_state == "NeedsLogin" {
            "Sign in".into()
        } else if ts_state.is_empty() {
            "—".into()
        } else {
            ts_state.clone()
        };
        let online = arr(ts_, "peers").iter().filter(|x| b(x, "online")).count();
        let names = shares_on.iter().map(|x| s(x, "name")).take(4).collect::<Vec<_>>().join(" · ");
        let glance = row![
            tile(p, "Storage used", if total > 0.0 { size(used) } else { "—".into() }, if total > 0.0 { format!("of {}", size(total)) } else { String::new() }, if total > 0.0 { String::new() } else { "nothing shared yet".into() }, if total > 0.0 { Some(used / total) } else { None }, None),
            tile(p, "Shared folders", shares_on.len().to_string(), String::new(), if names.is_empty() { "nothing is shared until you share it".into() } else { names }, None, None),
            tile(p, "Tailscale", ts_big, String::new(), if ts_state == "Running" { format!("{online} of your devices online") } else { "reach this PC from anywhere".into() }, None, if ts_state == "Running" { Some(p.good) } else { None }),
            tile(p, "Drive health", hbig.clone(), String::new(), hsub, None, if hbad { Some(p.danger) } else if hbig == "Good" { Some(p.good) } else { None }),
        ]
        .spacing(10.0);
        rs.add(container(glance).padding(Padding { top: 6.0, right: 0.0, bottom: 4.0, left: 0.0 }).into());

        // Its pages in four groups.
        let svc_on: Vec<&str> = [("media", "Media"), ("web", "Web"), ("ai", "AI"), ("pictures", "Pictures"), ("voices", "Voices")].iter().filter(|(k, _)| b(svc, &format!("{k}.on"))).map(|x| x.1).collect();
        let ai_shown = b(stv, "seen.commandCenter") && (b(stv, "seen.commandCenter.shown") || b(svc, "ai.on"));
        let people = arr(stv, "people").len() as f64;
        let links = n(svc, "links");
        let apps_on = n(st.get("appsq"), "on");
        let shares_all = arr(stv, "shares");
        let left = column![
            group(p, "Sharing", vec![
                link_card(p, "hs-shares", &shares_on.len().to_string(), Some(if shares_all.is_empty() { "Nothing is shared until you share it. Pick a folder for your devices".into() } else { "What your devices can open, and how".into() }), None),
                link_card(p, "hs-files", &if links > 0.0 { plural(links, "link", "links") } else { String::new() }, None, None),
                link_card(p, "hs-people", &if people > 0.0 { plural(people, "person", "people") } else { String::new() }, None, None),
                link_card(p, "hs-family", "", Some(format!("A Family folder{}", if b(stv, "seen.camera.shown") { ", calendar and cameras" } else { " and calendar" })), None),
            ]),
            group(p, "On this server", vec![
                link_card(p, "hs-services", &if svc_on.is_empty() { String::new() } else { format!("{} on", svc_on.len()) }, Some(format!("Media server, files on the web, AI pictures and voices{}", if ai_shown { ", AI chat" } else { "" })), None),
                link_card(p, "hs-apps", &if apps_on > 0.0 { format!("{apps_on} on") } else { String::new() }, None, None),
                link_card(p, "hs-server", "", None, None),
                link_card(p, "hs-gaming", "", None, None),
                link_card(p, "hs-aidevices", "", None, None),
            ]),
        ]
        .spacing(0.0)
        .width(Length::FillPortion(1));
        let failed = failed_backups(stv).len();
        let jobs = arr(stv, "backups").len();
        let (bbig, _) = backup_big(stv);
        let backups_chip = if failed > 0 { Some((format!("{failed} failed"), Ck::Warn)) } else { None };
        let backups_value = if failed > 0 { String::new() } else if jobs > 0 { bbig } else { "None yet".into() };
        let tripped = b(svc, "guardTripped");
        let light = nf(cfg, "lightMode");
        let right = column![
            group(p, "Keep it safe", vec![
                link_card(p, "hs-drives", if b(cfg, "snapshots") { "Snapshots on" } else { "" }, None, if hbig == "Good" { Some(("Good".into(), Ck::Ok)) } else if hbad { Some(("Check".into(), Ck::Warn)) } else { None }),
                link_card(p, "hs-backups", &backups_value, None, backups_chip),
                link_card(p, "hs-safety", "", None, if tripped { Some(("Read only".into(), Ck::Bad)) } else if b(cfg, "guard.on") { Some(("Watching".into(), Ck::Ok)) } else { None }),
                link_card(p, "hs-phone", "", None, None),
                link_card(p, "hs-alerts", if alerts.is_empty() && b(cfg, "alerts.on") { "All good" } else { "" }, None, if alerts.is_empty() { None } else { Some((plural(alerts.len() as f64, "alert", "alerts"), Ck::Warn)) }),
            ]),
            group(p, "Reach & play", vec![
                link_card(p, "hs-tailscale", if ts_state == "Running" { "Connected" } else if at(ts_, "installed") == &Value::Bool(false) { "Not installed" } else { "" }, Some("Lives in Network & internet now".into()), None),
                r("server", "Light mode while you play")
                    .sub(format!("Big jobs (backups, snapshots, emptying recycle bins) wait while a game runs. Sharing keeps working.{}", if b(stv, "playing") && b(cfg, "lightMode") { " · On now: you're playing" } else { "" }))
                    .switch(p, light, Some(act(H::Hs, &["set", "lightMode", if light { "off" } else { "on" }]).msg()))
                    .el(p),
                link_card(p, "remote", "", Some("Play this PC's games on your laptop, phone or TV · Gaming › Remote play".into()), None),
            ]),
        ]
        .spacing(0.0)
        .width(Length::FillPortion(1));
        rs.add(row![left, right].spacing(16.0).into());

        rs.add(container(h(p, "Hidden items")).padding(Padding { top: 18.0, right: 0.0, bottom: 0.0, left: 0.0 }).into());
        let hidden_now = n(stv, "hiddenCount");
        let show_hidden = b(cfg, "showHidden");
        rs.add(r("info", "Show hidden items")
            .sub(format!("Items for the Hearth AI Command Center (the AI page and service) show when it was seen on one of your devices in the last {} days, and cameras or UPS items once one is found. Hiding never turns anything off.{}", if n(cfg, "ccDays") > 0.0 { s(cfg, "ccDays") } else { "30".into() }, if hidden_now > 0.0 { format!(" {hidden_now} hidden now.") } else { String::new() }))
            .switch(p, show_hidden, Some(act(H::Hs, &["set", "showHidden", if show_hidden { "off" } else { "on" }]).msg()))
            .el(p));
        let cc = if n(cfg, "ccDays") > 0.0 { s(cfg, "ccDays") } else { "30".into() };
        rs.add(choice(p, "", "Command Center seen within", "", opts(&[("7", "7 days", false), ("30", "30 days", false), ("90", "90 days", false), ("365", "A year", false)], &cc, |v| act(H::Hs, &["set", "ccDays", v]).msg())));
    }
    if at(stv, "applied.ok") == &Value::Bool(false) {
        rs.add(words_c(format!("Your settings are saved, but the system couldn't be changed to match: {}", s(stv, "applied.error")), p.danger));
    }
    rs.add(message(p, st));
    rs.el()
}

// ------------------------------------------------------------------ Set up your home server
pub fn setup<'a>(p: &Pal, st: &'a St) -> E<'a, Msg> {
    let pp = *p;
    let stv = st.get("status");
    let ts_ = at(stv, "tailscale");
    let step = st.ln("step") as usize;
    let steps = ["Name", "Tailscale", "First share", "First backup", "Done"];
    let mut chips = Row::new().spacing(8.0);
    for (i, sname) in steps.iter().enumerate() {
        let (bg, fg) = if i == step { (Pal::a(pp.accent, 0.22), pp.accent) } else if i < step { (Pal::a(pp.good, 0.18), pp.good) } else { (pp.panel2, pp.muted) };
        chips = chips.push(container(t(format!("{}{sname}", if i < step { "✓ " } else { "" }), sz(12.0), fg)).padding(Padding { top: 5.0, right: 11.0, bottom: 5.0, left: 11.0 }).style(ui::boxed(bg, 14.0, None)));
    }
    let mut rs = Rows::new();
    rs.add(chips.into());
    let titles = ["Name this PC", "Reach your PC from anywhere", "Share your first folder", "Your first backup", "Your home server is ready"];
    let subs = [
        "Your other devices find this PC by its name: in Windows' File Explorer, on your phone and on Tailscale.",
        "Tailscale makes a private network just for your devices. Nothing is opened to the internet.",
        "Pick a folder your Windows PC, phone, TV and Hearth devices can open. It starts read only, on your home network.",
        "A second copy of your shares somewhere else: a backup drive, another Hearth device or the cloud, encrypted with your password.",
        "You can change everything in Settings › Home server.",
    ];
    rs.add(container(tb(titles[step.min(4)], sz(24.0), pp.text)).padding(Padding { top: 10.0, right: 0.0, bottom: 0.0, left: 0.0 }).into());
    rs.add(words(p, subs[step.min(4)]));
    let host = s(stv, "host");
    let running = s(ts_, "state") == "Running";
    match step {
        0 => {
            rs.add(r("display", host.clone()).sub("This PC's name now").el(p));
            rs.add(tfield(p, st, "name", "A new name (letters, digits and -), then Enter. Y types with the controller", false));
        }
        1 => {
            if running {
                rs.add(r("check", "Signed in ✓").sub(format!("This PC is {} on your tailnet", if s(ts_, "dns").is_empty() { s(ts_, "name") } else { s(ts_, "dns") })).el(p));
            }
            if at(ts_, "installed") == &Value::Bool(false) {
                rs.add(r("info", "Tailscale isn't on this PC yet").sub("It comes with the Hearth OS update that brings Home server. Skip this step for now.").el(p));
            }
            let login = st.get("login");
            if b(ts_, "installed") && !running {
                let login_act = act(H::Hs, &["ts-login"]).store("login").secs(60);
                let a = if at(ts_, "running") == &Value::Bool(false) { act(H::Hs, &["ts-start"]).then(login_act) } else { login_act };
                rs.add(r("lock", if login.is_object() && !b(login, "error") { "Waiting for you to sign in…" } else { "Show the sign-in code" }).sub("A QR code for your phone, and a link for any device").on(a.msg()).el(p));
            }
            if b(login, "authUrl") && !running {
                let mut c = column![
                    t(format!("1. Scan with your phone, or open {}", s(login, "authUrl")), sz(14.0), pp.text),
                    t("2. Sign in with Google, Microsoft, GitHub or Apple", sz(14.0), pp.text),
                    t(format!("3. This PC shows up as {host}"), sz(14.0), pp.text)
                ]
                .spacing(6.0);
                let q = st.get("qrtext");
                let mut rw = Row::new().spacing(18.0).align_y(iced::Center);
                if !s(login, "qr").is_empty() && q.is_string() {
                    rw = rw.push(container(qr(&s(q, ""), 200.0)).padding(10.0).style(ui::boxed(iced::Color::WHITE, pp.r(12.0), None)));
                }
                c = c.width(Length::Fill);
                rs.add(rw.push(c).into());
            }
        }
        2 => {
            let shares = arr(stv, "shares");
            for c in arr(st.get("cands"), "candidates").iter().filter(|c| !shares.iter().any(|x| s(x, "path") == s(c, "path"))).take(6) {
                let a = act_v(H::Hs, vec!["share-add".into(), s(c, "path")]).ok("Shared ✓ {share.name}. Set its password in Shared folders");
                rs.add(r(if b(c, "drive") { "store" } else { "folder" }, format!("Share {}", s(c, "name"))).sub(s(c, "path")).on(a.msg()).el(p));
            }
            if !shares.is_empty() {
                rs.add(r("check", format!("Shared: {}", shares.iter().map(|x| s(x, "name")).collect::<Vec<_>>().join(", "))).el(p));
            }
        }
        3 => {
            for j in arr(stv, "backups") {
                let every = s(j, "every");
                rs.add(r("check", format!("Backing up: {}", s(j, "name"))).sub(if every == "hourly" { "Every hour".to_string() } else { format!("{}{}", if every == "weekly" { "Weekly at " } else { "Daily at " }, s(j, "at")) }).el(p));
            }
            let any = !arr(stv, "backups").is_empty();
            rs.add(r("update", if any { "Add another backup" } else { "Set up a backup" }).sub(if !arr(stv, "shares").is_empty() { "Backups & sync: pick where it goes, how often and a password" } else { "Share a folder first (the step before): backups are of your shared folders" }).chev().on(Msg::Go("hs-backups")).el(p));
            rs.add(r("desktops", "Hearth Storage's backups").sub("Your Hearth devices back up their saves and screenshots to this PC (Hearth Link › Hearth Storage)").chev().on(Msg::Go("link")).el(p));
        }
        _ => {}
    }
    if step < 4 {
        let next = Msg::Locals(vec![("step".into(), json!(step + 1))]);
        rs.add(r("forward", if step == 1 && !running { "I've signed in" } else { "Next" }).sub(if step == 1 && !running { "Or skip it: you can sign in on the Tailscale page any time" } else { "" }).on(next).el(p));
    }
    if step > 0 && step < 4 {
        rs.add(r("back", "Back").on(Msg::Locals(vec![("step".into(), json!(step - 1))])).el(p));
    }
    if step == 4 {
        rs.add(r("check", "Go to Home server").chev().on(act(H::Hs, &["set", "setupDone", "on"]).go("hs-overview").no_reload().msg()).el(p));
    }
    rs.add(message(p, st));
    rs.el()
}

// ------------------------------------------------------------------ Server mode
pub fn server<'a>(p: &Pal, st: &'a St) -> E<'a, Msg> {
    let sm = st.get("sm");
    let s_ = at(sm, "settings");
    let has_s = s_.is_object();
    let pa = at(sm, "pauses");
    let has_p = pa.is_object();
    let mut rs = Rows::new();
    rs.add(words(p, "A third mode next to Game Mode and Desktop Mode: a dashboard on the screen, nothing else using memory, and full power for your services. One press of Play (or Guide) and you're in Game Mode; leaving Game Mode brings you back."));
    if b(sm, "error") {
        rs.add(words_c(s(sm, "error"), p.danger));
    }
    rs.add(h(p, "Server mode"));
    let on = b(s_, "on");
    let toggle_on = act(H::ServerMode, &["set", "on", if on { "off" } else { "on" }]).ok(if on { "Server mode is off" } else { "Server mode is on ✓ Open it now, or pick it in Start in" }).then(act(H::ServerMode, &["web-apply"]));
    rs.add(r("server", "Use Server mode").sub("Show it next to Game Mode and Desktop Mode: in Start in, here and on the dashboard. Off by default; Game Mode stays where your PC starts unless you choose Server mode").switch(p, on, Some(toggle_on.msg())).el(p));
    if on {
        let now_server = s(sm, "now") == "server";
        rs.add(r("play", "Open Server mode now").sub(if now_server { "You're in Server mode" } else { "Game Mode and Desktop Mode close first (save your work)" }).value("Open").on_maybe((!now_server).then(|| Msg::Detached(sv(&["nimbus-mode", "server"])))).el(p));
    }
    let start_in = match s(sm, "startIn").as_str() {
        "desktop" => "Desktop Mode",
        "server" => "Server mode",
        _ => "Game Mode",
    };
    rs.add(r("power", "Startup and schedule").sub(if b(s_, "schedule.on") { "Following a schedule".to_string() } else { format!("Start in: {start_in}. No schedule unless you make one") }).value("In System").chev().on(Msg::Go("startup")).el(p));
    rs.add(r("chat", "AI engine").sub("Ollama, Colibri or both. Colibri runs much bigger models: the busiest parts on the graphics card, the rest in memory and on your drive").chev().on(Msg::Go("hs-aiengine")).el(p));

    rs.add(h(p, "Power when idle"));
    let set = |k: &str, v: &str| act(H::ServerMode, &["set", k, v]).msg();
    let hs_set = |k: &str, v: &str| act(H::Hs, &["set", k, v]).also(rd("sm", H::ServerMode, &["status"])).msg();
    if has_s {
        let ip = b(s_, "idlePower");
        rs.add(r("bolt", "Lower power limits when idle")
            .sub(format!("After {} minutes with no jobs: the processor on power-saver{}. Full power comes back the moment a job starts, and always before Game Mode", s(s_, "idleMinutes"), if b(sm, "nvidia") { format!(", the graphics card at {} W", s(s_, "gpuWatts")) } else { String::new() }))
            .switch(p, ip, Some(set("idlePower", if ip { "off" } else { "on" })))
            .el(p));
        if ip && b(sm, "nvidia") {
            let cur = s(s_, "gpuWatts");
            rs.add(choice(p, "", "Graphics card limit when idle", "", opts(&[("80", "80 W", false), ("100", "100 W", false), ("120", "120 W", false), ("150", "150 W", false), ("200", "200 W", false)], &cur, |v| set("gpuWatts", v))));
        }
        if ip {
            let cur = s(s_, "idleMinutes");
            rs.add(choice(p, "", "Idle means no jobs for", "", opts(&[("5", "5 min", false), ("10", "10 min", false), ("20", "20 min", false), ("30", "30 min", false)], &cur, |v| set("idleMinutes", v))));
        }
    }
    if sm.is_object() && !b(sm, "error") {
        let cur = if has(sm, "spinDown") { s(sm, "spinDown") } else { "0".into() };
        rs.add(choice(p, "drive", "Spin down drives", "Data drives only; the system drive stays on (the same setting as Alerts & power)", opts(&[("0", "Never", false), ("10", "10 min", false), ("20", "20 min", false), ("30", "30 min", false), ("60", "1 hour", false)], &cur, |v| hs_set("spinDown", v))));
    }
    if has_s {
        let cur = s(s_, "dim");
        rs.add(choice(p, "display", "Screen dims to a clock after", "Dim shows the time and the status in dark grey, kind to OLED screens. Any button brings the dashboard back", opts(&[("0", "Never", false), ("1", "1 min", false), ("2", "2 min", false), ("5", "5 min", false), ("10", "10 min", false)], &cur, |v| set("dim", v))));
        let cur = s(s_, "off");
        rs.add(choice(p, "", "Then off after", if b(sm, "wlopm") { "The screen turns off" } else { "The screen goes black (install wlopm to turn it off completely)" }, opts(&[("0", "Never", false), ("5", "5 min", false), ("10", "10 min", false), ("20", "20 min", false), ("30", "30 min", false), ("60", "1 hour", false)], &cur, |v| set("off", v))));
    }
    if sm.is_object() && !b(sm, "error") {
        let cur = if has(sm, "sleepIdle") { s(sm, "sleepIdle") } else { "0".into() };
        rs.add(choice(p, "sleep", "Sleep", "Server mode keeps the PC awake unless you choose a time here. Waking it over Tailscale needs another device at home", opts(&[("0", "Never", false), ("30", "30 min", false), ("60", "1 hour", false), ("120", "2 hours", false), ("240", "4 hours", false)], &cur, |v| hs_set("sleepIdle", v))));
    }

    rs.add(h(p, "What pauses when you play"));
    if has_p {
        let lm = b(pa, "lightMode");
        rs.add(r("pad", "Light mode while you play").sub("The same rules for Server mode's Play button, Guide and any game. Backups wait while it's on").switch(p, lm, Some(hs_set("lightMode", if lm { "off" } else { "on" }))).el(p));
        if b(pa, "picturesOn") {
            let cur = if s(pa, "pictures").is_empty() { "stop".into() } else { s(pa, "pictures") };
            rs.add(choice(p, "image", "Pictures · ComfyUI", "", opts(&[("stop", "Stop, keep the queue", false), ("keep", "Keep running", false)], &cur, |v| hs_set("picturesPause", if v == "stop" { "on" } else { "off" }))));
        }
        if b(pa, "aiOn") {
            let cur = if s(pa, "ai").is_empty() { "small".into() } else { s(pa, "ai") };
            // One choice, written as Light mode's own keys (only the ones that differ are saved).
            let pick_ai = |v: &str| -> Msg {
                let want: [(&str, &str); 3] = match v {
                    "small" => [("aiSmall", "on"), ("aiPause", "off"), ("aiUnload", "on")],
                    "unload" => [("aiSmall", "off"), ("aiPause", "off"), ("aiUnload", "on")],
                    "stop" => [("aiSmall", "off"), ("aiPause", "on"), ("aiUnload", "on")],
                    _ => [("aiSmall", "off"), ("aiPause", "off"), ("aiUnload", "off")],
                };
                let a3 = act(H::Hs, &["set", want[2].0, want[2].1]).also(rd("sm", H::ServerMode, &["status"]));
                let a2 = act(H::Hs, &["set", want[1].0, want[1].1]).no_reload().then(a3);
                act(H::Hs, &["set", want[0].0, want[0].1]).no_reload().then(a2).msg()
            };
            rs.add(choice(p, "chat", "AI chat · Ollama", format!("The small model is {} on the processor (only when it's downloaded; nothing is ever deleted)", s(pa, "small")), opts(&[("small", "Switch to the small model", false), ("unload", "Unload the model", false), ("stop", "Stop AI chat", false), ("keep", "Keep it loaded", false)], &cur, pick_ai)));
        }
        if b(pa, "voicesOn") {
            let cur = if s(pa, "voices").is_empty() { "keep".into() } else { s(pa, "voices") };
            rs.add(choice(p, "wave", "Voices · Kokoro", "", opts(&[("pause", "Pause", false), ("keep", "Keep running", false)], &cur, |v| hs_set("voicesPause", if v == "pause" { "on" } else { "off" }))));
        }
        if b(pa, "mediaOn") {
            let cur = if s(pa, "media").is_empty() { "keep".into() } else { s(pa, "media") };
            rs.add(choice(p, "film", "Media · Jellyfin", "", opts(&[("keep", "Keep, convert on the processor", false), ("pause", "Pause", false)], &cur, |v| hs_set("mediaPause", if v == "pause" { "on" } else { "off" }))));
        }
    }
    rs.add(r("folder", "Shared folders, Tailscale").sub("Always keep working").value("Keep").el(p));

    rs.add(h(p, "Web page"));
    if has_s {
        let web_on = b(s_, "web.on");
        let url = s(sm, "url");
        let pin = b(sm, "pin");
        let sub = if !pin {
            format!("Open {url} on your phone or laptop. Only over Tailscale, never the internet. Set a PIN first (below)")
        } else if web_on && !b(sm, "webActive") {
            "Not running right now: turn this off and on again. If that doesn't help, restart the PC and use Send logs".to_string()
        } else if web_on && !b(sm, "tsIp") {
            "Tailscale isn't connected on this PC, so only this PC can open it. Connect Tailscale in Settings › Home server › Tailscale".to_string()
        } else {
            format!("Open {url} on your phone or laptop. Only over Tailscale, never the internet")
        };
        let toggle = if web_on {
            act(H::ServerMode, &["set", "webOn", "off"]).then(act(H::ServerMode, &["web-apply"]))
        } else if pin {
            act(H::ServerMode, &["set", "webOn", "on"]).then(act(H::ServerMode, &["web-apply"]).ok("The web page is on ✓"))
        } else {
            act(H::ServerMode, &["set", "webOn", "on"]).ok("Now set a PIN (4 to 12 digits): nobody can open the page without one").set("pinOpen", json!(true)).set("pinMsg", json!("Now set a PIN (4 to 12 digits): nobody can open the page without one"))
        };
        rs.add(r("globe", "Dashboard web page").sub(sub).switch(p, web_on, Some(toggle.msg())).el(p));
        let pin_open = st.lb("pinOpen");
        rs.add(r("lock", "PIN for the web page")
            .sub(if pin { "Asked once on each device, again after 30 days. Only its hash is kept; a new PIN signs every device out" } else { "4 to 12 digits. Nobody gets in until you set one" })
            .value(if pin { "●●●●●● · Change" } else { "Set" })
            .on(Msg::Locals(vec![("pinOpen".into(), if pin_open { Value::Null } else { json!(true) })]))
            .el(p));
        if pin_open {
            rs.add(pinpad(p, st, "pin", if pin { "Type a new PIN (4 to 12 digits): every device then types it again" } else { "Type a PIN of 4 to 12 digits" }, 4, 12));
        }
        if !st.ls("pinMsg").is_empty() && st.msg == st.ls("pinMsg") {
            rs.add(container(message(p, st)).padding(Padding { top: 0.0, right: 18.0, bottom: 0.0, left: 56.0 }).into());
        }
        if web_on {
            for (ic, title, sub, key, perm) in [
                ("restart", "Restart and pause services", "", "permServices", "services"),
                ("pad", "Switch modes", "The screen at home counts down 10 seconds first (cancel it there), and nothing switches while a game is running or Desktop Mode is open", "permModes", "modes"),
                ("sleep", "Sleep or restart the PC", "", "permPower", "power"),
                ("desktops", "Show it on my other Hearth devices", "Hearth Link and Quick Access on your Ally can show the status (read only) without the PIN, from your Tailscale devices only", "permDevices", "devices"),
            ] {
                let on_ = b(s_, &format!("web.perms.{perm}"));
                rs.add(r(ic, title).sub(sub).switch(p, on_, Some(set(key, if on_ { "off" } else { "on" }))).el(p));
            }
        }
    }
    if st.msg != st.ls("pinMsg") || st.ls("pinMsg").is_empty() {
        rs.add(message(p, st));
    }
    rs.el()
}

// ------------------------------------------------------------------ AI engine
fn gb(g: f64) -> String {
    if g >= 1000.0 {
        format!("{:.2} TB", g / 1000.0)
    } else if g >= 10.0 {
        format!("{} GB", g.round())
    } else {
        format!("{:.1} GB", g)
    }
}
fn fit_kind(f: &str) -> Ck {
    match f {
        "good" => Ck::Ok,
        "slow" | "very-slow" => Ck::Warn,
        _ => Ck::Bad,
    }
}

fn model_row<'a>(p: &Pal, st: &St, m: &Value) -> E<'a, Msg> {
    let dl = at(st.get("col"), "download");
    let downloading = b(dl, "running");
    let id = s(m, "id");
    let name = s(m, "name");
    let armed = st.ls("armed") == id;
    let mut sub = format!("{} · download {}, needs {} free · {}", s(m, "summary"), gb(n(m, "downloadGb")), gb(n(m, "diskGb")), s(m, "why"));
    if !s(m, "expect").is_empty() {
        sub.push_str(&format!(" · {}", s(m, "expect")));
    }
    sub.push_str(&format!(" · measured elsewhere: {}", s(m, "measured")));
    if !s(m, "license").is_empty() {
        sub.push_str(&format!(" · {}", s(m, "license")));
    }
    if armed {
        sub.push_str(" · press again to download");
    }
    let value = if b(m, "chosen") {
        "In use".to_string()
    } else if b(m, "have") {
        "Use".into()
    } else if b(m, "downloading") {
        if has(dl, "pct") { format!("{}%", s(dl, "pct")) } else { "…".into() }
    } else if s(m, "fit") == "no-disk" {
        "No room".into()
    } else {
        "Download".into()
    };
    let on = if b(m, "chosen") {
        Msg::Run(Box::new(Act { h: H::Colibri, args: vec!["plan".into(), id.clone()], store: Some("plan".into()), no_reload: true, secs: 60, ..Act::default() }))
    } else if b(m, "have") {
        act_v(H::Colibri, vec!["use".into(), id.clone()]).ok(format!("{name} is Colibri's model now")).then(act(H::Colibri, &["catalog"]).store("cat").no_reload()).msg()
    } else if b(m, "downloading") || downloading {
        Msg::Say("One download at a time: wait for it or cancel it".into(), true)
    } else if s(m, "fit") == "no-disk" {
        Msg::Say(format!("Not enough disk space for {name}: it needs {} free. Pick a bigger drive for your models", gb(n(m, "diskGb"))), true)
    } else if !armed {
        Msg::Locals(vec![("armed".into(), json!(id.clone()))])
    } else {
        act_v(H::Colibri, vec!["download".into(), id.clone()]).ok(format!("Downloading {name}…")).set("armed", Value::Null).secs(60).msg()
    };
    let title = format!("{}{}{}", name, if b(m, "recommended") { " · recommended" } else { "" }, if s(m, "kind") == "image" { " · pictures, not chat" } else { "" });
    let ic = if b(m, "chosen") { "check" } else if b(m, "have") { "drive" } else if s(m, "kind") == "image" { "image" } else { "plus" };
    r(ic, title).sub(sub).value(value).chip(s(m, "fitLabel"), fit_kind(&s(m, "fit"))).inset(s(m, "group") == "uncensored").on(on).el(p)
}

pub fn aiengine<'a>(p: &Pal, st: &'a St) -> E<'a, Msg> {
    let c = st.get("col");
    let cat = st.get("cat");
    let plan = st.get("plan");
    let inst = b(c, "installed");
    let hw = at(cat, "hardware");
    let dl = at(c, "download");
    let mut rs = Rows::new();
    rs.add(words(p, "Server mode's AI. Ollama runs models that fit on the graphics card. Colibri runs much bigger ones too: the busiest parts on the graphics card, the rest in memory and on your fast drive (slower, but far smarter models). Both answer only this PC and your devices on Tailscale."));
    rs.add(h(p, "AI engine"));
    let engine = s(c, "engine");
    let sub = if !c.is_object() {
        String::new()
    } else if engine == "none" {
        "Neither is on yet".into()
    } else if engine == "colibri" {
        "Colibri only (Ollama is off)".into()
    } else if engine == "both" {
        "Ollama and Colibri side by side (they share the graphics card)".into()
    } else {
        "Ollama, as before".into()
    };
    let cur = if engine == "none" { String::new() } else if engine.is_empty() { "ollama".into() } else { engine.clone() };
    rs.add(choice(p, "chat", "Server mode runs", sub, opts(&[("ollama", "Ollama", false), ("colibri", "Colibri", !inst), ("both", "Both", !inst)], &cur, |v| {
        act(H::Colibri, &["set", "engine", v]).ok(match v { "ollama" => "Ollama on, Colibri off", "colibri" => "Colibri on, Ollama off", _ => "Ollama and Colibri on" }).msg()
    })));
    if c.is_object() && !inst {
        let installing = b(c, "installing");
        rs.add(r("plus", if installing { format!("Installing Colibri… {}%", n(c, "progress")) } else { format!("Install Colibri {}", s(c, "pinned")) })
            .sub(if b(c, "error") { format!("Didn't install: {}. Try again", s(c, "error")) } else { "About 14 MB from Colibri's own release on GitHub, checked against the SHA-256 it publishes before anything is unpacked. Models come separately, only the one you pick".into() })
            .value(if installing { "" } else { "Install" })
            .on_maybe((!installing).then(|| act(H::Colibri, &["install"]).ok("Downloading Colibri…").secs(60).msg()))
            .el(p));
    }
    if inst {
        let (chip, k) = if !b(c, "on") { ("Off", Ck::Off) } else if !b(c, "active") { ("Stopped", Ck::Warn) } else if s(c, "server.health") == "ready" { ("Ready", Ck::Ok) } else { ("Starting", Ck::Warn) };
        let sub = if s(c, "model").is_empty() {
            "No model picked yet: download one below".to_string()
        } else {
            format!("{}{} · {} (model \"{}\")", s(c, "model"), if n(c, "server.tokensPerSecond") > 0.0 { format!(" · {} tokens/s on the last answer", s(c, "server.tokensPerSecond")) } else { String::new() }, s(c, "openai"), s(c, "model"))
        };
        rs.add(r("sparkle", format!("Colibri {}", s(c, "version"))).sub(sub).chip(chip, k).el(p));
        let dflt = if s(c, "default").is_empty() { "ollama".into() } else { s(c, "default") };
        rs.add(choice(p, "desktops", "Hearth's own AI uses", "What Hearth's AI pieces ask first (the AI team, Ithil when she asks Hearth). Ollama unless you change it", opts(&[("ollama", "Ollama", false), ("colibri", "Colibri", !b(c, "on"))], &dflt, |v| {
            act(H::Colibri, &["set", "default", v]).ok(format!("Hearth's AI uses {} first", if v == "colibri" { "Colibri" } else { "Ollama" })).msg()
        })));
        let pause = !c.is_object() || b(c, "pause");
        rs.add(r("pad", "Stop Colibri while you play").sub("It holds the graphics card's memory. Stopped when Game Mode or a game starts, started again after").switch(p, pause, Some(act(H::Colibri, &["set", "pause", if pause { "off" } else { "on" }]).msg())).el(p));

        rs.add(h(p, "Models"));
        let hw_line = if hw.is_object() {
            format!("This PC: {} GB memory{} · {} free for models", n(hw, "ramGb").round(), if b(hw, "gpu") { format!(" · {} {} GB", s(hw, "gpu"), s(hw, "vramGb")) } else { " · no graphics card found".into() }, gb(n(hw, "diskFreeGb")))
        } else {
            "Reading what this PC has…".into()
        };
        let models = arr(cat, "models");
        rs.add(r("cpu", hw_line).sub(if cat.is_object() && !b(cat, "error") { format!("Every model in {} ({}), from the smallest to the largest. The fit is Hearth's estimate for this PC; the speed is what Colibri measured on other PCs", s(cat, "source"), models.len()) } else { String::new() }).el(p));
        let dirs_open = st.lb("dirsOpen");
        let dirs_act = if dirs_open { Msg::Local("dirsOpen".into(), Value::Null) } else { act(H::Colibri, &["folders"]).store("dirs").set("dirsOpen", json!(true)).no_reload().msg() };
        rs.add(r("drive", "Models folder").sub(s(c, "modelsDir")).value(if dirs_open { "Close" } else { "Change" }).on(dirs_act).el(p));
        if dirs_open {
            let dirs = st.get("dirs");
            for f in arr(dirs, "folders") {
                let path = s(f, "path");
                let a = act_v(H::Colibri, vec!["set".into(), "modelsDir".into(), path.clone()]).ok(format!("Models go to {path}")).set("dirsOpen", Value::Null).then(act(H::Colibri, &["catalog"]).store("cat").no_reload());
                rs.add(r(if path == s(dirs, "current") { "check" } else { "drive" }, format!("{} · {} free", s(f, "label"), gb(n(f, "freeGb")))).sub(path).inset(true).on(a.msg()).el(p));
            }
        }
        if b(dl, "running") {
            let name = if s(dl, "name").is_empty() { s(dl, "id") } else { s(dl, "name") };
            let sub = if n(dl, "total") > 0.0 {
                format!("{} of {}{} · every file checked; safe to stop, it continues where it stopped", gb(n(dl, "done") / 1e9), gb(n(dl, "total") / 1e9), if n(dl, "rate") > 0.0 { format!(" · {:.1} MB/s", n(dl, "rate") / 1e6) } else { String::new() })
            } else if !s(dl, "phase").is_empty() {
                format!("Getting ready: {}", s(dl, "phase"))
            } else {
                "Starting…".into()
            };
            rs.add(r("update", format!("Downloading {name}{}", if has(dl, "pct") { format!("… {}%", s(dl, "pct")) } else { "…".into() })).sub(sub).value("Cancel").on(act(H::Colibri, &["cancel"]).ok("Stopped. What was downloaded stays").msg()).el(p));
        }
        if dl.is_object() && !b(dl, "running") && b(dl, "error") {
            rs.add(r("info", format!("{} didn't finish", if s(dl, "name").is_empty() { s(dl, "id") } else { s(dl, "name") })).sub(s(dl, "error")).el(p));
        }
        if cat.is_object() && !b(cat, "error") {
            rs.add(h(p, "Standard (filtered)"));
            for m in models.iter().filter(|m| s(m, "group") == "standard") {
                rs.add(model_row(p, st, m));
            }
            let mut families: Vec<String> = Vec::new();
            for m in models.iter().filter(|m| s(m, "group") == "uncensored") {
                let f = s(m, "family");
                if !families.contains(&f) {
                    families.push(f);
                }
            }
            if !families.is_empty() {
                rs.add(h(p, "Uncensored"));
                rs.add(words(p, "Builds of the same Qwen models with their refusals taken out (\"abliterated\"), from Hugging Face. They answer anything, so keep them off a PC children use. Each comes in every size its maker published: smaller is faster and fits more easily, bigger is closer to the original. Downloaded only when you press one twice, checked against Hugging Face's SHA-256, then made ready by Colibri (coli import)."));
                for f in families.iter() {
                    let rows_: Vec<&Value> = models.iter().filter(|m| s(m, "group") == "uncensored" && &s(m, "family") == f).collect();
                    let best = rows_.iter().find(|m| b(m, "recommended")).or_else(|| rows_.iter().find(|m| s(m, "quant") == "Q4_K_M" || s(m, "quant") == "Q4_K")).or(rows_.first()).copied();
                    let lo = rows_.iter().map(|m| n(m, "downloadGb")).fold(f64::MAX, f64::min);
                    let hi = rows_.iter().map(|m| n(m, "downloadGb")).fold(0.0, f64::max);
                    let ic = if rows_.iter().any(|m| b(m, "chosen")) { "check" } else if rows_.iter().any(|m| b(m, "have")) { "drive" } else { "lock" };
                    let title = best.map(|m| s(m, "familyName")).unwrap_or_else(|| f.clone());
                    let sub = format!("{} sizes, {} to {}{}", rows_.len(), gb(lo), gb(hi), best.map(|m| format!(" · {}", s(m, "summary"))).unwrap_or_default());
                    let chip = best.map(|m| (format!("{}: {}", s(m, "quant"), s(m, "fitLabel")), fit_kind(&s(m, "fit"))));
                    // `hearth-settings hs-aiengine:fam:FAMILY` opens one (CI's pictures).
                    let key = format!("fam:{f}");
                    let st2_open = st.lb(&key);
                    let inner: Vec<E<'a, Msg>> = if st2_open { rows_.iter().map(|m| model_row(p, st, m)).collect() } else { Vec::new() };
                    let head_ = r(ic, title).sub(sub).on(lv(&key, Value::Bool(!st2_open))).control(hearth_ui::ui::glyph(if st2_open { hearth_ui::ui::G::Down } else { hearth_ui::ui::G::Chevron }, p.muted, 14.0, 2.0));
                    let head_ = match chip {
                        Some((c_, k)) => head_.chip(c_, k),
                        None => head_,
                    };
                    let mut colm = Column::new().spacing(3.0).push(head_.el(p));
                    for e in inner {
                        colm = colm.push(e);
                    }
                    rs.add(colm.into());
                }
            }
        }
        if b(plan, "id") {
            rs.add(h(p, "On this PC"));
            let mut sub = String::new();
            if !s(plan, "bottleneck").is_empty() {
                sub.push_str(&format!("What sets its speed: {}", s(plan, "bottleneck")));
            }
            if has(plan, "hitRate") {
                sub.push_str(&format!(" · experts found in memory: about {}%", (n(plan, "hitRate") * 100.0).round()));
            }
            if !s(plan, "doctor").is_empty() {
                sub.push_str(&format!(" · check: {}", s(plan, "doctor")));
            }
            let probs: Vec<String> = arr(plan, "problems").iter().map(vs).collect();
            if !probs.is_empty() {
                sub.push_str(&format!(" · {}", probs.join("; ")));
            }
            rs.add(r("rocket", format!("{}: {} GB on the graphics card · {} GB in memory · {} GB on the drive", s(plan, "id"), n(plan, "vramGb"), n(plan, "ramGb"), n(plan, "diskGb"))).sub(sub).el(p));
        }
    }
    rs.add(message(p, st));
    rs.el()
}
