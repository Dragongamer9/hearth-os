//! Shared folders (SetHomeShares.qml), Files & links (SetHomeFiles.qml), People (SetHomePeople.qml) and Family &
//! home (SetHomeFamily.qml).

use super::kit::*;
use hearth_ui::Pal;
use serde_json::{json, Value};

pub fn reads(page: &str, st: &St) -> Vec<Rd> {
    match page {
        "hs-shares" => {
            let mut v = vec![rd("status", H::Hs, &["status"])];
            if !st.d.contains_key("cands") {
                v.push(rd("cands", H::Hs, &["candidates"]));
            }
            v
        }
        "hs-files" => vec![rd("files", H::Hs, &["files"])],
        "hs-people" => vec![rd("status", H::Hs, &["status"])],
        _ => vec![rd("family", H::Hs, &["family"])],
    }
}

pub fn live(_page: &str, st: &St) -> bool {
    b(st.get("files"), "scanning")
}

fn add_share(st: &St, path: &str) -> Act {
    let pw_set = n(st.get("status"), "settings.smbPasswordSet") > 0.0;
    act_v(H::Hs, vec!["share-add".into(), path.to_string()])
        .ok(format!("Shared ✓ {{share.name}}{}", if pw_set { "" } else { ". Set a sharing password below so your devices can sign in" }))
        .set("adding", Value::Null)
        .set_from("sel", "share.id")
}

fn make_link(st: &St, pw: &str) -> Act {
    let mut req = json!({ "path": st.ls("linkPath"), "days": if st.l.contains_key("linkDays") { st.ln("linkDays") } else { 7.0 }, "for": st.ls("linkFor"), "max": 0 });
    if !pw.is_empty() {
        req["password"] = json!(pw);
    }
    act(H::Hs, &["link-add"]).input(req.to_string()).ok("Link made ✓ Copy it below").set("linkPath", Value::Null).set("linkPw", Value::Null)
}

pub fn submit(st: &mut St, key: &str, text: String) -> Result<Vec<Act>, String> {
    match key {
        "path" => {
            let t = text.trim().to_string();
            if t.is_empty() {
                return Ok(Vec::new());
            }
            Ok(vec![add_share(st, &t)])
        }
        "pw" => {
            if text.chars().count() < 8 {
                return Err("Use at least 8 characters".into());
            }
            Ok(vec![act(H::Hs, &["smb-password"]).input(text).ok("Sharing password set ✓").set("pwOpen", Value::Null)])
        }
        "search" => {
            let t = text.trim().to_string();
            if t.is_empty() {
                return Ok(Vec::new());
            }
            st.t.insert(key.into(), t.clone());
            Ok(vec![act_v(H::Hs, vec!["files-search".into(), t]).store("found").no_reload().secs(30)])
        }
        "linkfor" => {
            st.setl("linkFor", json!(text));
            Ok(Vec::new())
        }
        "linkpw" => {
            if text.chars().count() < 6 {
                return Err("Use at least 6 characters".into());
            }
            Ok(vec![make_link(st, &text)])
        }
        "newname" => {
            let n_ = text.trim().to_lowercase();
            let ok = n_.len() >= 2 && n_.len() <= 24 && n_.chars().next().map(|c| c.is_ascii_lowercase()).unwrap_or(false) && n_.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-');
            if !ok {
                return Err("Use 2 to 24 small letters, digits, - or _, starting with a letter".into());
            }
            st.setl("newName", json!(n_));
            st.t.insert(key.into(), String::new());
            Ok(Vec::new())
        }
        "newpw" => {
            if text.chars().count() < 8 {
                return Err("Use at least 8 characters".into());
            }
            let n_ = st.ls("newName");
            Ok(vec![act_v(H::Hs, vec!["person-add".into(), n_.clone()]).input(text).ok(format!("Added {n_} ✓ Now pick what they may open")).set("adding", Value::Null).set("newName", Value::Null).set("sel", json!(n_))])
        }
        "whopw" => {
            if text.chars().count() < 8 {
                return Err("Use at least 8 characters".into());
            }
            let sel = st.ls("sel");
            let account_missing = arr(st.get("status"), "people").iter().find(|x| s(x, "name") == sel).map(|x| at(x, "account") == &Value::Bool(false)).unwrap_or(false);
            let verb = if account_missing { "person-add" } else { "person-password" };
            Ok(vec![act_v(H::Hs, vec![verb.into(), sel]).input(text).ok("Password set ✓").set("pwOpen", Value::Null)])
        }
        _ => Ok(Vec::new()),
    }
}

// ------------------------------------------------------------------ Shared folders
fn badge(w: &str) -> &'static str {
    match w {
        "drive" => "Whole drive",
        "home" => "Home",
        "system" => "System drive · read only",
        _ => "",
    }
}

pub fn shares<'a>(p: &Pal, st: &'a St) -> E<'a, Msg> {
    let stv = st.get("status");
    let cfg = at(stv, "settings");
    let shares_ = arr(stv, "shares");
    let sel = st.ls("sel");
    let share = shares_.iter().find(|x| s(x, "id") == sel && !sel.is_empty());
    let pw_set = n(cfg, "smbPasswordSet") > 0.0;
    let ts_only = b(cfg, "tsOnly");
    let confirming = st.ls("confirming");
    let adding = st.lb("adding");
    let user = s(stv, "user");
    let mut rs = Rows::new();
    rs.add(words(p, "Folders your other devices can open: Windows (File Explorer), phones, TVs and Hearth devices. Nothing is shared until you share it, and nobody gets in without your sharing password."));
    if has(stv, "tools") && !b(stv, "tools.samba") {
        rs.add(words_c("Samba (sharing) isn't on this PC yet: it comes with the Hearth OS update that brings Home server. What you pick here is kept and starts working then.", p.flame));
    }
    if b(stv, "error") {
        rs.add(words_c(s(stv, "error"), p.danger));
    }
    rs.add(h(p, "Your shares"));
    for x in shares_ {
        let id = s(x, "id");
        let whole = s(x, "whole");
        let ic = match whole.as_str() {
            "drive" => "drive",
            "home" => "home",
            "system" => "shield",
            _ => "folder",
        };
        let mut sub = format!("{} · ", s(x, "path"));
        if !nf(x, "on") {
            sub.push_str("not shared now");
        } else {
            sub.push_str(&format!("{} · {}{}", if b(x, "write") { "read & write" } else { "read only" }, if s(x, "reach") == "tailscale" || ts_only { "Tailscale only" } else { "home network and Tailscale" }, if b(x, "write") && nf(x, "recycle") { " · recycle bin" } else { "" }));
        }
        if !b(x, "there") {
            sub.push_str(" · its folder isn't there");
        }
        let value = if sel == id { "Open below" } else if !nf(x, "on") { "Off" } else { "On" };
        rs.add(r(ic, s(x, "name")).chip(badge(&whole), if whole == "system" || whole == "home" { Ck::Warn } else { Ck::Ok }).sub(sub).value(value).on(Msg::Locals(vec![("sel".into(), if sel == id { Value::Null } else { json!(id) }), ("removing".into(), Value::Null)])).el(p));
    }
    if stv.is_object() && shares_.is_empty() && !b(stv, "error") {
        rs.add(words(p, "Nothing is shared yet."));
    }
    rs.add(r("plus", "Share a folder").sub("A folder in your Home or on a drive, or a whole drive. It starts read only, on your home network.").on(toggle(st, "adding")).el(p));
    if adding {
        let pick_add = add_share(st, "{path}");
        rs.add(r("folder", "Pick it in Hearth Files").on(act(H::Hs, &["pick-folder"]).secs(600).no_reload().then_if("path", pick_add).msg()).el(p));
        for c in arr(st.get("cands"), "candidates").iter().filter(|c| !shares_.iter().any(|x| s(x, "path") == s(c, "path"))).take(10) {
            rs.add(r(if b(c, "drive") { "store" } else { "folder" }, s(c, "name")).sub(s(c, "path")).on(add_share(st, &s(c, "path")).msg()).el(p));
        }
        let whole = |args: &[&str]| {
            let mut v = vec!["share-whole"];
            v.extend_from_slice(args);
            act(H::Hs, &v).ok("Shared ✓ {share.name} (read only)").set_from("sel", "share.id").set("confirming", Value::Null).msg()
        };
        for d in arr(st.get("cands"), "drives").iter().filter(|d| !shares_.iter().any(|x| s(x, "path") == s(d, "path"))) {
            let path = s(d, "path");
            rs.add(r("drive", format!("Share the whole drive: {}", s(d, "name"))).sub(format!("{path} · everything on it, read only to start, on your home network and Tailscale; hidden files stay out")).chip("Whole drive", Ck::Ok).on(whole(&["drive", &path])).el(p));
        }
        if !shares_.iter().any(|x| s(x, "whole") == "home") {
            let sub = if confirming == "home" { "Everything in your Home is shared, including settings folders; hidden files (.ssh, .config, .local, keyrings) stay blocked. Read only. Press again to share it" } else { "Everything in your Home, read only to start, on your home network and Tailscale. Asks first" };
            let m = if confirming == "home" { whole(&["home", "home"]) } else { lv("confirming", json!("home")) };
            rs.add(r("home", "Share your whole Home").chip("Home", Ck::Warn).sub(sub).on(m).el(p));
        }
        if !shares_.iter().any(|x| s(x, "whole") == "system") {
            let sub = match confirming.as_str() {
                "system" => "Your devices can read the system's files (secrets like passwords and keys stay blocked). Press again if you're sure",
                "system2" => "Last check: shared read only, never read & write (writing system files over the network can stop the PC from starting). Press again to share it",
                _ => "Read only, never read & write: writing system files over the network can stop the PC from starting. Asks twice",
            };
            let m = match confirming.as_str() {
                "system" => lv("confirming", json!("system2")),
                "system2" => whole(&["system", "system", "system-sure"]),
                _ => lv("confirming", json!("system")),
            };
            rs.add(r("shield", "Share the system drive (read only)").chip("System drive", Ck::Warn).sub(sub).on(m).el(p));
        }
        rs.add(tfield(p, st, "path", "Or type its full path, like /home/you/Videos", false));
    }

    if let Some(x) = share {
        let id = s(x, "id");
        let whole = s(x, "whole");
        rs.add(h(p, format!("Share “{}”", s(x, "name"))));
        let on = nf(x, "on");
        rs.add(r("folder", "Shared").sub(s(x, "path")).switch(p, on, Some(act_v(H::Hs, vec!["share-set".into(), id.clone(), "on".into(), if on { "off" } else { "on" }.into()]).ok(if on { "Not shared now (its files stay where they are)" } else { "Shared again" }).msg())).el(p));
        let write = b(x, "write");
        let sub = if whole == "system" {
            "The system drive is only ever read only: writing system files over the network can stop the PC from starting"
        } else if whole == "home" {
            if confirming == "home-write" { "Your devices could change or delete anything in your Home (hidden files stay blocked). Pick Read & write again to confirm" } else { "Read & write for your whole Home asks a second time" }
        } else {
            ""
        };
        let rw_msg = if whole == "home" {
            if confirming == "home-write" {
                act_v(H::Hs, vec!["share-set".into(), id.clone(), "write".into(), "on".into(), "home-write".into()]).set("confirming", Value::Null).msg()
            } else {
                lv("confirming", json!("home-write"))
            }
        } else {
            act_v(H::Hs, vec!["share-set".into(), id.clone(), "write".into(), "on".into()]).msg()
        };
        rs.add(choice(p, "", "Access", sub, vec![
            ("Read only".into(), !write, Some(act_v(H::Hs, vec!["share-set".into(), id.clone(), "write".into(), "off".into()]).msg())),
            ("Read & write".into(), write, if whole == "system" { None } else { Some(rw_msg) }),
        ]));
        let reach = if ts_only { "tailscale".to_string() } else if s(x, "reach").is_empty() { "home".into() } else { s(x, "reach") };
        let id2 = id.clone();
        rs.add(choice(p, "", "Reachable", if ts_only { "“Only reachable through Tailscale” is on for everything (Tailscale page)" } else { "Tailscale only: your home network can't open it; your devices on Tailscale can, at home or away" }, opts(&[("home", "Home network", ts_only), ("tailscale", "Tailscale only", false)], &reach, move |v| act_v(H::Hs, vec!["share-set".into(), id2.clone(), "reach".into(), v.into()]).msg())));
        let rec = nf(x, "recycle");
        rs.add(r("trash", "Recycle bin (30 days)").sub(if write { "Files deleted from another device go to the share's recycle bin first, kept 30 days" } else { "For shares your devices can change (read & write)" }).switch(p, rec, Some(act_v(H::Hs, vec!["share-set".into(), id.clone(), "recycle".into(), if rec { "off" } else { "on" }.into()]).msg())).el(p));
        let people = keys(x, "people");
        let who_sub = format!("You ({user}){}. Add people and pick what they may open on the People page", people.iter().map(|nm| format!(", {nm}{}", if s(x, &format!("people.{nm}")) == "read" || !write { " (read only)" } else { "" })).collect::<String>());
        rs.add(r("user", "Who").value(if people.is_empty() { "Only you".to_string() } else { format!("{} people", people.len() + 1) }).sub(who_sub).chev().on(Msg::Go("hs-people")).el(p));
        let snap = b(x, "snapshots");
        let can = b(x, "snapOk") || snap;
        rs.add(r("restart", "Snapshots").sub(if b(x, "snapOk") { "Undo for this folder: hourly, daily and weekly copies (Drives & undo)".to_string() } else { s(x, "snapWhy") }).switch(p, snap, can.then(|| act_v(H::Hs, vec!["share-set".into(), id.clone(), "snapshots".into(), if snap { "off" } else { "on" }.into()]).msg())).el(p));
        if st.ls("removing").is_empty() {
            rs.add(r("close", "Stop sharing it").on(lv("removing", json!(id.clone()))).el(p));
        } else if st.ls("removing") == id {
            rs.add(r("close", format!("Stop sharing “{}”", s(x, "name"))).danger(true).sub("Your devices can't open it anymore. Its files stay where they are: nothing is deleted.").on(act_v(H::Hs, vec!["share-remove".into(), id.clone()]).ok("Not shared anymore (its files are still there)").set("sel", Value::Null).set("removing", Value::Null).msg()).el(p));
        }
    }

    rs.add(h(p, "Sharing password"));
    let set_on = if pw_set { format!("Set on {}. ", date(ms(n(cfg, "smbPasswordSet")))) } else { "Not set yet. ".into() };
    rs.add(r("lock", if pw_set { "Change your sharing password" } else { "Set your sharing password" }).sub(format!("{set_on}Your devices ask for it with your user name ({user}). Hearth never keeps it.")).on(toggle(st, "pwOpen")).el(p));
    if st.lb("pwOpen") {
        rs.add(tfield(p, st, "pw", "At least 8 characters, then Enter", true));
    }

    rs.add(h(p, "How to open them"));
    let host = if s(stv, "host").is_empty() { "this-pc".to_string() } else { s(stv, "host") };
    let first = shares_.first();
    let how = |k: &str, dflt: String| first.map(|x| s(x, &format!("how.{k}"))).filter(|x| !x.is_empty()).unwrap_or(dflt);
    rs.add(r("display", "Windows").sub(format!("File Explorer › This PC › Map network drive, then {}", how("windows", format!("\\\\{host}\\Name")))).el(p));
    rs.add(r("pad", "Phone or TV").sub(format!("Any Files app with network storage (SMB): {}", how("phone", format!("smb://{host}/Name")))).el(p));
    rs.add(r("desktops", "Hearth devices").sub(format!("Hearth Files › Network, or type {}", how("mac", format!("smb://{host}/Name")))).el(p));
    let away = first.map(|x| s(x, "how.away")).unwrap_or_default();
    rs.add(r("globe", "Away from home").sub(if away.is_empty() { "Sign in to Tailscale (Tailscale page) and the same names work from anywhere".to_string() } else { format!("The same through Tailscale: {away}") }).el(p));
    rs.add(words(p, "Sign in with your user name and your sharing password. Hidden files (names starting with a dot) are never shown."));
    if at(stv, "applied.ok") == &Value::Bool(false) {
        rs.add(words_c(format!("Saved, but sharing couldn't be changed to match: {}", s(stv, "applied.error")), p.danger));
    }
    rs.add(message(p, st));
    rs.el()
}

// ------------------------------------------------------------------ Files & links
fn fsize(b_: f64) -> String {
    size1(b_)
}

pub fn files<'a>(p: &Pal, st: &'a St) -> E<'a, Msg> {
    let fv = st.get("files");
    let scan = at(fv, "scan");
    let found = st.get("found");
    let link_path = st.ls("linkPath");
    let mut rs = Rows::new();
    rs.add(words(p, "Find anything in your shares, free up space, and share files safely: links work on Tailscale or your home network only."));
    if b(fv, "error") {
        rs.add(words_c(s(fv, "error"), p.danger));
    }
    rs.add(tfield(p, st, "search", "Search every share, then Enter", false));
    for x in arr(found, "results").iter().take(30) {
        let path = s(x, "path");
        let dir = b(x, "dir");
        rs.add(r(if dir { "folder" } else { "file" }, s(x, "name")).sub(format!("{} · {}{}", s(x, "share"), path, if dir { String::new() } else { format!(" · {}", fsize(n(x, "size"))) })).value(if link_path == path { "Picked" } else { "Share a link" }).on(pick(st, "linkPath", &path)).el(p));
    }
    if found.is_object() && !b(found, "error") {
        let k = arr(found, "results").len();
        rs.add(words(p, if k > 0 { format!("{k} found{}. Select one to share a link to it", if b(found, "partial") { " (stopped looking after a few seconds: type more of the name)" } else { "" }) } else { "Nothing found".into() }));
    }

    if !link_path.is_empty() {
        rs.add(h(p, format!("Share a link to {}", link_path.rsplit('/').next().unwrap_or(""))));
        let days = if st.l.contains_key("linkDays") { st.ls("linkDays") } else { "7".into() };
        rs.add(choice(p, "", "Works for", "", opts(&[("1", "1 day", false), ("7", "7 days", false), ("30", "30 days", false), ("0", "Until I stop it", false)], &days, |v| lv("linkDays", json!(v.parse::<i64>().unwrap_or(7))))));
        rs.add(tfield(p, st, "linkfor", if st.ls("linkFor").is_empty() { "Who it's for (only a note for you), then Enter".to_string() } else { format!("For {} (Enter changes it)", st.ls("linkFor")) }, false));
        let pw = st.lb("linkPw");
        rs.add(r("lock", "With a password").sub("They type it when they open the link").switch(p, pw, Some(toggle(st, "linkPw"))).el(p));
        if pw {
            rs.add(tfield(p, st, "linkpw", "The link's password (at least 6 characters), then Enter to make the link", true));
        } else {
            let urls = arr(fv, "urls");
            let sub = if !urls.is_empty() { format!("Reachable at {} (Tailscale{})", vs(&urls[0]), if s(fv, "web.reach") == "home" { " and your home network" } else { " only" }) } else { "Turn on Tailscale first (or pick Home network in Services › Files on the web)".into() };
            rs.add(r("globe", "Make the link").sub(sub).on(make_link(st, "").msg()).el(p));
        }
    }

    rs.add(h(p, "Shared links"));
    for l in arr(fv, "links") {
        let urls = arr(l, "urls");
        let state = if b(l, "expired") { "Expired".to_string() } else if b(l, "used") { "Used up".into() } else if n(l, "expires") > 0.0 { format!("Until {}", hearth_ui::time::day(ms(n(l, "expires")) as i64)) } else { "Until you stop it".into() };
        let sub = format!("{state} · {} · {}{} downloads{}", if b(l, "pw") { "password" } else { "no password" }, s(l, "count"), if n(l, "max") > 0.0 { format!(" of {}", s(l, "max")) } else { String::new() }, if urls.is_empty() { String::new() } else { format!(" · {}", vs(&urls[0])) });
        rs.add(r(if b(l, "expired") || b(l, "used") { "clock" } else { "globe" }, format!("{}{}", s(l, "label"), if s(l, "for").is_empty() { String::new() } else { format!(" · for {}", s(l, "for")) })).sub(sub).value(if urls.is_empty() { "" } else { "Copy" }).on_maybe(urls.first().map(|u| Msg::Copy(vs(u)))).el(p));
    }
    for l in arr(fv, "links") {
        rs.add(r("close", format!("Stop {}", s(l, "label"))).sub("The link stops working. The file stays").on(act_v(H::Hs, vec!["link-remove".into(), s(l, "id")]).ok("Stopped").msg()).el(p));
    }
    if fv.is_object() && arr(fv, "links").is_empty() {
        rs.add(r("globe", "No links yet").sub("Search for a file or folder above and select it to share a link").el(p));
    }

    rs.add(h(p, "Drop-off folder"));
    if fv.is_object() && has(fv, "dropoff") {
        let on = b(fv, "dropoff.on");
        rs.add(r("folder", "Drop-off folder").sub(if on { format!("Anyone with its link can add files to {}. They can't see or delete anything", s(fv, "dropoff.path")) } else { "Friends add files with a link, but can't see or delete anything".into() }).switch(p, on, Some(act(H::Hs, &["dropoff", if on { "off" } else { "on" }]).msg())).el(p));
        let urls = arr(fv, "dropoff.urls");
        if on && !urls.is_empty() {
            rs.add(r("globe", "Drop-off link").sub(vs(&urls[0])).value("Copy").on(Msg::Copy(vs(&urls[0]))).el(p));
        }
    }

    rs.add(h(p, "Smart folders"));
    let scanning = b(fv, "scanning");
    let at_ = n(scan, "at");
    rs.add(r("search", if scanning { "Looking through your shares…" } else if at_ > 0.0 { "Look again" } else { "Look through your shares" })
        .sub(format!("{}Finds recent screenshots, big files, duplicates and files not changed in a year, at low priority", if at_ > 0.0 { format!("Last looked {}. ", when(at_)) } else { String::new() }))
        .on_maybe((!scanning).then(|| act(H::Hs, &["files-scan"]).msg()))
        .el(p));
    let open_smart = st.ls("openSmart");
    if at_ > 0.0 {
        for (id, title) in [("screenshots", "Recent screenshots"), ("big", "Big files"), ("duplicates", "Duplicates"), ("old", "Not changed in a year")] {
            let d = at(scan, id);
            let cnt = n(d, "count");
            rs.add(r("folder", title).sub(format!("{} · {}{}", plural(cnt, "file", "files"), fsize(n(d, "bytes")), if id == "duplicates" { " you could free" } else { "" })).value(if open_smart == id { "Close" } else if cnt > 0.0 { "Review" } else { "" }).on(pick(st, "openSmart", id)).el(p));
        }
    }
    if !open_smart.is_empty() {
        let d = at(scan, &open_smart);
        let items: Vec<&Value> = arr(d, "items").iter().take(40).collect();
        for i in &items {
            let path = s(i, "path");
            rs.add(r("file", path.rsplit('/').next().unwrap_or("").to_string()).sub(format!("{} · {} · {}", s(i, "share"), fsize(n(i, "size")), date(n(i, "mtime")))).value("Show").on(Msg::Show(path)).el(p));
        }
        if n(d, "count") > 0.0 {
            let list: Vec<String> = items.iter().map(|i| s(i, "path")).collect();
            let a = act(H::Hs, &["files-recycle"]).input(json!(list).to_string()).ok("{moved} moved to the recycle bin. Put back undoes it").set("openSmart", Value::Null).then(act(H::Hs, &["files-scan"]));
            rs.add(r("trash", if open_smart == "duplicates" { "Move the extra copies to the recycle bin" } else { "Move these to the recycle bin" }).sub("Each goes to its share's recycle bin for 30 days (never deleted now). Put back undoes it").on(a.msg()).el(p));
        }
    }
    let recycled = arr(fv, "recycled");
    if !recycled.is_empty() {
        rs.add(r("restart", "Put back").sub(format!("{} files moved to the recycle bin last time go back where they were", recycled.len())).on(act(H::Hs, &["files-putback"]).ok("{putBack} put back").msg()).el(p));
    }
    if n(scan, "recent.count") > 0.0 {
        rs.add(h(p, "Changed this week"));
        for i in arr(scan, "recent.items").iter().take(12) {
            let path = s(i, "path");
            rs.add(r("file", path.rsplit('/').next().unwrap_or("").to_string()).sub(format!("{} · {} · {}", s(i, "share"), when(n(i, "mtime")), fsize(n(i, "size")))).on(Msg::Show(path)).el(p));
        }
    }
    if has(scan, "sizes") {
        rs.add(h(p, "Space per share"));
        for x in arr(fv, "shares") {
            rs.add(r("folder", s(x, "name")).sub(format!("{} · {}", fsize(n(scan, &format!("sizes.{}", s(x, "id")))), s(x, "path"))).el(p));
        }
    }
    rs.add(message(p, st));
    rs.el()
}

// ------------------------------------------------------------------ People
fn access_of(share: &Value, name: &str) -> String {
    let a = s(share, &format!("people.{name}"));
    if a.is_empty() { "none".into() } else { a }
}

fn summary(stv: &Value, person: &Value) -> String {
    let name = s(person, "name");
    let mut parts: Vec<String> = arr(stv, "shares").iter().filter(|x| access_of(x, &name) != "none").map(|x| format!("{}{}", s(x, "name"), if access_of(x, &name) == "read" { " (read only)" } else { "" })).collect();
    if b(person, "folder") {
        parts.push(format!("own folder{}", if n(person, "quotaGb") > 0.0 { format!(" ({} GB)", s(person, "quotaGb")) } else { String::new() }));
    }
    if parts.is_empty() { "No access yet: pick their shares below".into() } else { parts.join(", ") }
}

pub fn people<'a>(p: &Pal, st: &'a St) -> E<'a, Msg> {
    let stv = st.get("status");
    let people_ = arr(stv, "people");
    let sel = st.ls("sel");
    let who = people_.iter().find(|x| s(x, "name") == sel && !sel.is_empty());
    let mut rs = Rows::new();
    rs.add(words(p, "Family and friends who can open your shares, each with their own name and password and only the shares you pick. They can't sign in to this PC."));
    if has(stv, "tools") && !b(stv, "tools.samba") {
        rs.add(words_c("Samba (sharing) isn't on this PC yet: people come with the Hearth OS update that brings Home server.", p.flame));
    }
    if b(stv, "error") {
        rs.add(words_c(s(stv, "error"), p.danger));
    }
    rs.add(h(p, "People"));
    if stv.is_object() && !b(stv, "error") {
        rs.add(r("user", format!("You ({})", s(stv, "user"))).value("Owner").sub("Everything, with your sharing password (Shared folders)").el(p));
    }
    for x in people_ {
        let name = s(x, "name");
        let sub = format!("{}{}", summary(stv, x), if at(x, "account") == &Value::Bool(false) { " · not set up on this PC: set their password again" } else { "" });
        rs.add(r("user", name.clone()).sub(sub).value(if sel == name { "Open below" } else { "Edit" }).on(Msg::Locals(vec![("sel".into(), if sel == name { Value::Null } else { json!(name) }), ("removing".into(), Value::Null), ("pwOpen".into(), Value::Null)])).el(p));
    }
    let adding = st.lb("adding");
    rs.add(r("plus", "Add a person").sub("A name (small letters, like sam or mia-k) and their own password").on(Msg::Locals(vec![("adding".into(), if adding { Value::Null } else { json!(true) }), ("newName".into(), Value::Null)])).el(p));
    if adding && st.ls("newName").is_empty() {
        rs.add(tfield(p, st, "newname", "Their name: 2 to 24 small letters, digits, - or _, then Enter", false));
    }
    if adding && !st.ls("newName").is_empty() {
        rs.add(tfield(p, st, "newpw", format!("A password for {} (at least 8 characters), then Enter", st.ls("newName")), true));
    }
    if let Some(w) = who {
        let name = s(w, "name");
        rs.add(h(p, name.clone()));
        for x in arr(stv, "shares") {
            let write = b(x, "write");
            let id = s(x, "id");
            let cur = access_of(x, &name);
            let nm = name.clone();
            rs.add(choice(p, "", s(x, "name"), if write { s(x, "path") } else { format!("{} · read only for everyone (make it read & write in Shared folders to let them change it)", s(x, "path")) }, opts(&[("none", "No access", false), ("read", "Read only", false), ("write", "Read & write", !write)], &cur, move |v| act_v(H::Hs, vec!["person-access".into(), nm.clone(), id.clone(), v.into()]).msg())));
        }
        if arr(stv, "shares").is_empty() {
            rs.add(words(p, "Share a folder first (Shared folders), then pick it here."));
        }
        let folder = b(w, "folder");
        let sub = if folder { format!("{} · only {name} and you can open it (as \\\\{}\\{name})", s(w, "folder"), s(stv, "host")) } else { format!("Home › People › {name}, shared with only them and you. Turning it off keeps the folder and its files") };
        rs.add(r("folder", "Their own folder").sub(sub).switch(p, folder, Some(act_v(H::Hs, vec!["person-folder".into(), name.clone(), if folder { "off" } else { "on" }.into()]).ok(if folder { "Not shared anymore (the folder and its files stay)" } else { "Their folder is ready" }).msg())).el(p));
        let q = if n(w, "quotaGb") > 0.0 { s(w, "quotaGb") } else { "0".into() };
        let nm = name.clone();
        rs.add(choice(p, "", "Size limit for their folder", "On a btrfs drive (your Home on a Hearth install is). Other drives: no limit is possible", opts(&[("0", "None", false), ("50", "50 GB", false), ("200", "200 GB", false), ("500", "500 GB", false), ("1000", "1 TB", false)], &q, move |v| {
            act_v(H::Hs, vec!["person-quota".into(), nm.clone(), v.into()]).ok(if v == "0" { "{note|No limit}".to_string() } else { format!("{{note|Limit set: {v} GB}}") }).msg()
        })));
        let missing = at(w, "account") == &Value::Bool(false);
        rs.add(r("lock", if missing { "Set their password (to finish setting them up)" } else { "Change their password" }).sub(if n(w, "pwSet") > 0.0 { format!("Set on {}. Hearth never keeps it", date(ms(n(w, "pwSet")))) } else { "Hearth never keeps it".into() }).on(toggle(st, "pwOpen")).el(p));
        if st.lb("pwOpen") {
            rs.add(tfield(p, st, "whopw", "Their new password (at least 8 characters), then Enter", true));
        }
        if st.ls("removing") != name {
            rs.add(r("close", format!("Remove {name}")).on(lv("removing", json!(name.clone()))).el(p));
        } else {
            rs.add(r("close", format!("Remove {name} now")).danger(true).sub("They can't open anything anymore. Their folder and everything they saved stay where they are.").on(act_v(H::Hs, vec!["person-remove".into(), name.clone()]).ok(format!("{name} can't open your shares anymore (their files are still there)")).set("sel", Value::Null).set("removing", Value::Null).msg()).el(p));
        }
    }
    rs.add(h(p, "How they sign in"));
    rs.add(r("display", "With their own name and password").sub("The same way as you (Shared folders › How to open them), with their name instead of yours. Hidden files are never shown, and nobody gets in without a password.").el(p));
    if at(stv, "applied.ok") == &Value::Bool(false) {
        rs.add(words_c(format!("Saved, but sharing couldn't be changed to match: {}", s(stv, "applied.error")), p.danger));
    }
    rs.add(message(p, st));
    rs.el()
}

// ------------------------------------------------------------------ Family & home
pub fn family<'a>(p: &Pal, st: &'a St) -> E<'a, Msg> {
    let f = st.get("family");
    let cam_shown = b(f, "seen.camera.shown");
    let mut rs = Rows::new();
    rs.add(words(p, "Folders for the whole family, who can open what, and cameras. It all stays on this PC and your own devices: no outside cloud."));
    if b(f, "error") {
        rs.add(words_c(s(f, "error"), p.danger));
    }
    rs.add(h(p, "Shared family folder"));
    if f.is_object() && !b(f, "family") && !b(f, "error") {
        rs.add(r("folder", "Make a Family folder").sub("Home › Family, read & write for you and every person you added (People): holiday photos, school things, shared notes and lists").on(act(H::Hs, &["folder-share", "family"]).ok("Family folder shared ✓ Everyone in People can open it").msg()).el(p));
    }
    if b(f, "family") {
        let fam = at(f, "family");
        let ppl = keys(fam, "people");
        rs.add(r("folder", "Family folder").sub(format!("{} · {} · {}", s(fam, "path"), if b(fam, "write") { "read & write" } else { "read only" }, if ppl.is_empty() { "only you so far".to_string() } else { format!("you, {}", ppl.join(", ")) })).chev().on(Msg::Go("hs-shares")).el(p));
    }
    let k = arr(f, "people").len() as f64;
    rs.add(r("user", "People").sub("Family and friends with their own password and only the folders you pick").value(if k > 0.0 { plural(k, "person", "people") } else { String::new() }).chev().on(Msg::Go("hs-people")).el(p));
    rs.add(r("calendar", "Calendar, notes and lists").sub("Keep them as files in the Family folder (any notes app on a phone can open a shared folder). A shared calendar server isn't part of Hearth yet").el(p));

    rs.add(h(p, "Security cameras"));
    let looking = st.busy > 0 && st.msg == "Looking for cameras…";
    let last = n(f, "camerasAt");
    // "N cameras found" / "No cameras answered" below, from the list read again after the scan.
    let scan = act(H::Hs, &["cameras-scan"]).secs(60).wait("Looking for cameras…").set("scanned", json!(true));
    rs.add(r("cam", if looking { "Looking for cameras…" } else { "Look for cameras" })
        .sub(format!("Asks your home network for cameras (ONVIF). Finding one shows the camera items; it turns nothing on{}", if last > 0.0 { format!(" · last looked {}", when(ms(last))) } else { String::new() }))
        .on_maybe((!looking).then(|| scan.msg()))
        .el(p));
    if st.lb("scanned") && !looking {
        let k = arr(f, "cameras").len() as f64;
        rs.add(words(p, if k > 0.0 { plural(k, "camera found", "cameras found") } else { "No cameras answered".into() }));
    }
    if cam_shown {
        for c in arr(f, "cameras") {
            rs.add(r("cam", s(c, "name")).sub(format!("{} · opens the camera's own page", s(c, "ip"))).on(Msg::Open(s(c, "url"))).el(p));
        }
        rs.add(r("info", "Recording to this PC").sub("Hearth doesn't record cameras yet. Your camera's own app or page can save clips to a shared folder (most cameras can save to an SMB share: use a person's name and password from People)").el(p));
    } else if f.is_object() {
        rs.add(words(p, "Camera items show once a camera is found (or with Show hidden items on the Overview page)."));
    }
    rs.add(words(p, "Not here yet: parental controls (play time) and ad blocking for the whole home."));
    rs.add(message(p, st));
    rs.el()
}
