//! Drives & undo (SetHomeDrives.qml), Backups & sync (SetHomeBackups.qml), Safety (SetHomeSafety.qml) and Alerts &
//! power (SetHomeAlerts.qml).

use super::kit::*;
use crate::widgets::sz;
use hearth_ui::ui::{self, t, tb};
use hearth_ui::Pal;
use iced::widget::{column, container};
use iced::{Length, Padding};
use serde_json::{json, Value};

pub fn reads(page: &str, st: &St) -> Vec<Rd> {
    let once = |v: &mut Vec<Rd>, slot: &str, h_: H, args: &[&str]| {
        if !st.d.contains_key(slot) {
            v.push(rd(slot, h_, args));
        }
    };
    match page {
        "hs-drives" => {
            let mut v = vec![rd("status", H::Hs, &["status"])];
            once(&mut v, "drives", H::Hs, &["drives"]);
            once(&mut v, "cands", H::Hs, &["candidates"]);
            v
        }
        "hs-backups" => {
            let mut v = vec![rd("status", H::Hs, &["status"]), rd("sync", H::Hs, &["sync-status"])];
            once(&mut v, "cands", H::Hs, &["candidates"]);
            v
        }
        "hs-safety" => vec![rd("safety", H::Hs, &["safety"])],
        _ => vec![
            rd("status", H::Hs, &["status"]),
            // The last night's maintenance and the power history: records the root helper keeps, readable by you.
            rd("night", H::Sh, &["cat /var/lib/hearth/home-server-night.json 2>/dev/null || echo null"]),
            rd("powerh", H::Sh, &["cat /var/lib/hearth/power-history.json 2>/dev/null || echo null"]),
        ],
    }
}

pub fn follow(_page: &str, slot: &str, st: &St) -> Vec<Rd> {
    let stv = st.get("status");
    if slot == "status" && b(stv, "seen.ups.shown") && b(stv, "settings.power.ups") {
        return vec![rd("ups", H::Hs, &["ups-status"])];
    }
    Vec::new()
}

pub fn live(_page: &str, st: &St) -> bool {
    arr(st.get("status"), "backups").iter().any(|j| b(j, "state.running"))
}

fn add_backup(st: &St, pw: &str) -> Result<Act, String> {
    let t_ = st.loc("newTarget").clone();
    if !t_.is_object() {
        return Err("Pick where it goes first".into());
    }
    if pw.chars().count() < 8 {
        return Err("The password needs at least 8 characters".into());
    }
    let kind = s(&t_, "kind");
    let raw_name = match kind.as_str() {
        "drive" => "To backup drive".to_string(),
        "hearth" => format!("To {}", s(&t_, "label")),
        _ => "To the cloud".into(),
    };
    let name: String = raw_name.chars().filter(|c| c.is_ascii_alphanumeric() || " _-".contains(*c)).take(40).collect();
    let shares: Value = if st.l.contains_key("newShares") { st.loc("newShares").clone() } else { json!(arr(st.get("status"), "shares").iter().map(|x| s(x, "id")).collect::<Vec<_>>()) };
    let provider = if st.ls("provider").is_empty() { "b2".to_string() } else { st.ls("provider") };
    let every = if st.ls("newEvery").is_empty() { "daily".to_string() } else { st.ls("newEvery") };
    let target = match kind.as_str() {
        "cloud" => json!({ "kind": "cloud", "provider": provider, "path": st.txt("bucket").trim() }),
        "drive" => json!({ "kind": "drive", "path": s(&t_, "path") }),
        _ => json!({ "kind": "hearth", "peer": s(&t_, "peer") }),
    };
    let mut req = json!({ "name": name, "shares": shares, "every": every, "at": "03:00", "password": pw, "target": target });
    if kind == "hearth" {
        req["code"] = json!(st.txt("code").trim());
    }
    if kind == "cloud" {
        req["cloud"] = json!({ "access": st.txt("keyid").trim(), "secret": st.txt("keysecret").trim(), "endpoint": st.txt("endpoint").trim(), "region": "" });
    }
    Ok(act(H::Hs, &["backup-add"]).input(req.to_string()).ok("Backup added ✓ It runs {backup.every}. Keep your password somewhere safe: restoring on another PC needs it").set("adding", Value::Null).set_from("sel", "backup.id"))
}

pub fn submit(st: &mut St, key: &str, text: String) -> Result<Vec<Act>, String> {
    match key {
        "newpw" => {
            let a = add_backup(st, &text)?;
            for k in ["code", "bucket", "endpoint", "keyid", "keysecret"] {
                st.t.remove(k);
            }
            Ok(vec![a])
        }
        "jobpw" => Ok(vec![act_v(H::Hs, vec!["backup-password".into(), st.ls("sel")]).input(text).ok("Password saved ✓").set("pwOpen", Value::Null)]),
        "recvname" => {
            let t_ = text.trim().to_string();
            if t_.is_empty() {
                return Ok(Vec::new());
            }
            Ok(vec![act_v(H::Hs, vec!["receive-add".into(), t_]).set_from("code", "code")])
        }
        "syncid" => {
            let (path, peer) = (st.ls("syncPath"), st.ls("syncPeer"));
            if path.is_empty() || peer.is_empty() {
                return Err("Pick a folder and a device first".into());
            }
            Ok(vec![act_v(H::Hs, vec!["sync-add".into(), path, peer.clone(), text.trim().into()]).ok(format!("Offered to {peer}: accept it there")).set("syncAdding", Value::Null)])
        }
        _ => Ok(Vec::new()),
    }
}

fn when_ms(msv: f64) -> String {
    when(ms(msv))
}

// ------------------------------------------------------------------ Drives & undo
fn format_request(st: &St, dry: bool) -> String {
    let plan = st.get("plan");
    let label = st.txt("label").trim().to_string();
    json!({
        "device": st.ls("pick"), "use": if st.ls("use").is_empty() { "storage".to_string() } else { st.ls("use") },
        "label": if label.is_empty() { "Storage".to_string() } else { label }, "token": s(plan, "token"),
        "confirm1": st.ln("step") >= 1.0, "confirm2": s(plan, "confirm"), "target": st.ls("target"), "dryRun": dry
    })
    .to_string()
}

pub fn drives<'a>(p: &Pal, st: &'a St) -> E<'a, Msg> {
    let stv = st.get("status");
    let cfg = at(stv, "settings");
    let drv = st.get("drives");
    let all = arr(drv, "drives");
    let shares = arr(stv, "shares");
    let mut rs = Rows::new();
    rs.add(words(p, "Space, health and temperature for every drive, and snapshots to bring back deleted or changed files."));
    rs.add(h(p, "Drives"));
    if drv.is_null() {
        rs.add(words(p, "Looking at your drives…"));
    } else if b(drv, "error") {
        rs.add(words(p, format!("Couldn't read the drives: {}", s(drv, "error"))));
    }
    for d in all {
        let smart = at(d, "smart");
        let value = if !smart.is_object() { "No health data" } else if at(smart, "passed") == &Value::Bool(false) { "Failing: back it up" } else if s(smart, "level") == "watch" { "Wearing: back it up soon" } else if b(smart, "passed") { "Good" } else { "Unknown" };
        let danger = smart.is_object() && (at(smart, "passed") == &Value::Bool(false) || s(smart, "level") == "watch");
        let signs: Vec<String> = arr(smart, "signs").iter().map(vs).collect();
        let mounts: Vec<String> = arr(d, "mounts").iter().map(vs).collect();
        let sub = join(&[
            format!("{} {}", s(d, "sizeText"), s(d, "kind")),
            if n(smart, "temp") > 0.0 { format!("{} °C", s(smart, "temp")) } else { String::new() },
            signs.join(", "),
            if n(d, "total") > 0.0 { format!("{} of {} used", size(n(d, "used")), size(n(d, "total"))) } else if b(d, "empty") { "empty".into() } else { "not in use".into() },
            if b(d, "system") { "Hearth OS lives here: never shared, formatted or picked".into() } else { mounts.join(", ") },
        ], " · ");
        rs.add(r("store", format!("{}{}", s(d, "model"), if b(d, "system") { " (system)" } else { "" })).value(value).danger(danger).sub(sub).el(p));
    }
    for w in arr(st.get("cands"), "drives").iter().filter(|w| !shares.iter().any(|x| s(x, "path") == s(w, "path"))) {
        let path = s(w, "path");
        rs.add(r("folder", format!("Share the whole drive: {}", s(w, "name"))).chip("Whole drive", Ck::Ok).sub(format!("{path} · everything on it, read only to start, on your home network and Tailscale; hidden files stay out")).on(act_v(H::Hs, vec!["share-whole".into(), "drive".into(), path.clone()]).ok("Shared ✓ {share.name} (Shared folders has its settings)").msg()).el(p));
    }
    if has(stv, "tools") && !b(stv, "tools.smartctl") {
        rs.add(words(p, "Drive health (smartmontools) comes with the Hearth OS update that brings Home server."));
    }

    rs.add(h(p, "Snapshots (undo)"));
    if cfg.is_object() {
        let snaps_on = b(cfg, "snapshots");
        rs.add(r("restart", "Snapshots").sub("Read-only copies of your shares' drives, taken every hour, day and week, to bring back what was deleted or changed. Only on btrfs drives; with Light mode they wait while you play.").switch(p, snaps_on, Some(act(H::Hs, &["set", "snapshots", if snaps_on { "off" } else { "on" }]).msg())).el(p));
        if snaps_on {
            for (title, key, path, list) in [
                ("Keep hourly", "keepHourly", "keep.hourly", [("0", "None"), ("12", "12"), ("24", "24"), ("48", "48")]),
                ("Keep daily", "keepDaily", "keep.daily", [("0", "None"), ("7", "7"), ("14", "14"), ("30", "30")]),
                ("Keep weekly", "keepWeekly", "keep.weekly", [("0", "None"), ("4", "4"), ("8", "8"), ("12", "12")]),
            ] {
                let cur = s(cfg, path);
                let items = list.iter().map(|(v, l)| (l.to_string(), *v == cur, Some(act(H::Hs, &["set", key, *v]).msg()))).collect();
                rs.add(choice(p, "", title, "", items));
            }
        }
    }
    for x in shares {
        let snap = b(x, "snapshots");
        let can = b(x, "snapOk") || snap;
        rs.add(r("folder", format!("Take snapshots of {}", s(x, "name"))).sub(if b(x, "snapOk") { s(x, "path") } else { s(x, "snapWhy") }).switch(p, snap, can.then(|| act_v(H::Hs, vec!["share-set".into(), s(x, "id"), "snapshots".into(), if snap { "off" } else { "on" }.into()]).msg())).el(p));
    }
    if stv.is_object() && shares.is_empty() && !b(stv, "error") {
        rs.add(words(p, "Share a folder first (Shared folders): snapshots are taken of the drives your shares are on."));
    }
    let snap_share = st.ls("snapShare");
    let snaps = st.get("snaps");
    for x in shares.iter().filter(|x| b(x, "snapshots")) {
        let id = s(x, "id");
        let sub = if snap_share == id { if snaps.is_object() { if b(snaps, "ok") { format!("{} snapshots, newest first", arr(snaps, "snapshots").len()) } else { s(snaps, "why") } } else { "Looking…".into() } } else { "See its snapshots".into() };
        let m = Msg::Run(Box::new(Act { store: Some("snaps".into()), set: vec![("snapShare".into(), json!(id.clone())), ("snapSel".into(), Value::Null)], ..act_v(H::Hs, vec!["snapshots".into(), id.clone()]).no_reload().fail_set("snapShare", json!(id.clone())) }));
        rs.add(r("clock", format!("Browse & restore {}", s(x, "name"))).sub(sub).on(m).el(p));
    }
    let snap_sel = st.ls("snapSel");
    if b(snaps, "ok") {
        for sn in arr(snaps, "snapshots").iter().take(24) {
            let name = s(sn, "name");
            rs.add(r("clock", when_ms(n(sn, "at"))).value(s(sn, "kind")).sub(if snap_sel == name { "Below: browse it, or restore it into a new folder" } else { "Select to browse or restore" }).on(pick(st, "snapSel", &name)).el(p));
        }
    }
    if !snap_sel.is_empty() {
        if let Some(sn) = arr(snaps, "snapshots").iter().find(|x| s(x, "name") == snap_sel) {
            rs.add(r("open", "Browse this snapshot").sub("Opens it in Hearth Files (read only): copy back what you need").on(Msg::Show(s(sn, "path"))).el(p));
        }
        rs.add(r("restart", "Restore into a new folder").sub("Copies the folder as it was into Home › Restored from snapshots. Nothing of yours is overwritten.").on(act_v(H::Hs, vec!["restore".into(), snap_share.clone(), snap_sel.clone()]).ok("Restoring into {dest} (you'll get a notification when it's done)").no_reload().msg()).el(p));
    }

    rs.add(h(p, "Add a drive"));
    let addable: Vec<&Value> = all.iter().filter(|d| !b(d, "system") && arr(d, "mounts").is_empty()).collect();
    let mirror_targets: Vec<String> = all.iter().filter(|d| !b(d, "system")).flat_map(|d| arr(d, "mounts").iter().map(vs).filter(|m| m.starts_with("/var/mnt/") || m.starts_with("/mnt/")).collect::<Vec<_>>()).collect();
    if drv.is_object() && !b(drv, "error") && addable.is_empty() {
        rs.add(words(p, "No drive to add right now. Plug one in (a drive that's in use has to be ejected in Hearth Files first). The system drive can never be picked."));
    }
    let pick_ = st.ls("pick");
    for d in &addable {
        let path = s(d, "path");
        let m = Msg::Run(Box::new(Act { set: vec![("pick".into(), json!(path.clone())), ("step".into(), Value::Null)], store: Some("plan".into()), ..act_v(H::Hs, vec!["format-plan".into(), path.clone()]).no_reload() }));
        rs.add(r("plus", format!("{} · {}{}", s(d, "model"), s(d, "sizeText"), if b(d, "empty") { " (empty)" } else { "" })).sub(if pick_ == path { "Set it up below" } else { "Use it as more storage, a mirror or a backup target" }).on(m).el(p));
    }
    let step = st.ln("step") as i64;
    let plan = st.get("plan");
    let use_ = if st.ls("use").is_empty() { "storage".to_string() } else { st.ls("use") };
    if !pick_.is_empty() && step == 0 {
        let sub = match use_.as_str() {
            "mirror" => "A second copy of a drive: if one breaks, nothing is lost (btrfs RAID1)",
            "backup" => "Where backups go (Backups & sync, next update)",
            _ => "A new drive for your shares and games",
        };
        let first_target = mirror_targets.first().cloned().unwrap_or_default();
        rs.add(choice(p, "", "Use it as", sub, vec![
            ("More storage".into(), use_ == "storage", Some(lv("use", json!("storage")))),
            ("Mirror".into(), use_ == "mirror", (!mirror_targets.is_empty()).then(|| Msg::Locals(vec![("use".into(), json!("mirror")), ("target".into(), json!(first_target.clone()))]))),
            ("Backup target".into(), use_ == "backup", Some(lv("use", json!("backup")))),
        ]));
        if use_ == "mirror" {
            let cur = st.ls("target");
            let items = mirror_targets.iter().map(|m| (m.trim_start_matches("/var/mnt/").trim_start_matches("/mnt/").to_string(), *m == cur, Some(lv("target", json!(m))))).collect();
            rs.add(choice(p, "", "Mirror of", "", items));
        } else {
            rs.add(tfield(p, st, "label", "Its name (letters, digits, - or _), like Storage", false));
        }
    }
    if !pick_.is_empty() && plan.is_object() && !b(plan, "canFormat") {
        rs.add(r("info", "Can't use this drive").sub(if b(plan, "error") { s(plan, "error") } else { s(plan, "why") }).el(p));
    }
    if !pick_.is_empty() && b(plan, "canFormat") {
        if step < 2 {
            let txt = format!("{}{}. Hearth asks twice. The system drive can never be picked.", if use_ == "mirror" { "Setting it up erases everything on " } else { "Formatting erases everything on " }, s(plan, "confirm"));
            rs.add(container(t(txt, sz(13.0), p.text)).padding(14.0).width(Length::Fill).style(ui::boxed(Pal::mix(p.panel, p.flame, 0.12), p.r(12.0), None)).into());
        }
        if step == 0 {
            rs.add(r("check", "Set up this drive…").sub("First of two confirmations").on(lv("step", json!(1))).el(p));
            let dry = act(H::Hs, &["format"]).input(format_request(st, true)).no_reload().set_from("dry", "steps");
            rs.add(r("info", "Show what it would do").sub("A dry run: nothing is changed").on(dry.msg()).el(p));
            let steps = arr(st.loc("dry"), "");
            if !steps.is_empty() {
                let words_: Vec<String> = steps.iter().map(|x| arr(x, "").iter().map(vs).collect::<Vec<_>>().join(" ")).collect();
                rs.add(words(p, format!("It would run: {}", words_.join("; "))));
            }
        }
        if step == 1 {
            let ok = if use_ == "mirror" { "Mirroring started: it copies in the background at low priority" } else { "Ready ✓ at {mount}" };
            let a = act(H::Hs, &["format"]).input(format_request(st, false)).ok(ok).set("step", json!(2)).fail_set("step", Value::Null).secs(900).also(rd("drives", H::Hs, &["drives"]));
            rs.add(r("trash", format!("Erase {} now", s(plan, "confirm"))).danger(true).sub("Second confirmation. Everything on this drive is deleted.").on(a.msg()).el(p));
        }
    }
    if !pick_.is_empty() && step < 2 {
        rs.add(r("close", "Not now").on(Msg::Locals(vec![("pick".into(), Value::Null), ("step".into(), Value::Null)])).el(p));
    }
    rs.add(message(p, st));
    rs.el()
}

// ------------------------------------------------------------------ Backups & sync
fn share_names(stv: &Value, ids: &[Value]) -> String {
    let all = arr(stv, "shares");
    let v: Vec<String> = ids.iter().filter_map(|i| all.iter().find(|x| at(x, "id") == i).map(|x| s(x, "name"))).collect();
    if v.is_empty() { "no folders".into() } else { v.join(", ") }
}
fn where_text(t_: &Value) -> String {
    match s(t_, "kind").as_str() {
        "drive" => s(t_, "path"),
        "hearth" => format!("{} (Hearth, over Tailscale)", s(t_, "peer")),
        "" => String::new(),
        _ => format!("{}: {}", if s(t_, "provider") == "b2" { "Backblaze B2".to_string() } else if !s(t_, "remote").is_empty() { format!("rclone {}", s(t_, "remote")) } else { "S3".into() }, s(t_, "path")),
    }
}
fn every_text(j: &Value) -> String {
    match s(j, "every").as_str() {
        "hourly" => "every hour".into(),
        "weekly" => format!("weekly at {}", s(j, "at")),
        _ => format!("daily at {}", s(j, "at")),
    }
}
fn state_text(j: &Value) -> String {
    let st_ = at(j, "state");
    if b(st_, "running") {
        "Backing up now…".into()
    } else if b(st_, "waiting") {
        "Waiting for your game".into()
    } else if at(st_, "last.ok") == &Value::Bool(false) {
        "Didn't finish".into()
    } else if n(st_, "lastOk") > 0.0 {
        format!("✓ {}", ago(n(st_, "lastOk")))
    } else if !nf(j, "on") {
        "Off".into()
    } else {
        "Not yet".into()
    }
}
fn last_line(j: &Value) -> String {
    let st_ = at(j, "state");
    if at(st_, "last.ok") == &Value::Bool(false) {
        return format!("Last try {}: {}", ago(n(st_, "last.at")), if s(st_, "last.error").is_empty() { "it didn't finish".into() } else { s(st_, "last.error") });
    }
    if n(st_, "lastOk") <= 0.0 {
        return format!("No backup yet{}", if b(st_, "waiting") { format!(": {}", s(st_, "waiting")) } else { String::new() });
    }
    let l = at(st_, "last");
    format!(
        "Last backup {}{}{}{}",
        ago(n(st_, "lastOk")),
        if n(l, "files") > 0.0 { format!(" · {} files, {}", s(l, "files"), size(n(l, "bytes"))) } else { String::new() },
        if n(st_, "count") > 0.0 { format!(" · {} kept", s(st_, "count")) } else { String::new() },
        if has(st_, "checked") { format!(" · checked {}{}", ago(n(st_, "checked.at")), if b(st_, "checked.ok") { " (every file readable)" } else { " (found a problem)" }) } else { String::new() }
    )
}

pub fn backups<'a>(p: &Pal, st: &'a St) -> E<'a, Msg> {
    let stv = st.get("status");
    let cfg = at(stv, "settings");
    let jobs = arr(stv, "backups");
    let sel = st.ls("sel");
    let cur = jobs.iter().find(|j| s(j, "id") == sel && !sel.is_empty());
    let rc = at(cfg, "receive");
    let peers = arr(stv, "tailscale.peers");
    let sync = st.get("sync");
    let cands = arr(st.get("cands"), "candidates");
    let mut rs = Rows::new();
    rs.add(words(p, "Back up your shared folders to a backup drive, another Hearth device or the cloud, encrypted with your password. Restoring always goes to a new folder: nothing is ever written over your files."));
    if has(stv, "tools") && !b(stv, "tools.restic") {
        rs.add(words_c("restic (backups) isn't on this PC yet: it comes with the Hearth OS update that brings it. What you set up here is kept.", p.flame));
    }
    if nf(cfg, "lightMode") && b(stv, "playing") {
        rs.add(words_c("You're playing: backups wait until you stop (Light mode, on the Overview page).", p.flame));
    }
    if b(stv, "error") {
        rs.add(words_c(s(stv, "error"), p.danger));
    }
    rs.add(h(p, "Back up this PC"));
    for j in jobs {
        let id = s(j, "id");
        let sub = format!("{} → {} · {}{}\n{}", share_names(stv, arr(j, "shares")), where_text(at(j, "target")), every_text(j), if nf(j, "on") { "" } else { " · off" }, last_line(j));
        rs.add(r("update", s(j, "name")).sub(sub).value(if sel == id { "Open below".to_string() } else { state_text(j) }).on(Msg::Locals(vec![("sel".into(), if sel == id { Value::Null } else { json!(id) }), ("removing".into(), Value::Null), ("pwOpen".into(), Value::Null), ("snapsOpen".into(), Value::Null)])).el(p));
    }
    if stv.is_object() && jobs.is_empty() && !b(stv, "error") {
        rs.add(words(p, "No backups yet. A second copy somewhere else keeps your files safe from a broken drive, a deleted folder or ransomware."));
    }
    if let Some(j) = cur {
        let id = s(j, "id");
        let bs = |k: &str, v: &str| act_v(H::Hs, vec!["backup-set".into(), id.clone(), k.into(), v.into()]).msg();
        rs.add(h(p, format!("“{}”", s(j, "name"))));
        let on = nf(j, "on");
        rs.add(r("update", "On").sub(format!("Runs {} (when the PC is on; with Light mode, after you stop playing)", every_text(j))).switch(p, on, Some(bs("on", if on { "off" } else { "on" }))).el(p));
        let every = s(j, "every");
        rs.add(choice(p, "", "How often", "", opts(&[("hourly", "Every hour", false), ("daily", "Daily", false), ("weekly", "Weekly (Sunday)", false)], &every, |v| bs("every", v))));
        if every != "hourly" {
            let at_ = s(j, "at");
            rs.add(choice(p, "", "At", "", opts(&[("01:00", "1:00", false), ("03:00", "3:00", false), ("05:00", "5:00", false), ("13:00", "13:00", false), ("22:00", "22:00", false)], &at_, |v| bs("at", v))));
        }
        let keep = if n(j, "keep.daily") > 0.0 { s(j, "keep.daily") } else { "7".into() };
        rs.add(choice(p, "", "Keep", "Older backups are thinned out: the newest each day, week and month are kept", opts(&[("7", "7 days", false), ("14", "14 days", false), ("30", "30 days", false)], &keep, |v| bs("keepDaily", v))));
        for x in arr(stv, "shares") {
            let sid = s(x, "id");
            let inn = arr(j, "shares").iter().any(|i| vs(i) == sid);
            rs.add(r("folder", s(x, "name")).sub(s(x, "path")).switch(p, inn, Some(act_v(H::Hs, vec!["backup-share".into(), id.clone(), sid, if inn { "off" } else { "on" }.into()]).msg())).el(p));
        }
        let running = b(j, "state.running");
        let playing_note = if nf(cfg, "lightMode") && b(stv, "playing") { "It runs once you stop playing (Light mode)" } else { "Backing up ✓ It runs in the background" };
        rs.add(r("forward", if running { "Backing up now…" } else { "Back up now" }).sub(last_line(j)).on(act_v(H::Hs, vec!["backup-now".into(), id.clone()]).ok(playing_note).msg()).el(p));
        let snaps_open = st.lb("snapsOpen");
        rs.add(r("restart", "Restore…").sub("Pick a backup, then a folder: it's copied into Home › Restored from backups, never over your files").on(Msg::Run(Box::new(Act { set: vec![("snapsOpen".into(), json!(true)), ("snapSel".into(), Value::Null)], store: Some("snaps".into()), ..act_v(H::Hs, vec!["backup-snapshots".into(), id.clone()]).no_reload().wait("Reading the backup…").secs(120) }))).el(p));
        if snaps_open {
            let snaps = arr(st.get("snaps"), "snapshots");
            let snap_sel = st.ls("snapSel");
            for sn in snaps.iter().take(12) {
                let sid = s(sn, "id");
                let paths = arr(sn, "paths");
                rs.add(r("clock", s(sn, "time").replace('T', " ").chars().take(16).collect::<String>()).sub(format!("{} · from {}", plural(paths.len() as f64, "folder", "folders"), s(sn, "host"))).value(if snap_sel == sid { "Pick a folder below" } else { "" }).on(pick(st, "snapSel", &sid)).el(p));
            }
            if let Some(sn) = snaps.iter().find(|x| s(x, "id") == snap_sel && !snap_sel.is_empty()) {
                for path in arr(sn, "paths") {
                    let path = vs(path);
                    rs.add(r("folder", format!("Restore {}", path.rsplit('/').next().unwrap_or(""))).sub(format!("{path} → a new folder in Home › Restored from backups")).on(act_v(H::Hs, vec!["backup-restore".into(), id.clone(), snap_sel.clone(), path.clone()]).ok("Restoring into {dest} (a notification says when it's done)").no_reload().msg()).el(p));
                }
            }
        }
        rs.add(r("lock", "Change its password").sub("The new password is used from the next backup on (restic keeps the old one working too)").on(toggle(st, "pwOpen")).el(p));
        if st.lb("pwOpen") {
            rs.add(tfield(p, st, "jobpw", "Its password (at least 8 characters), then Enter", true));
        }
        if st.ls("removing") != id {
            rs.add(r("close", "Remove this backup").on(lv("removing", json!(id.clone()))).el(p));
        } else {
            rs.add(r("close", format!("Remove “{}”", s(j, "name"))).danger(true).sub("It stops running, and its password and keys are forgotten on this PC. What it backed up stays where it is.").on(act_v(H::Hs, vec!["backup-remove".into(), id.clone()]).ok("Removed (the backup itself is still where it went)").set("sel", Value::Null).set("removing", Value::Null).msg()).el(p));
        }
    }

    // Adding one.
    let adding = st.lb("adding");
    let start_add = if adding {
        Msg::Locals(vec![("adding".into(), Value::Null), ("newTarget".into(), Value::Null)])
    } else {
        let mut a = act(H::Hs, &["backup-targets"]).store("targets").no_reload();
        a.set = vec![("adding".into(), json!(true)), ("newTarget".into(), Value::Null), ("newShares".into(), json!(arr(stv, "shares").iter().map(|x| s(x, "id")).collect::<Vec<_>>()))];
        a.fail_set = a.set.clone();
        a.msg()
    };
    rs.add(r("plus", "Add a backup").sub("To a backup drive, another Hearth device on Tailscale, or the cloud (Backblaze B2 or S3)").on(start_add).el(p));
    if adding {
        let targets = st.get("targets");
        let nt = st.loc("newTarget");
        rs.add(step_title(p, "1. Where it goes"));
        for d in arr(targets, "drives") {
            let chosen = s(nt, "path") == s(d, "path") && !s(d, "path").is_empty();
            rs.add(r("store", s(d, "label")).sub(format!("{}{}", s(d, "path"), if n(d, "free") > 0.0 { format!(" · {} free", size(n(d, "free"))) } else { String::new() })).value(if chosen { "✓" } else { "" }).on(lv("newTarget", { let mut x = d.clone(); if s(&x, "kind").is_empty() { x["kind"] = json!("drive"); } x })).el(p));
        }
        if targets.is_object() && arr(targets, "drives").is_empty() {
            rs.add(words(p, "No backup drive yet: add one in Drives & undo (Add a drive › Backup target), or plug one in."));
        }
        for d in arr(targets, "devices") {
            let chosen = !s(d, "peer").is_empty() && s(nt, "peer") == s(d, "peer");
            rs.add(r("desktops", format!("{}{}", s(d, "label"), if b(d, "online") { "" } else { " (offline)" })).sub("Another Hearth device, over Tailscale. Turn on “Backups from your other Hearth devices” there and make a code for this PC").value(if chosen { "✓" } else { "" }).on(lv("newTarget", { let mut x = d.clone(); if s(&x, "kind").is_empty() { x["kind"] = json!("hearth"); } x })).el(p));
        }
        rs.add(r("globe", "The cloud").sub("Backblaze B2 or any S3-compatible storage, with your own account. Encrypted here before it leaves").value(if s(nt, "kind") == "cloud" { "✓" } else { "" }).on(lv("newTarget", json!({ "kind": "cloud", "label": "The cloud" }))).el(p));
        if s(nt, "kind") == "hearth" {
            rs.add(tfield(p, st, "code", "The backup code it shows (like ally-3f9a:ABCD-EFGH-JKLM-NPQR)", false));
        }
        if s(nt, "kind") == "cloud" {
            let provider = if st.ls("provider").is_empty() { "b2".to_string() } else { st.ls("provider") };
            rs.add(choice(p, "", "Cloud storage", "", opts(&[("b2", "Backblaze B2", false), ("s3", "S3-compatible", false)], &provider, |v| lv("provider", json!(v)))));
            rs.add(tfield(p, st, "bucket", "Bucket and folder, like my-bucket/hearth", false));
            if provider == "s3" {
                rs.add(tfield(p, st, "endpoint", "Endpoint, like s3.eu-central-003.backblazeb2.com (empty for Amazon)", false));
            }
            rs.add(tfield(p, st, "keyid", if provider == "b2" { "Key ID" } else { "Access key" }, false));
            rs.add(tfield(p, st, "keysecret", if provider == "b2" { "Application key (kept only in a root-only file)" } else { "Secret key (kept only in a root-only file)" }, true));
        }
        rs.add(step_title(p, "2. What and how often"));
        let new_shares: Vec<String> = arr(st.loc("newShares"), "").iter().map(vs).collect();
        for x in arr(stv, "shares") {
            let sid = s(x, "id");
            let inn = new_shares.contains(&sid);
            let next: Vec<String> = if inn { new_shares.iter().filter(|i| **i != sid).cloned().collect() } else { let mut v = new_shares.clone(); v.push(sid.clone()); v };
            rs.add(r("folder", s(x, "name")).sub(s(x, "path")).switch(p, inn, Some(lv("newShares", json!(next)))).el(p));
        }
        if arr(stv, "shares").is_empty() {
            rs.add(words(p, "Share a folder first (Shared folders): backups are of your shared folders."));
        }
        let every = if st.ls("newEvery").is_empty() { "daily".to_string() } else { st.ls("newEvery") };
        rs.add(choice(p, "", "How often", "", opts(&[("hourly", "Every hour", false), ("daily", "Daily at 3:00", false), ("weekly", "Weekly", false)], &every, |v| lv("newEvery", json!(v)))));
        rs.add(step_title(p, "3. A password for it. Write it down: restoring on another PC needs it, and nobody can get it back for you."));
        rs.add(tfield(p, st, "newpw", "Backup password (at least 8 characters), then Enter to add it", true));
    }

    rs.add(h(p, "Devices backing up here"));
    rs.add(r("desktops", "Your Hearth devices' saves and screenshots").sub("They back up to this PC through Hearth Storage (Hearth Link)").chev().on(Msg::Go("link")).el(p));
    if cfg.is_object() {
        let on = b(rc, "on");
        let can = on || !s(rc, "path").is_empty();
        rs.add(r("update", "Backups from your other Hearth devices").sub(if on { format!("On, over Tailscale only, into {}. Each device has its own code and its own folder", if s(rc, "path").is_empty() { "the folder below".into() } else { s(rc, "path") }) } else { "Lets your other Hearth devices back up their shares here, over Tailscale only (with a code each)".into() }).switch(p, on, can.then(|| act(H::Hs, &["set", "receiveOn", if on { "off" } else { "on" }]).ok(if on { "Off (what they backed up stays)" } else { "On: make a code for each device below" }).msg())).el(p));
        rs.add(r("folder", "Where they go").value(if s(rc, "path").is_empty() { "Pick a folder".to_string() } else { s(rc, "path") }).sub("A folder on a big drive is best (each device gets its own folder inside)").on(toggle(st, "receivePick")).el(p));
    }
    if st.lb("receivePick") {
        for c in cands.iter().take(8) {
            let path = s(c, "path");
            rs.add(r(if b(c, "drive") { "store" } else { "folder" }, s(c, "name")).sub(path.clone()).on(act_v(H::Hs, vec!["receive-path".into(), path.clone()]).ok(format!("Backups from your devices go to {path}")).set("receivePick", Value::Null).msg()).el(p));
        }
    }
    for d in arr(rc, "devices") {
        let user = s(d, "user");
        rs.add(r("desktops", s(d, "name")).sub(format!("Its code starts with {user} · made {}. Select to take its code away (what it backed up stays)", date(ms(n(d, "at"))))).on(act_v(H::Hs, vec!["receive-remove".into(), user.clone()]).ok(format!("{} can't back up here anymore", s(d, "name"))).msg()).el(p));
    }
    if b(rc, "on") {
        rs.add(tfield(p, st, "recvname", "Make a code for a device: type its name (like rog-ally), then Enter", false));
    }
    if !st.ls("code").is_empty() {
        let c = column![t("Type this on the other device (Backups & sync › Add a backup › this PC). It's shown only now.", sz(12.0), p.muted), tb(st.ls("code"), sz(20.0), p.text).font(iced::Font::MONOSPACE)].spacing(6.0);
        rs.add(container(c).padding(14.0).width(Length::Fill).style(ui::boxed(p.panel2, p.r(12.0), None)).into());
    }

    rs.add(h(p, "Phone photos"));
    let phone_addr = arr(stv, "shares").first().map(|x| s(x, "how.phone")).filter(|x| !x.is_empty()).unwrap_or_else(|| "smb://this-pc/Photos".into());
    rs.add(r("cam", "Android").sub(format!("An app that backs up to a network folder (SMB), like FolderSync or PhotoSync: point it at {phone_addr} with your user name and sharing password (or a person's, People page). Or Syncthing-Fork, with folder sync below.")).el(p));
    rs.add(r("cam", "iPhone").sub(format!("PhotoSync (or any app that uploads to SMB): add this PC as an SMB server ({}), pick a read & write share, and turn on automatic transfer. Away from home it works through Tailscale too.", if s(stv, "host").is_empty() { "this-pc".into() } else { s(stv, "host") })).el(p));

    rs.add(h(p, "Folders kept the same on your Hearth devices"));
    if sync.is_object() && !b(sync, "installed") {
        rs.add(words(p, "Syncthing (folder sync) isn't on this PC yet: it comes with the Hearth OS update that brings it."));
    }
    if b(sync, "installed") {
        let id = s(sync, "id");
        rs.add(r("info", if id.is_empty() { "Show this PC's sync ID" } else { "This PC's sync ID" }).sub(if id.is_empty() { "Your other device needs it to sync with this PC (this starts folder sync, which only talks over Tailscale)".to_string() } else { id.clone() }).on_maybe(id.is_empty().then(|| act(H::Hs, &["sync-id"]).msg())).el(p));
        for f in arr(sync, "folders") {
            let value = if !has(f, "completion") { String::new() } else if n(f, "completion") >= 100.0 { "Up to date".into() } else { format!("Syncing {} files", s(f, "need")) };
            let dev: String = s(f, "device").chars().take(7).collect();
            rs.add(r("restart", s(f, "label")).sub(format!("{} ↔ {}{}. Select to stop syncing (the files stay on both)", s(f, "path"), if s(f, "peer").is_empty() { dev } else { s(f, "peer") }, if b(f, "connected") { " · connected" } else { " · not connected" })).value(value).on(act_v(H::Hs, vec!["sync-remove".into(), s(f, "id")]).ok("Not synced anymore (the files stay)").msg()).el(p));
        }
        for o in arr(sync, "offers") {
            let dev: String = s(o, "device").chars().take(7).collect();
            rs.add(r("plus", format!("Accept “{}”", s(o, "label"))).sub(format!("Offered by {dev}…: it's kept in Home › Synced › {}", s(o, "label"))).on(act_v(H::Hs, vec!["sync-accept".into(), s(o, "id"), s(o, "device")]).ok(format!("Syncing {}", s(o, "label"))).msg()).el(p));
        }
        rs.add(r("plus", "Sync a folder").sub("Pick a folder, one of your devices on Tailscale and its sync ID. Changed or deleted files are kept 30 days first").on(Msg::Locals(vec![("syncAdding".into(), if st.lb("syncAdding") { Value::Null } else { json!(true) }), ("syncPath".into(), Value::Null), ("syncPeer".into(), Value::Null)])).el(p));
    }
    if st.lb("syncAdding") {
        for c in cands.iter().filter(|c| !b(c, "drive")).take(8) {
            let path = s(c, "path");
            rs.add(r("folder", s(c, "name")).sub(path.clone()).value(if st.ls("syncPath") == path { "✓" } else { "" }).on(lv("syncPath", json!(path))).el(p));
        }
        for pe in peers.iter().filter(|x| s(x, "os") == "linux") {
            let name = s(pe, "name");
            rs.add(r("desktops", format!("{name}{}", if b(pe, "online") { "" } else { " (offline)" })).value(if st.ls("syncPeer") == name { "✓" } else { "" }).on(lv("syncPeer", json!(name))).el(p));
        }
        rs.add(tfield(p, st, "syncid", "Its sync ID (Backups & sync on that device), then Enter", false));
    }
    if at(stv, "applied.ok") == &Value::Bool(false) {
        rs.add(words_c(format!("Saved, but the system couldn't be changed to match: {}", s(stv, "applied.error")), p.danger));
    }
    rs.add(message(p, st));
    rs.el()
}

// ------------------------------------------------------------------ Safety
fn where_word(w: &str) -> &'static str {
    match w {
        "tailscale" => "Tailscale only",
        "local" => "This PC only",
        "home" => "Home network and Tailscale",
        _ => "Every network",
    }
}

pub fn safety<'a>(p: &Pal, st: &'a St) -> E<'a, Msg> {
    let sf = st.get("safety");
    let guard = at(sf, "guard");
    let mut rs = Rows::new();
    rs.add(words(p, "Who can reach what on this PC, and simple fixes. Every fix says what it changes and can be undone."));
    if b(sf, "error") {
        rs.add(words_c(s(sf, "error"), p.danger));
    }
    if b(guard, "tripped") {
        rs.add(r("lock", "Your shares are read only").danger(true).sub(format!("{} files were renamed or deleted in 5 minutes at {}. Check your devices first. Snapshots (Drives & undo) bring back what changed", s(guard, "count"), when(n(guard, "tripped")))).el(p));
        rs.add(r("restart", "Make them writable again").sub("It was you (a big move or clean up), or it's dealt with").on(act(H::Hs, &["guard-reset"]).ok("Your shares are writable again").msg()).el(p));
    }
    rs.add(h(p, "Worth a look"));
    for f in arr(sf, "findings") {
        let fix = s(f, "fix");
        let page = s(f, "page");
        let mut row_ = r(if s(f, "level") == "warn" { "bell" } else { "info" }, s(f, "title"))
            .danger(s(f, "level") == "warn" && fix.is_empty() && page.is_empty())
            .sub(format!("{}{}", s(f, "sub"), if fix.is_empty() { String::new() } else { format!(" · Fix: {fix}") }))
            .value(if !fix.is_empty() { "Fix" } else if !page.is_empty() { "Open" } else { "" });
        if !page.is_empty() {
            let target = crate::registry::page(&page).map(|x| x.id);
            row_ = row_.chev().on_maybe(target.map(Msg::Go));
        } else if !fix.is_empty() {
            row_ = row_.on(act_v(H::Hs, vec!["fix".into(), s(f, "id")]).ok(format!("Done: {fix}. Undo is below")).msg());
        }
        rs.add(row_.el(p));
    }
    if sf.is_object() && !b(sf, "error") && arr(sf, "findings").is_empty() {
        rs.add(r("check", "Nothing to fix").sub("Your shares, services and backups look right").el(p));
    }
    for d in arr(sf, "done") {
        let id = s(d, "id");
        let name = match id.as_str() {
            "ts-only" => "Everything Tailscale only".to_string(),
            "guard" => "Ransomware guard on".into(),
            "media-ts" => "Media server Tailscale only".into(),
            "snapshots" => "Snapshots on".into(),
            other => other.to_string(),
        };
        let k = arr(d, "steps").len() as f64;
        rs.add(r("restart", format!("Undo: {name}")).sub(format!("Done {} · puts back exactly what was there ({})", when_ms(n(d, "at")), plural(k, "change)", "changes)"))).on(act_v(H::Hs, vec!["fix-undo".into(), id]).ok("Undone").msg()).el(p));
    }
    rs.add(h(p, "At a glance"));
    if sf.is_object() && !b(sf, "error") {
        rs.add(r("globe", if b(sf, "tsOnly") { "Only reachable through Tailscale" } else { "Home network and Tailscale" }).sub(format!("Tailscale: {}{}", if s(sf, "tailscale") == "Running" { "connected".to_string() } else if s(sf, "tailscale").is_empty() { "off".into() } else { s(sf, "tailscale") }, if s(sf, "firewall").is_empty() { String::new() } else { format!(" · firewall rule: {}", s(sf, "firewall")) })).el(p));
        rs.add(r("update", "Last backup").sub(if n(sf, "lastBackup") > 0.0 { when(n(sf, "lastBackup")) } else if b(sf, "backups") { "None finished yet".into() } else { "No backup set up".into() }).chev().on(Msg::Go("hs-backups")).el(p));
        rs.add(r("lock", "Shared folders: SMB3, no guests").sub(if b(sf, "smb.known") { format!("{} · {}", if b(sf, "smb.smb3") { "SMB3 or newer only" } else { "Older SMB allowed" }, if b(sf, "smb.noGuests") { "no guest access" } else { "guest access possible" }) } else { "Samba's settings couldn't be read".into() }).el(p));
        rs.add(r("restart", "Snapshots").sub(if b(sf, "snapshots") { "On: deleted, changed or encrypted files can be brought back" } else { "Off" }).el(p));
    }
    if guard.is_object() {
        let on = b(guard, "on");
        rs.add(r("lock", "Ransomware guard").sub(if on { format!("Watching: if {} or more files are renamed or deleted on your shares within 5 minutes, they all turn read only and you're told", s(guard, "limit")) } else { "If far more files change at once than you'd ever do, your shares turn read only right away and you're told. Off until you turn it on".into() }).switch(p, on, Some(act(H::Hs, &["set", "guardOn", if on { "off" } else { "on" }]).msg())).el(p));
        if on {
            let cur = s(guard, "limit");
            rs.add(choice(p, "", "Turn shares read only after", "Files renamed or deleted through Samba within 5 minutes", opts(&[("500", "500", false), ("1000", "1,000", false), ("5000", "5,000", false)], &cur, |v| act(H::Hs, &["set", "guardLimit", v]).msg())));
        }
    }
    rs.add(h(p, "Who can reach what"));
    for x in arr(sf, "reach") {
        let w = s(x, "where");
        rs.add(r(if w == "tailscale" { "lock" } else { "wifi" }, s(x, "what")).sub(format!("{} · {}", where_word(&w), s(x, "who"))).el(p));
    }
    if sf.is_object() && !b(sf, "error") && arr(sf, "reach").is_empty() {
        rs.add(r("check", "Nothing is shared or served").el(p));
    }
    rs.add(h(p, "Open ports"));
    for x in arr(sf, "ports") {
        let w = s(x, "where");
        rs.add(r(if w == "all" { "wifi" } else if w == "tailscale" { "lock" } else { "check" }, s(x, "what")).sub(format!("{} {} · {}", s(x, "proto").to_uppercase(), s(x, "port"), where_word(&w))).el(p));
    }
    rs.add(h(p, "Updates for this PC"));
    rs.add(r("update", "The previous version stays startable").sub("Hearth OS updates always keep the version before them in the boot menu. Settings › Updates").el(p));
    rs.add(message(p, st));
    rs.el()
}

// ------------------------------------------------------------------ Alerts & power
fn mins(m: &str) -> String {
    let v: f64 = m.parse().unwrap_or(0.0);
    if v >= 60.0 {
        format!("{}{}", v / 60.0, if v == 60.0 { " hour" } else { " hours" })
    } else {
        format!("{v} minutes")
    }
}

pub fn alerts<'a>(p: &Pal, st: &'a St) -> E<'a, Msg> {
    let stv = st.get("status");
    let cfg = at(stv, "settings");
    let al = at(cfg, "alerts");
    let pw = at(cfg, "power");
    let active = arr(stv, "alerts.active");
    let ups_shown = b(stv, "seen.ups.shown");
    let set = |k: &str, v: &str, ok: &str| act(H::Hs, &["set", k, v]).ok(ok).msg();
    let flip = |k: &str, on: bool| act(H::Hs, &["set", k, if on { "off" } else { "on" }]).msg();
    let mut rs = Rows::new();
    rs.add(words(p, "Hear about problems early, and let this PC sleep when nobody needs it. Everything here is off until you turn it on."));
    if b(stv, "error") {
        rs.add(words_c(s(stv, "error"), p.danger));
    }
    rs.add(h(p, "Tell me when"));
    for a in active {
        let k = s(a, "kind");
        let at_ = n(stv, "alerts.at");
        rs.add(r("bell", s(a, "text")).danger(k == "smart" || k == "mirror").sub(format!("Now · checked {}", if at_ > 0.0 { hearth_ui::time::clock(at_ as i64) } else { String::new() })).el(p));
    }
    if al.is_object() {
        let on = b(al, "on");
        rs.add(r("bell", "Alerts").sub(if on { "On: checked every hour, and you're told once a day while something's wrong" } else { "Off. Turn on to hear about a failing drive, a full drive, a failed backup or a broken mirror" }).switch(p, on, Some(set("alertsOn", if on { "off" } else { "on" }, if on { "Alerts off" } else { "Alerts on" }))).el(p));
        if on {
            let smart = b(al, "smart");
            rs.add(r("store", "A drive starts to fail").sub("Its health check (SMART) fails, or it runs very hot (65 °C or more)").switch(p, smart, Some(flip("alertSmart", smart))).el(p));
            let full = b(al, "full");
            rs.add(r("store", "A drive is almost full").sub("A drive with a share or a backup on it").switch(p, full, Some(flip("alertFull", full))).el(p));
            if full {
                let cur = s(al, "fullPct");
                rs.add(choice(p, "", "Almost full means", "", opts(&[("80", "80%", false), ("85", "85%", false), ("90", "90%", false), ("95", "95%", false)], &cur, |v| set("alertFullPct", v, ""))));
            }
            let bk = b(al, "backup");
            rs.add(r("update", "A backup fails or is late").sub("It didn't finish, didn't run for days, or its weekly check found a problem").switch(p, bk, Some(flip("alertBackup", bk))).el(p));
            let mir = b(al, "mirror");
            rs.add(r("restart", "A mirror is missing a drive").sub("A mirrored drive (Drives & undo) lost one of its drives or has read or write errors").switch(p, mir, Some(flip("alertMirror", mir))).el(p));
            let tsn = b(al, "tailscale");
            let peers_on: Vec<String> = arr(al, "peers").iter().map(vs).collect();
            rs.add(r("globe", "Also send them to my devices").sub(if peers_on.is_empty() { "Through Taildrop (Tailscale), to the devices you pick below. Nothing goes anywhere else".to_string() } else { format!("Through Taildrop to {} (a small note in their Taildrop files). Nothing goes anywhere else", peers_on.join(", ")) }).switch(p, tsn, Some(flip("alertTailscale", tsn))).el(p));
            if tsn {
                for pe in arr(stv, "tailscale.peers") {
                    let id = if s(pe, "dns").is_empty() { s(pe, "name") } else { s(pe, "dns") };
                    let picked = peers_on.contains(&id);
                    let os = s(pe, "os");
                    rs.add(r(if os == "android" || os == "iOS" { "cam" } else { "desktops" }, s(pe, "name")).sub(format!("{} · {os}", if b(pe, "online") { "Online" } else { "Offline" })).switch(p, picked, Some(act_v(H::Hs, vec!["alert-peer".into(), id.clone(), if picked { "off" } else { "on" }.into()]).msg())).el(p));
                }
            }
            rs.add(r("bell", "Send a test alert").sub("A notification here (and a note to the devices you picked)").on(act(H::Hs, &["alerts-test"]).ok("Sent ✓").no_reload().msg()).el(p));
        }
    }

    rs.add(h(p, "Power"));
    if pw.is_object() {
        let cur = s(pw, "sleepIdle");
        rs.add(choice(p, "", "Sleep when nobody needs it for", "Never while Game Mode runs, a share is open or a backup runs. While you're signed in to the desktop, Settings › Power's own timer decides", opts(&[("0", "Never", false), ("30", "30 min", false), ("60", "1 hour", false), ("120", "2 hours", false), ("240", "4 hours", false)], &cur, |v| {
            act(H::Hs, &["set", "sleepIdle", v]).ok(if v == "0" { "It doesn't sleep by itself".to_string() } else { format!("It sleeps after {} nobody needs it", mins(v)) }).msg()
        })));
        let ka = b(pw, "keepAwake");
        rs.add(r("sleep", "Stay awake while my devices use it").sub("While a share is open or a backup runs, nothing puts it to sleep (Settings › Power's timer included)").switch(p, ka, Some(flip("keepAwake", ka))).el(p));
    }
    if cfg.is_object() {
        let wired: Vec<String> = arr(stv, "wired").iter().map(|w| s(w, "iface")).collect();
        let wol = b(cfg, "wakeFromAway");
        rs.add(r("wired", "Wake-on-LAN (wake from your devices)").sub(if wired.is_empty() { "Needs a network cable: no wired network card found".to_string() } else { format!("Your other devices can wake this PC over the network ({}). Wi-Fi rarely can: use a cable", wired.join(", ")) }).switch(p, wol, Some(flip("wakeFromAway", wol))).el(p));
    }
    if pw.is_object() {
        let sched = b(pw, "schedule");
        rs.add(r("clock", "Sleep at night, wake in the morning").sub(if sched { format!("Sleeps at {} (unless it's busy) and wakes at {} every day", s(pw, "sleepAt"), s(pw, "wakeAt")) } else { "A nightly schedule (backups run once it's awake)".into() }).switch(p, sched, Some(flip("schedule", sched))).el(p));
        if sched {
            let cur = s(pw, "sleepAt");
            rs.add(choice(p, "", "Sleep at", "", opts(&[("23:00", "23:00", false), ("00:00", "0:00", false), ("01:00", "1:00", false), ("02:00", "2:00", false)], &cur, |v| set("sleepAt", v, ""))));
            let cur = s(pw, "wakeAt");
            rs.add(choice(p, "", "Wake at", "", opts(&[("02:30", "2:30 (backups)", false), ("06:00", "6:00", false), ("07:00", "7:00", false), ("08:00", "8:00", false)], &cur, |v| set("wakeAt", v, ""))));
        }
        let cur = s(pw, "spinDown");
        rs.add(choice(p, "", "Spin down idle storage drives after", if has(stv, "tools") && !b(stv, "tools.hdparm") { "hdparm isn't on this PC yet" } else { "Spinning hard drives only, never the system drive or an SSD (hdparm)" }, opts(&[("0", "Never", false), ("10", "10 min", false), ("20", "20 min", false), ("30", "30 min", false), ("60", "1 hour", false)], &cur, |v| set("spinDown", v, ""))));
        let night = b(pw, "night");
        rs.add(r("clock", "Night maintenance").sub("Once a night: optimize drives (TRIM), tidy the system logs, update apps (Flatpak) and clear downloaded package files. Never while it's used or busy, and never anything that touches starting the PC").switch(p, night, Some(set("night", if night { "off" } else { "on" }, if night { "Night maintenance is off" } else { "Night maintenance is on" }))).el(p));
        if night {
            let cur = if s(pw, "nightAt").is_empty() { "03:30".into() } else { s(pw, "nightAt") };
            rs.add(choice(p, "", "Start at", if sched { format!("It runs only while the PC is awake: pick a time after it wakes ({}) or before it sleeps ({})", s(pw, "wakeAt"), s(pw, "sleepAt")) } else { String::new() }, opts(&[("02:30", "2:30", false), ("03:30", "3:30", false), ("04:30", "4:30", false), ("05:30", "5:30", false)], &cur, |v| set("nightAt", v, ""))));
            let nl = st.get("night");
            if nl.is_object() {
                let steps: Vec<String> = arr(nl, "steps").iter().map(|x| format!("{}{}", s(x, "step"), match at(x, "ok") { Value::Bool(true) => " ✓".to_string(), Value::Bool(false) => format!(" ✗ {}", s(x, "note")), _ => " (not here)".into() })).collect();
                rs.add(r("info", "Last night").sub(format!("{} · {}", when(n(nl, "at")), steps.join(" · "))).el(p));
            }
        }
        let hist = b(pw, "history");
        rs.add(r("power", "Power history").sub("Keeps what the processor and graphics card draw, every 5 minutes for 30 days, and shows it here").switch(p, hist, Some(flip("powerHistory", hist))).el(p));
        if hist {
            let since = hearth_ui::time::now() as f64 - 86_400.0;
            let pts: Vec<&Value> = arr(st.get("powerh"), "points").iter().filter(|x| n(x, "0") >= since).collect();
            let cpu: Vec<f32> = pts.iter().map(|x| n(x, "1") as f32).collect();
            let both: Vec<f32> = pts.iter().map(|x| (n(x, "1") + n(x, "2")) as f32).collect();
            let avg = if both.is_empty() { 0.0 } else { both.iter().sum::<f32>() / both.len() as f32 };
            let mx = both.iter().cloned().fold(10.0f32, f32::max) * 1.15;
            let head_ = if pts.is_empty() { "Collecting: the first points show within 5 minutes".to_string() } else { format!("Last 24 hours · about {} W on average · {:.2} kWh a day (processor and graphics card)", avg.round(), avg * 24.0 / 1000.0) };
            let graph = |vals: Vec<f32>, c: iced::Color| hearth_ui::ui::graph(hearth_ui::ui::Graph { values: vals, dashed: Vec::new(), max: mx, min: 0.0, color: c, grid: Pal::a(p.line, 0.6), fill: false, width: 2.0, centred: false, gridlines: Vec::new() }, Length::Fill, Length::Fixed(130.0));
            let blue = iced::Color::from_rgb8(0x66, 0xc0, 0xf4);
            let orange = iced::Color::from_rgb8(0xff, 0x9a, 0x4d);
            let g = iced::widget::stack![graph(cpu, blue), graph(both, orange)];
            rs.add(container(column![t(head_, sz(13.0), p.text), g, t("Blue: processor · orange: processor and graphics card together", sz(11.0), p.muted)].spacing(6.0)).padding(14.0).width(Length::Fill).style(ui::boxed(p.panel2, p.r(12.0), None)).into());
        }
    }
    if n(stv, "power.quietSince") > 0.0 {
        rs.add(r("info", "Right now").sub(format!("Nobody has needed it since {}", hearth_ui::time::clock(n(stv, "power.quietSince") as i64))).el(p));
    }
    if ups_shown {
        rs.add(h(p, "UPS"));
        if pw.is_object() {
            let ups = b(pw, "ups");
            rs.add(r("batt", "Shut down cleanly on battery").sub(format!("{}{}", if b(stv, "ups") { format!("{} found. ", s(stv, "ups")) } else { "No UPS plugged in right now. ".into() }, if has(stv, "tools") && !b(stv, "tools.nut") { "NUT isn't on this PC yet" } else { "When its battery runs low, this PC shuts down cleanly (Network UPS Tools, only on this PC)" })).switch(p, ups, Some(flip("ups", ups))).el(p));
            let u = at(st.get("ups"), "ups");
            if ups && u.is_object() {
                rs.add(r("batt", format!("Battery {}%", s(u, "charge"))).sub(format!("{}{}", if s(u, "status").contains("OB") { "On battery" } else { "On mains power" }, if n(u, "runtime") > 0.0 { format!(" · about {} minutes on battery", (n(u, "runtime") / 60.0).round()) } else { String::new() })).el(p));
            }
        }
    } else if stv.is_object() {
        rs.add(words(p, "UPS settings show once one is plugged in (or with Show hidden items on the Overview page)."));
    }
    if at(stv, "applied.ok") == &Value::Bool(false) {
        rs.add(words_c(format!("Saved, but the system couldn't be changed to match: {}", s(stv, "applied.error")), p.danger));
    }
    rs.add(message(p, st));
    let _ = Padding::ZERO;
    rs.el()
}
