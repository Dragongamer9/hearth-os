//! Hearth Link (SetLink.qml) and Your devices (SetDevices.qml), with Mantel's helpers: hearth-link-settings (the
//! Hearth Link service's socket), hearth-device-status, hearth-files (Show other devices in Files) and
//! hearth-home-server quick. The pieces and the running are shared with Home server (cats/hs/kit.rs).
//!
//! Pairing has three kinds of device: My device (paired with all your devices, with your settings and library), A
//! family member's device (status and Remote desktop, no settings, library or saves) and A friend's device (paired
//! with this one only, for copying games). The words sent to hearth-link-settings for each are hearth-link's own
//! (feature/link-family, hearth-link-settings and SetLink.qml): pairing's last word "family" or "friend"
//! (`pair_args`), make-family / make-friend / make-mine ID (`kind_args`), and a family device's switches
//! `share ID status|library|saves|storage|backup on|off` (`share_args`).

use crate::cats::hs::kit::{self, *};
use crate::shared::Shared;
use crate::widgets::sz;
use crate::App;
use hearth_ui::ui::{self, t, ts};
use hearth_ui::Pal;
use iced::widget::{column, container, row, Column, Row};
use iced::{Element, Length, Padding, Subscription, Task};
use serde_json::{json, Value};

pub const PAGES: &[&str] = &["link", "devices"];

pub type State = St;
pub type Msg = kit::Msg;

/// The kinds of paired device, their words, and what each shares.
pub const KINDS: [(&str, &str, &str); 3] = [
    ("mine", "My device", "Yours: paired with all your devices at once, with your settings, library, saves and status"),
    ("family", "A family member's device", "Pick this on your own device. You see it in Your devices and can help on it (wake it, remote desktop). None of your settings, saves or library go onto it"),
    ("friend", "A friend's device", "Paired with this one only, just for copying games. Nothing else is shared"),
];

/// A pairing's words for the kind picked (hearth-link-settings pair-open, pair-open-internet, join, join-code).
pub fn pair_args(base: Vec<String>, kind: &str) -> Vec<String> {
    let mut v = base;
    match kind {
        "friend" => v.push("friend".into()),
        "family" => v.push("family".into()),
        _ => {}
    }
    v
}

/// Making a paired device another kind (hearth-link-settings make-friend / make-family / make-mine; My device
/// again only by pairing again on both, which hearth-link says).
pub fn kind_args(id: &str, kind: &str) -> Vec<String> {
    let cmd = match kind {
        "friend" => "make-friend",
        "family" => "make-family",
        _ => "make-mine",
    };
    vec![cmd.into(), id.into()]
}

/// A family member's device's switches on this device (hearth-link-settings share ID KEY on|off).
pub fn share_args(id: &str, key: &str, on: bool) -> Vec<String> {
    vec!["share".into(), id.into(), key.into(), if on { "on" } else { "off" }.into()]
}

/// A peer's kind: hearth-link's `relation` (mine / family), a friend from `friend`.
pub fn kind_of(peer: &Value) -> String {
    if s(peer, "relation") == "family" {
        return "family".into();
    }
    if b(peer, "friend") { "friend".into() } else { "mine".into() }
}

/// What a family member chose to share with you (SetLink.qml theirLine).
fn their_line(pe: &Value) -> String {
    let t = at(pe, "theirShares");
    let on: Vec<&str> = [("status", "its status"), ("library", "its game library"), ("saves", "its saves"), ("storage", "your Hearth Storage"), ("backup", "backing the PC up")]
        .iter().filter(|(k, _)| b(t, k)).map(|(_, w)| *w).collect();
    if on.is_empty() {
        "They don't share anything else with you yet. Only they can turn that on, on their device".into()
    } else {
        format!("They share: {}. Only they can change this, on their device", on.join(", "))
    }
}
fn kind_word(k: &str) -> &'static str {
    KINDS.iter().find(|x| x.0 == k).map(|x| x.1).unwrap_or("My device")
}

fn reads(page: &str, st: &St) -> Vec<Rd> {
    match page {
        "link" => {
            let mut v = vec![rd("st", H::Link, &["status"])];
            if has(st.get("st"), "storage.config") {
                v.push(rd("devs", H::Link, &["devices"]));
            }
            v
        }
        _ => {
            let mut v = vec![rd("devices", H::DevStatus, &["devices"])];
            if !st.d.contains_key("prefs") {
                v.push(rd("prefs", H::Files, &["prefs"]));
                v.push(rd("hsq", H::Hs, &["quick"]));
            }
            v
        }
    }
}

fn follow(page: &str, slot: &str, st: &St) -> Vec<Rd> {
    if page == "link" && slot == "st" && has(st.get("st"), "storage.config") && !st.d.contains_key("devs") {
        return vec![rd("devs", H::Link, &["devices"])];
    }
    Vec::new()
}

fn live(page: &str, st: &St) -> bool {
    if page != "link" {
        return false;
    }
    let v = st.get("st");
    b(v, "pairing") || b(v, "joining") || arr(v, "transfers").iter().any(|x| ["copying", "waiting", "preparing", "comparing", "finishing"].contains(&s(x, "state").as_str()))
}

// Short codes (hosts/hearth-link shortcode.rs): K7M-4QX-92HC, no 0/O/1/I/L, a check character.
const ABC: &str = "23456789ABCDEFGHJKMNPQRSTUVWXYZ";
fn squash(t_: &str) -> String {
    t_.chars().filter(|c| !" -._·".contains(*c)).collect::<String>().to_uppercase()
}
fn short_fmt(t_: &str) -> String {
    let c: Vec<char> = squash(t_).chars().take(10).collect();
    let parts: Vec<String> = [&c[..c.len().min(3)], &c[c.len().min(3)..c.len().min(6)], &c[c.len().min(6)..]].iter().map(|x| x.iter().collect::<String>()).filter(|x| !x.is_empty()).collect();
    parts.join("-")
}
pub fn short_check(t_: &str) -> String {
    let c = squash(t_);
    if c.chars().any(|x| "01OIL".contains(x)) {
        return "That code doesn't look right: codes never have 0, O, 1, I or L in them".into();
    }
    if let Some(bad) = c.chars().find(|x| !ABC.contains(*x)) {
        return format!("That code doesn't look right: \"{bad}\" isn't in codes");
    }
    if c.chars().count() != 10 {
        return format!("That code doesn't look right: it's 10 characters, and this is {}", c.chars().count());
    }
    let sum: usize = c.chars().enumerate().map(|(i, ch)| (i + 1) * ABC.find(ch).unwrap_or(0)).sum();
    if sum % 31 != 0 { "That code doesn't look right: one of the characters is off. Check it on the other device".into() } else { String::new() }
}

fn pair_kind(st: &St) -> String {
    let k = st.ls("pairKind");
    if k.is_empty() { "mine".into() } else { k }
}

fn join(st: &St, base: Vec<String>, input: String) -> Act {
    let kind = pair_kind(st);
    act_v(H::Link, pair_args(base, &kind))
        .input(input)
        .secs(120)
        .wait("Pairing…")
        .ok(format!("Paired ✓ with {{name|{}}}{}", if st.ls("joiningName").is_empty() { "the other device".to_string() } else { st.ls("joiningName") }, match kind.as_str() {
            "friend" => ": a friend's device, for copying games only",
            "family" => ": a family member's device",
            _ => ": it's paired with your other devices too",
        }))
        .set("joining", Value::Null)
}

fn submit(st: &mut St, key: &str, text: String) -> Result<Vec<Act>, String> {
    match key {
        "rename" => {
            let t_ = text.trim().to_string();
            if t_.is_empty() {
                return Ok(Vec::new());
            }
            Ok(vec![act_v(H::Link, vec!["set".into(), "name".into(), t_]).ok("Renamed")])
        }
        "code" => {
            let c: String = text.chars().filter(|x| x.is_ascii_digit()).collect();
            if c.len() != 6 {
                return Err("The code is 6 digits".into());
            }
            let j = st.ls("joining");
            // A wrong code keeps the field open (with the tries left); anything else closes it.
            let mut a = join(st, vec!["join".into(), j], c);
            a.fail_set = Vec::new();
            Ok(vec![a])
        }
        "short" => {
            let bad = short_check(&text);
            if !bad.is_empty() {
                return Err(bad);
            }
            st.setl("joiningName", json!("the other device"));
            Ok(vec![join(st, vec!["join-code".into()], short_fmt(&text)).fail_set("joining", Value::Null)])
        }
        "far" => {
            let t_ = text.trim().to_string();
            if !t_.is_empty() {
                st.setl("joining", json!(t_));
                st.setl("joiningName", json!("the other device"));
                st.t.insert(key.into(), String::new());
            }
            Ok(Vec::new())
        }
        "pin" => {
            // Moonlight's PIN for the open device's remote desktop, on stdin (hearth-link-settings remote-pin ID).
            let c: String = text.chars().filter(|x| x.is_ascii_digit()).collect();
            if c.len() != 4 {
                return Err("Moonlight's PIN is 4 numbers".into());
            }
            st.t.insert(key.into(), String::new());
            Ok(vec![act_v(H::Link, vec!["remote-pin".into(), st.ls("peer")]).input(c).ok("Sent: Moonlight is paired with it")])
        }
        "share" => {
            let t_ = text.trim().to_string();
            if t_.is_empty() {
                return Ok(Vec::new());
            }
            Ok(vec![act_v(H::Link, vec!["share-add".into(), t_]).ok("Shared").set("shareOpen", Value::Null)])
        }
        _ => Ok(Vec::new()),
    }
}

pub const HOOKS: Hooks = Hooks { reads, follow, submit, live };

pub fn open(sh: &mut Shared, st: &mut State, page: &'static str) -> Task<Msg> {
    let part = std::mem::take(&mut sh.part);
    kit::open(st, page, &part, &HOOKS)
}

pub fn update(sh: &mut Shared, st: &mut State, m: Msg) -> Task<Msg> {
    kit::update(sh, st, m, &HOOKS)
}

/// The Hearth Link tile on Home (settings-A1: "3 devices"): how many devices are paired, read by Home itself.
pub fn summary_of(v: &Value) -> String {
    if v.is_null() {
        return "…".into();
    }
    if b(v, "error") {
        return "Pair your devices".into();
    }
    let k = arr(v, "peers").len();
    match k {
        0 => "No devices paired yet".into(),
        1 => "1 device".into(),
        _ => format!("{k} devices"),
    }
}

/// Share this device's status (desktop.json shareStatus, on unless turned off): through hearth_settings_io.py.
fn share_status_row<'a>(a: &'a App) -> E<'a, Msg> {
    let p = a.sh.p;
    let on = if a.link.l.contains_key("shareStatus") { a.link.lb("shareStatus") } else { a.sh.d_bool("shareStatus", true) };
    let path = hearth_ui::theme::desktop_json().to_string_lossy().to_string();
    let cmd = act_v(H::Io, vec!["merge".into(), path, json!({ "shareStatus": !on }).to_string(), "--history".into()]).set("shareStatus", json!(!on)).no_reload();
    r("eye", "Let my other devices see this device's status").sub("Battery, use, temperatures, drives and what's playing, only for devices paired with this one (Hearth Link). Never the internet.").switch(&p, on, Some(cmd.msg())).el(&p)
}

fn mins_left(until_ms: f64) -> String {
    let m = ((until_ms / 1000.0 - hearth_ui::time::now() as f64) / 60.0).ceil().max(0.0);
    if m <= 1.0 { "less than a minute left".into() } else { format!("{m} min left") }
}
fn what(nd: &Value) -> String {
    let kind = match s(nd, "kind").as_str() {
        "handheld" => "Handheld",
        "laptop" => "Laptop",
        "desktop" => "Desktop",
        "tablet" => "Tablet",
        "tv" => "TV",
        _ => "",
    };
    join_words(&[kind.to_string(), s(nd, "edition")])
}
fn join_words(v: &[String]) -> String {
    kit::join(v, " · ")
}
fn bytes(x: f64) -> String {
    if x >= 1e9 { format!("{:.1} GB", x / 1e9) } else { format!("{} MB", (x / 1e6).round()) }
}
fn job_line(t_: &Value) -> String {
    let st_ = s(t_, "state");
    let mut out = match st_.as_str() {
        "waiting" => "Waiting for the other device".to_string(),
        "interrupted" => "Interrupted".into(),
        "preparing" => "Getting ready".into(),
        "comparing" => "Comparing with what's here".into(),
        "copying" => "Copying".into(),
        "finishing" => "Finishing".into(),
        "paused" => "Paused".into(),
        "done" => "Done".into(),
        "error" => "Didn't work".into(),
        "cancelled" => "Cancelled".into(),
        other => other.to_string(),
    };
    if st_ == "copying" && n(t_, "total") > 0.0 {
        out.push_str(&format!(" · {}% of {}{}", (100.0 * n(t_, "done") / n(t_, "total")).floor(), bytes(n(t_, "total")), if n(t_, "speed") > 0.0 { format!(" · {}/s", bytes(n(t_, "speed"))) } else { String::new() }));
    }
    if st_ == "done" && !s(t_, "note").is_empty() {
        out.push_str(&format!(" · {}", s(t_, "note")));
    }
    if st_ == "error" && !s(t_, "error").is_empty() {
        out.push_str(&format!(": {}", s(t_, "error")));
    }
    if b(t_, "relay") && st_ != "done" {
        out.push_str(" · through a relay (slower)");
    }
    out
}
fn event_line(ev: &Value) -> String {
    match s(ev, "state").as_str() {
        "checking" => format!("Found {}: checking the code…", s(ev, "name")),
        "wrong" => format!("{} typed a wrong code ({} {} left)", s(ev, "name"), s(ev, "triesLeft"), if n(ev, "triesLeft") == 1.0 { "try" } else { "tries" }),
        _ => "Waiting for the other device…".into(),
    }
}

fn link_page<'a>(a: &'a App) -> E<'a, Msg> {
    let p = a.sh.p;
    let st = &a.link;
    let v = st.get("st");
    let me = at(v, "me");
    let has_me = me.is_object();
    let pairing = at(v, "pairing");
    let mut rs = Rows::new();
    if b(v, "error") {
        rs.add(words(&p, s(v, "error")));
    }
    if has_me {
        rs.add(h(&p, "This device"));
        rs.add(r("display", s(me, "name")).sub(join_words(&[what(me), "Its name on your other Hearth devices".into()])).el(&p));
        rs.add(tfield(&p, st, "rename", "Rename this device", false));
        rs.add(r("mic", "Share this device's AI").sub("Its AI (chat, voices, pictures, speech to text) for your other devices and the Command Center, through Tailscale").chev().on(Msg::Go("hs-aidevices")).el(&p));
        // What the next device you pair is: one of three kinds (only for this screen, never saved).
        let pk = pair_kind(st);
        let sub = KINDS.iter().find(|x| x.0 == pk).map(|x| x.2).unwrap_or("");
        rs.add(choice(&p, "users", "The next device you pair is", sub, KINDS.iter().map(|(k, l, _)| (l.to_string(), *k == pk, Some(lv("pairKind", json!(k))))).collect()));
        if !pairing.is_object() {
            let ev = at(v, "pairEvent");
            let sub = match s(ev, "state").as_str() {
                "paired" => format!("Paired ✓ with {}. Select to pair another", s(ev, "name")),
                "locked" => "Too many wrong codes, so pairing stopped. Select for a new code".into(),
                _ => "Shows a code to type on your other Hearth device".into(),
            };
            rs.add(r("lock", "Pair a device").sub(sub).on(act_v(H::Link, pair_args(vec!["pair-open".into()], &pk)).msg()).el(&p));
        }
    }
    if pairing.is_object() {
        let code = s(pairing, "code");
        if !code.is_empty() {
            let (a1, a2) = code.split_at(code.len().min(3));
            rs.add(r("lock", format!("Code: {a1} {a2}")).sub(format!("{} On the other device: Settings › Hearth Link, pick {} and type this code. {}.", event_line(at(v, "pairEvent")), s(me, "name"), mins_left(n(pairing, "until")))).el(&p));
        }
        let net = s(pairing, "netCode");
        if !net.is_empty() {
            rs.add(r("globe", format!("Code for somewhere else: {net}")).sub(format!("On the other device: Settings › Hearth Link › Enter a code. Works once, {}.", mins_left(n(pairing, "netUntil")))).el(&p));
        } else if nf(v, "shortCodes") {
            rs.add(r("globe", "Not on the same network?").sub("Get a code to type on a device somewhere else (over the internet)").on(act_v(H::Link, pair_args(vec!["pair-open-internet".into()], &pair_kind(st))).msg()).el(&p));
        }
        rs.add(r("close", "Stop pairing").on(act(H::Link, &["pair-close"]).msg()).el(&p));
    }
    if has_me {
        rs.add(h(&p, "Pair with a device"));
        if st.busy > 0 && st.msg == "Pairing…" {
            let j = at(v, "joining");
            let line = match s(j, "state").as_str() {
                "looking" => "Looking up the code…".to_string(),
                "found" => format!("Found {}", s(j, "name")),
                "connecting" => format!("Found {}: connecting…", s(j, "name")),
                "checking" => format!("Checking the code with {}…", s(j, "name")),
                _ => "Pairing…".into(),
            };
            rs.add(r("update", line).el(&p));
        }
        let joining = st.ls("joining");
        let nearby = arr(v, "nearby");
        for nd in nearby {
            let id = s(nd, "id");
            let name = s(nd, "name");
            let ready = b(nd, "pairing");
            let w = what(nd);
            let sub = format!("{}{}", if w.is_empty() { String::new() } else { format!("{w} · ") }, if joining == id { "Type the 6-digit code it shows below, then Enter" } else if ready { "Ready to pair: select, then type the code it shows" } else { "To pair, select Pair a device on it first" });
            let m = if ready { Msg::Locals(vec![("joining".into(), json!(id)), ("joiningName".into(), json!(name.clone()))]) } else { Msg::Say(format!("On {name}, select Settings › Hearth Link › Pair a device first"), true) };
            let ic = if ["handheld", "tablet"].contains(&s(nd, "kind").as_str()) { "pad" } else { "display" };
            let mut row_ = r(ic, name).sub(sub).on(m);
            if ready {
                row_ = row_.chev();
            }
            rs.add(row_.el(&p));
        }
        if !nearby.iter().any(|x| b(x, "pairing")) {
            rs.add(words(&p, "Looking for Hearth devices on this network. On the other device, select Settings › Hearth Link › Pair a device."));
        }
        if !joining.is_empty() {
            rs.add(tfield(&p, st, "code", format!("The 6-digit code {} shows", if st.ls("joiningName").is_empty() { "it".into() } else { st.ls("joiningName") }), false));
        }
        if nf(v, "shortCodes") {
            rs.add(tfield(&p, st, "short", "Enter a code from a device somewhere else (like K7M-4QX-92HC), then Enter", false));
        }
        rs.add(r("info", "Other ways: this device's long link").sub(s(me, "ticket")).el(&p));
        rs.add(tfield(&p, st, "far", "Other ways: paste the other device's long link, then Enter", false));
    }
    let peers = arr(v, "peers");
    if has(v, "peers") {
        rs.add(h(&p, "Your devices"));
    }
    let open_peer = st.ls("peer");
    for pe in peers {
        let id = s(pe, "id");
        let name = s(pe, "name");
        let kind = kind_of(pe);
        let side = s(pe, "familySide");
        let state = if b(pe, "online") { "Online" } else if n(pe, "lastSeen") > 0.0 { "Not on right now" } else { "Not reached yet" };
        let whose = if kind == "family" && side == "helper" { "Family · helps on this device".to_string() } else { kind_word(&kind).to_string() };
        let sub = format!("{whose} · {state}{} · select for its choices", if s(pe, "introducedBy").is_empty() { String::new() } else { format!(" · paired through {}", s(pe, "introducedBy")) });
        let ic = match kind.as_str() { "family" => "users", "friend" => "user", _ => "display" };
        rs.add(r(ic, name.clone()).sub(sub).value(if b(pe, "online") { "Online" } else { "" }).chev().on(pick(st, "peer", &id)).el(&p));
        if open_peer == id {
            // Its choices (SetLink.qml): whose it is, a family device's switches, help on it, unpair.
            let fam = kind == "family";
            if !fam {
                rs.add(r("users", "Make it a family member's device").inset(true).sub("It stays on your devices and you can help on it, but it stops getting your settings, library, saves and status right away").on(act_v(H::Link, kind_args(&id, "family")).ok(format!("{name} is a family member's device now")).msg()).el(&p));
            }
            if kind != "friend" {
                rs.add(r("user", "Make it a friend's device").inset(true).sub("This device stops sharing anything with it but game copies, right away").on(act_v(H::Link, kind_args(&id, "friend")).ok(format!("{name} is a friend's device now")).msg()).el(&p));
            }
            if kind != "mine" {
                rs.add(r("display", "Make it one of my devices").inset(true).sub("Both devices have to agree: unpair it, then pair again and pick My device on both").on(act_v(H::Link, kind_args(&id, "mine")).ok("{note|Unpair it, then pair again and pick My device on both}").msg()).el(&p));
            }
            let shares = at(pe, "shares");
            let sw = |key: &str| -> (bool, Msg) {
                let on = b(shares, key);
                (on, act_v(H::Link, share_args(&id, key, !on)).ok(if on { "Off" } else { "On" }).msg())
            };
            if fam {
                // What this device lets that family device see (its status), on either side.
                let (on, m) = sw("status");
                rs.add(r("eye", "Let it see this device's status").inset(true).sub(format!("Battery, use, temperatures, drives and what's playing, for {name} only")).switch(&p, on, Some(m)).el(&p));
            }
            if fam && side != "member" {
                // On the family member's own device: what it shares with the person who helps on it (all off to
                // start with; only changed here).
                for (key, ic, title, line) in [
                    ("library", "store", "Share my game library", format!("Favorites, hidden games, collections and play time, both ways with {name}. Never settings")),
                    ("saves", "update", "Back up my saves", "Game saves kept on their Hearth Storage, and saves follow you there".to_string()),
                    ("storage", "store", "Use Hearth Storage (shared folders, the game archive, backups)", "Their shared folders, their game archive and their download caches, from this device".to_string()),
                    ("backup", "update", "Back up this PC", "This PC's saves and screenshots backed up to their Hearth Storage".to_string()),
                ] {
                    let (on, m) = sw(key);
                    rs.add(r(ic, title).inset(true).sub(line).switch(&p, on, Some(m)).el(&p));
                }
            }
            if fam && side != "helper" {
                rs.add(r("info", "What they share with you").inset(true).sub(their_line(pe)).el(&p));
            }
            if kind == "mine" || (fam && side != "helper") {
                // Its remote desktop: how to reach it in Moonlight (it says, over Hearth Link), then Moonlight's PIN.
                let rem = st.get("remote");
                let here = st.ls("remoteFor") == id && rem.is_object();
                let sub = if !here {
                    "Its screen here with Moonlight, through Tailscale. Select to see how to reach it".to_string()
                } else if b(rem, "on") {
                    format!("In Moonlight, add {}{}. Select to open Moonlight", s(rem, "host"), if b(rem, "tailscale") { "" } else { " (it isn't signed in to Tailscale right now)" })
                } else {
                    "Use this PC from anywhere is off on it: it has to be turned on there (Settings › Remote play)".to_string()
                };
                let m = if here && b(rem, "on") {
                    Msg::Detached(sv(&["sh", "-c", "command -v moonlight >/dev/null && exec moonlight || exec flatpak run com.moonlight_stream.Moonlight"]))
                } else {
                    act_v(H::Link, vec!["remote".into(), id.clone()]).store("remote").set("remoteFor", json!(id.clone())).no_reload().msg()
                };
                rs.add(r("play", "Remote desktop").inset(true).sub(sub).on(m).el(&p));
                if here && b(rem, "on") {
                    rs.add(tfield(&p, st, "pin", format!("Moonlight's 4-number PIN for {name}, then Enter"), false));
                }
            }
            if !b(pe, "online") && b(pe, "canWake") {
                rs.add(r("power", "Wake it").inset(true).sub("Wake-on-LAN (it has to be set to wake from the network)").on(act_v(H::Link, vec!["wake".into(), id.clone()]).ok(format!("Waking {name}…")).msg()).el(&p));
            }
            rs.add(r("trash", "Unpair it").inset(true).sub(if kind == "friend" { "Nothing is copied between them anymore, until you pair again" } else { "Nothing is shared with it anymore, until you pair again. Your other devices remove it too" }).on(act_v(H::Link, vec!["unpair".into(), id.clone()]).ok(format!("Unpaired {name}")).set("peer", Value::Null).msg()).el(&p));
        }
    }
    if !peers.is_empty() {
        rs.add(r("cpu", "Your devices").sub("Each device's battery, use, temperatures, drives and what's playing, with the last hour").chev().on(Msg::Go("devices")).el(&p));
    }
    let transfers = arr(v, "transfers");
    if !transfers.is_empty() {
        rs.add(h(&p, "Copies"));
    }
    for tr in transfers {
        let st_ = s(tr, "state");
        let pausable = ["copying", "waiting", "preparing", "comparing"].contains(&st_.as_str());
        let resumable = ["paused", "error", "interrupted"].contains(&st_.as_str());
        let id = s(tr, "id");
        let m = if pausable { Some(act_v(H::Link, vec!["pause".into(), id.clone()]).ok("Paused").msg()) } else if resumable { Some(act_v(H::Link, vec!["resume".into(), id.clone()]).ok("Resumed").msg()) } else { None };
        rs.add(r("store", format!("{}{}", s(tr, "title"), if s(tr, "from").is_empty() { String::new() } else { format!(" from {}", s(tr, "from")) })).sub(format!("{}{}", job_line(tr), if pausable { " · select to pause" } else if resumable { " · select to resume" } else { "" })).on_maybe(m).el(&p));
    }
    if !peers.is_empty() {
        rs.add(r("store", "Get a game from another Hearth").sub("Pick the game in your library's Get from another Hearth. Opens in Hearth Game Mode (Desktop Mode closes first).").on(Msg::Detached(sv(&["nimbus-mode", "hearth-page", "library"]))).el(&p));
    }
    for hint in arr(v, "signInHints") {
        rs.add(words(&p, s(hint, "text")));
    }
    let set_ = at(v, "settings");
    if set_.is_object() {
        rs.add(h(&p, "Settings"));
        let sync = b(set_, "sync");
        rs.add(r("update", "Keep Hearth data in sync").sub("Favorites, collections, play time, per-game settings, profile, appearance, controller layouts. Never sign-ins.").switch(&p, sync, Some(act(H::Link, &["set", "sync", if sync { "off" } else { "on" }]).msg())).el(&p));
        let aa = b(set_, "alwaysAccept");
        rs.add(r("store", "Accept games from my devices without asking").switch(&p, aa, Some(act(H::Link, &["set", "alwaysAccept", if aa { "off" } else { "on" }]).msg())).el(&p));
    }
    rs.add(share_status_row(a));
    if set_.is_object() {
        let cur = if s(set_, "relay").is_empty() { "default".into() } else { s(set_, "relay") };
        rs.add(choice(&p, "", "Relay", "Only when two devices can't connect directly over the internet; the data stays encrypted end to end", opts(&[("default", "n0's public relays", false), ("off", "Off (same network only)", false)], &cur, |x| act(H::Link, &["set", "relay", x]).ok("Hearth Link restarts with the new setting").msg())));
    }

    // Hearth Storage.
    let stc = at(v, "storage.config");
    if stc.is_object() {
        let set = |k: &str, on: bool| act(H::Link, &["storage-set", k, if on { "off" } else { "on" }]).msg();
        rs.add(h(&p, "Hearth Storage · This device"));
        let server = b(stc, "server");
        rs.add(r("store", "Use this device as Hearth Storage").sub("A home server for your paired Hearth devices: shared folders, games kept for them, their backups and saves. Only paired devices; nothing is opened to the internet.").switch(&p, server, Some(set("server", server))).el(&p));
        let devs = st.get("devs");
        let dl = arr(devs, "devices");
        if !dl.is_empty() {
            rs.add(h(&p, "Servers & devices"));
            if b(devs, "serverOffline") {
                rs.add(words(&p, format!("{} isn't answering (asleep or away), so only what this device knows is shown.", if s(devs, "serverName").is_empty() { "Your Hearth Storage".into() } else { s(devs, "serverName") })));
            }
            let removing = st.ls("removing");
            for d in dl {
                let id = s(d, "id");
                let bat = at(d, "battery");
                let line = join_words(&[
                    if b(d, "me") { "This device".to_string() } else if b(d, "online") { "Online".into() } else if n(d, "lastSeen") > 0.0 { format!("Last seen {}", when(ms(n(d, "lastSeen")))) } else { "Not seen yet".into() },
                    if has(bat, "percent") { format!("Battery {}%{}", s(bat, "percent"), if b(bat, "charging") { " (charging)" } else { "" }) } else { String::new() },
                    if has(d, "free") { format!("{} free", bytes(n(d, "free"))) } else { String::new() },
                    if n(d, "lastBackup") > 0.0 { format!("Backed up {}", when(ms(n(d, "lastBackup")))) } else { String::new() },
                    if s(d, "version").is_empty() { String::new() } else { format!("Hearth {}{}", s(d, "version"), if at(d, "upToDate") == &Value::Bool(false) { format!(" ({} is out)", s(d, "newest")) } else { String::new() }) },
                    if b(d, "boot.fellBack") { "Last update didn't start: went back to the previous version".into() } else if at(d, "boot.good") == &Value::Bool(false) { "Start not confirmed yet".into() } else { String::new() },
                ]);
                let me_ = b(d, "me");
                let mut row_ = r("display", format!("{}{}", s(d, "name"), if b(d, "server") { " · Hearth Storage" } else { "" })).sub(format!("{line}{}", if me_ { "" } else { " · select to remove it" })).value(if me_ || b(d, "online") { "OK" } else { "Off" });
                if !me_ {
                    row_ = row_.chev().on(pick(st, "removing", &id));
                }
                rs.add(row_.el(&p));
            }
            if !removing.is_empty() {
                rs.add(r("trash", "Remove it").sub("Unpaired from your devices. What Hearth Storage keeps of it stays. It can pair again any time.").on(act_v(H::Link, vec!["device-remove".into(), removing.clone()]).ok("Removed").set("removing", Value::Null).also(rd("devs", H::Link, &["devices"])).msg()).el(&p));
                rs.add(r("trash", "Remove it and delete its backups and archived games").sub("Also frees their space on Hearth Storage. This can't be undone.").on(act_v(H::Link, vec!["device-remove".into(), removing.clone(), "backups".into(), "archive".into()]).ok("Removed, and its backups and games deleted").set("removing", Value::Null).also(rd("devs", H::Link, &["devices"])).msg()).el(&p));
            }
        }
        if server {
            rs.add(h(&p, "Shared folders"));
            for x in arr(stc, "shares") {
                rs.add(r("store", s(x, "name")).sub(format!("{} · select to stop sharing it", s(x, "path"))).on(act_v(H::Link, vec!["share-remove".into(), s(x, "id")]).ok(format!("{} isn't shared anymore", s(x, "name"))).msg()).el(&p));
            }
            rs.add(r("store", "Share a folder").sub("Type its full path (in Home or on a drive). Hidden files are never shared.").on(toggle(st, "shareOpen")).el(&p));
            if st.lb("shareOpen") {
                rs.add(tfield(&p, st, "share", "Folder to share, like /home/you/Videos", false));
            }
        }
        if b(stc, "backupTo") || server {
            rs.add(h(&p, "Backups & saves"));
        }
        if b(stc, "backupTo") {
            rs.add(r("update", "Back up now").sub(format!("Saves and screenshots to your Hearth Storage (only what changed){}", if n(stc, "lastBackup") > 0.0 { format!(" · last {}", when(ms(n(stc, "lastBackup")))) } else { String::new() })).on(act(H::Link, &["backup-now"]).ok("Backing up").msg()).el(&p));
            let sf = nf(stc, "savesFollow");
            rs.add(r("update", "Saves follow you").sub("Games without their store's cloud saves get their newest save from Hearth Storage before they start, and send it back after. If both changed, you're asked.").switch(&p, sf, Some(set("savesFollow", sf))).el(&p));
        }
        if server {
            let bk = b(stc, "backups");
            rs.add(r("store", "Keep backups and saves for my devices").switch(&p, bk, Some(set("backups", bk))).el(&p));
            rs.add(h(&p, "Advanced"));
            let away = b(stc, "away");
            rs.add(r("update", "Away from home").sub("Your devices can use it when they aren't home, over Hearth Link's encrypted connection. No port is opened.").switch(&p, away, Some(set("away", away))).el(&p));
            let smb = b(stc, "smb");
            rs.add(r("display", "Windows sharing (SMB)").sub("The shared folders for Windows PCs on this network (port 445, this network only). Its password, media and downloads: Hearth Storage's extras, below.").switch(&p, smb, Some(set("smb", smb))).el(&p));
            rs.add(r("open", "Hearth Storage's extras").sub("The Windows sharing password, browsing the game archive, media and downloads. Opens in Hearth Game Mode (Desktop Mode closes first).").on(Msg::Detached(sv(&["nimbus-mode", "hearth-page", "storage"]))).el(&p));
        }
    }
    rs.add(message(&p, st));
    rs.el()
}

// ------------------------------------------------------------------ Your devices
fn ago_ms(m: f64) -> String {
    if m <= 0.0 {
        return "never".into();
    }
    let sec = (hearth_ui::time::now() as f64 - ms(m)).max(0.0);
    if sec < 90.0 {
        "just now".into()
    } else if sec < 3600.0 {
        format!("{} min ago", (sec / 60.0).round())
    } else if sec < 86400.0 {
        format!("{} h ago", (sec / 3600.0).round())
    } else {
        format!("{} days ago", (sec / 86400.0).round())
    }
}
fn up_text(x: &Value) -> String {
    if x.is_null() {
        return String::new();
    }
    let s_ = n(x, "") as i64;
    let (d, h_, m) = (s_ / 86400, s_ % 86400 / 3600, s_ % 3600 / 60);
    if d > 0 { format!("{d} d {h_} h") } else if h_ > 0 { format!("{h_} h {m} min") } else { format!("{m} min") }
}
fn gbs(x: f64) -> String {
    if x >= 100.0 { format!("{} GB", x.round()) } else { format!("{} GB", (x * 10.0).round() / 10.0) }
}
fn headline(d: &Value) -> String {
    if !b(d, "online") {
        return format!("Offline or asleep · seen {}", ago_ms(n(d, "lastSeen")));
    }
    if b(d, "old") {
        return "Online · update it to Hearth OS 1.6 to see more".into();
    }
    if !b(d, "shared") {
        return "Online · doesn't share its status".into();
    }
    let st_ = at(d, "status");
    let mode = match s(st_, "mode").as_str() {
        "game" => "Game Mode",
        "desktop" => "Desktop Mode",
        "server" => "Server mode",
        _ => "",
    };
    join_words(&["Online".into(), mode.into(), if b(st_, "playing") { format!("Playing {}", s(st_, "playing.title")) } else { String::new() }])
}
fn summary(s_: &Value) -> String {
    if !s_.is_object() {
        return String::new();
    }
    let mut out = Vec::new();
    if b(s_, "battery") {
        out.push(format!("Battery {}%{}", s(s_, "battery.percent"), if b(s_, "battery.charging") { " charging" } else { "" }));
    }
    if has(s_, "cpu.use") {
        out.push(format!("CPU {}%{}", s(s_, "cpu.use"), if has(s_, "cpu.temp") { format!(" · {}°C", s(s_, "cpu.temp")) } else { String::new() }));
    }
    if has(s_, "gpu.use") {
        out.push(format!("GPU {}%{}", s(s_, "gpu.use"), if has(s_, "gpu.temp") { format!(" · {}°C", s(s_, "gpu.temp")) } else { String::new() }));
    }
    if b(s_, "ram") {
        out.push(format!("RAM {} / {} GB", s(s_, "ram.used"), s(s_, "ram.total")));
    }
    out.join("   ")
}

fn device_card<'a>(p: &Pal, d: &Value, on: Option<Msg>) -> E<'a, Msg> {
    let p = *p;
    let s_ = at(d, "status");
    let online = b(d, "online");
    let ic = match s(s_, "kind").as_str() {
        "handheld" => "pad",
        "laptop" => "display",
        "desktop" => "cpu",
        _ => "desktops",
    };
    let mut words_ = Column::new().spacing(3.0).push(row![ts(if s(d, "name").is_empty() { "Device".into() } else { s(d, "name") }, sz(15.0), p.text), chip_of(&p, if online { "Online" } else { "Offline" }, if online { Ck::Ok } else { Ck::Warn })].spacing(8.0).align_y(iced::Center)).push(t(headline(d), sz(13.0), p.muted));
    let sm = summary(s_);
    if !sm.is_empty() {
        words_ = words_.push(t(sm, sz(13.0), p.text));
    }
    let mut line = Row::new().spacing(14.0).align_y(iced::Center).push(ibox(&p, icon_named(ic), online.then_some(p.accent))).push(words_.width(Length::Fill));
    if b(s_, "battery") || b(s_, "ram") {
        let (label, frac, warn) = if b(s_, "battery") {
            (format!("{}%", s(s_, "battery.percent")), n(s_, "battery.percent") / 100.0, n(s_, "battery.percent") < 20.0 && !b(s_, "battery.charging"))
        } else {
            let f = n(s_, "ram.used") / n(s_, "ram.total").max(1.0);
            (format!("{}% RAM", (100.0 * f).round()), f, false)
        };
        line = line.push(column![ts(label, sz(13.0), p.text), container(meter(&p, frac, warn)).width(Length::Fixed(90.0))].spacing(4.0).align_x(iced::alignment::Horizontal::Right));
    }
    if on.is_some() {
        line = line.push(hearth_ui::ui::glyph(hearth_ui::ui::G::Chevron, p.muted, 14.0, 2.0));
    }
    let c = container(line).padding(Padding { top: sz(14.0), right: 20.0, bottom: sz(14.0), left: 20.0 }).width(Length::Fill).style(ui::boxed(if online { p.panel } else { Pal::mix(p.panel, p.bg, 0.3) }, p.r(10.0), None));
    match on {
        Some(m) => iced::widget::mouse_area(c).on_press(m).interaction(iced::mouse::Interaction::Pointer).into(),
        None => c.into(),
    }
}

fn graph_card<'a>(p: &Pal, hist: &[Value], label: &str, unit: &str, key: &str) -> Option<E<'a, Msg>> {
    let pts: Vec<f32> = hist.iter().filter(|x| has(x, key)).map(|x| n(x, key) as f32).collect();
    if pts.len() < 2 {
        return None;
    }
    let last = pts[pts.len() - 1];
    let g = hearth_ui::ui::graph(hearth_ui::ui::Graph { values: pts, dashed: Vec::new(), max: 100.0, min: 0.0, color: p.accent, grid: p.line, fill: true, width: 2.0, centred: false, gridlines: Vec::new() }, Length::Fill, Length::Fixed(64.0));
    Some(container(column![row![t(label.to_string(), sz(12.0), p.muted), iced::widget::Space::new().width(Length::Fill), ts(format!("{}{unit}", last.round()), sz(15.0), p.text)], g].spacing(8.0)).padding(12.0).width(Length::FillPortion(1)).style(ui::boxed(p.panel, p.r(10.0), None)).into())
}

fn devices_page<'a>(a: &'a App) -> E<'a, Msg> {
    let p = a.sh.p;
    let st = &a.link;
    let dv = st.get("devices");
    let list = arr(dv, "devices");
    let detail = st.ls("detail");
    let cur = list.iter().find(|d| s(d, "id") == detail && !detail.is_empty());
    let mut rs = Rows::new();
    match cur {
        None => {
            let mut g = Vec::new();
            if dv.is_null() {
                g.push(words(&p, "Asking your devices…"));
            } else if list.is_empty() {
                g.push(if b(dv, "error") { words_c(s(dv, "error"), p.danger) } else { words(&p, "No other Hearth devices are paired yet. Pair one on Hearth Link's page.") });
            }
            for d in list {
                g.push(device_card(&p, d, Some(lv("detail", json!(s(d, "id"))))));
            }
            rs.add(group(&p, "Your other devices", g));
            let hs = st.get("hsq");
            if b(hs, "available") {
                let mut roles = Vec::new();
                if n(hs, "shares") > 0.0 {
                    roles.push(plural(n(hs, "shares"), "shared folder", "shared folders"));
                }
                if n(hs, "backups.count") > 0.0 {
                    roles.push("backups".to_string());
                }
                if n(hs, "people") > 0.0 {
                    roles.push(plural(n(hs, "people"), "person", "people"));
                }
                if b(hs, "ai.on") {
                    roles.push("AI".into());
                }
                if !roles.is_empty() {
                    let paused: Vec<String> = arr(hs, "paused").iter().map(vs).collect();
                    let sub = format!("{}{}", roles.join(" · "), if paused.is_empty() { String::new() } else { format!(" · paused while you play: {}", paused.join(", ")) });
                    rs.add(group(&p, "This PC is the home server", vec![r("server", format!("{} · home server", if s(hs, "host").is_empty() { "This PC".into() } else { s(hs, "host") })).sub(sub).chev().on(Msg::Go("hs-overview")).el(&p)]));
                }
            }
            let show = if st.l.contains_key("showDevices") { st.lb("showDevices") } else { at(st.get("prefs"), "showDevices") != &Value::Bool(false) };
            let set = act_v(H::Files, vec!["prefs-set".into(), json!({ "showDevices": !show }).to_string()]).set("showDevices", json!(!show)).no_reload();
            rs.add(group(&p, "This device", vec![share_status_row(a), r("folder", "Show other devices in Files").sub("Your devices' shared folders, and other computers sharing on this network or through Tailscale, in Hearth Files' sidebar and Open / Save windows").switch(&p, show, Some(set.msg())).el(&p)]));
        }
        Some(d) => {
            let s_ = at(d, "status");
            rs.add(r("back", "Back to your devices").on(lv("detail", Value::Null)).el(&p));
            rs.add(device_card(&p, d, None));
            let hist = arr(s_, "history");
            if hist.len() > 1 {
                let cards: Vec<E<'a, Msg>> = [("CPU", "%", "cpu"), ("GPU", "%", "gpu"), ("Memory", "%", "ram"), ("CPU temperature", "°C", "cpuTemp"), ("GPU temperature", "°C", "gpuTemp"), ("Battery", "%", "battery")].iter().filter_map(|(l, u, k)| graph_card(&p, hist, l, u, k)).collect();
                let mut grid = Column::new().spacing(10.0);
                let mut it = cards.into_iter();
                loop {
                    let mut rw = Row::new().spacing(10.0);
                    let mut k = 0;
                    for e in it.by_ref().take(3) {
                        rw = rw.push(e);
                        k += 1;
                    }
                    if k == 0 {
                        break;
                    }
                    grid = grid.push(rw);
                }
                rs.add(group(&p, "The last hour", vec![grid.into()]));
            }
            let srv = at(s_, "server");
            if srv.is_object() {
                let mut g = Vec::new();
                for al in arr(srv, "alerts") {
                    g.push(r("warn", vs(al)).el(&p));
                }
                for sv_ in arr(srv, "services") {
                    let state = s(sv_, "state");
                    g.push(r("server", format!("{}{}", s(sv_, "title"), if s(sv_, "engine").is_empty() { String::new() } else { format!(" · {}", s(sv_, "engine")) })).sub(join_words(&[s(sv_, "big"), s(sv_, "unit")]).replace(" · ", " ")).chip(s(sv_, "chip"), if state == "running" { Ck::Ok } else if state == "off" { Ck::Off } else { Ck::Warn }).el(&p));
                }
                let apps = arr(srv, "apps");
                if !apps.is_empty() {
                    g.push(r("box", "Server apps").sub(apps.iter().map(|x| format!("{}{}", s(x, "name"), match s(x, "state").as_str() { "active" => "", "failed" => " (stopped)", _ => " (starting)" })).collect::<Vec<_>>().join(" · ")).el(&p));
                }
                if b(d, "online") {
                    let host = if s(s_, "name").is_empty() { s(d, "name") } else { s(s_, "name") };
                    let port = if s(srv, "web").is_empty() { "8460".into() } else { s(srv, "web") };
                    g.push(r("globe", "Open its dashboard").sub("Its Server mode web page over Tailscale (its PIN): pause, restart, switch modes").chev().on(Msg::Open(format!("http://{host}:{port}"))).el(&p));
                }
                rs.add(group(&p, &format!("Its server · {}", s(srv, "status.title")), g));
            }
            if s_.is_object() {
                let mut g = Vec::new();
                let fact = |ic: &str, title: &str, value: String| -> Option<E<'a, Msg>> { (!value.is_empty()).then(|| r(ic, title).value(value).el(&p)) };
                g.extend(fact("cpu", "Processor", join_words(&[if has(s_, "cpu.use") { format!("{}% in use", s(s_, "cpu.use")) } else { String::new() }, if has(s_, "cpu.temp") { format!("{}°C", s(s_, "cpu.temp")) } else { String::new() }])));
                let gpu = at(s_, "gpu");
                let vendor = match s(gpu, "vendor").as_str() {
                    "nvidia" => "Nvidia".to_string(),
                    "amd" => "AMD".into(),
                    "intel" => "Intel".into(),
                    o => o.to_string(),
                };
                g.extend(fact("display", "Graphics", if gpu.is_object() { join_words(&[vendor, if has(gpu, "use") { format!("{}% in use", s(gpu, "use")) } else { String::new() }, if has(gpu, "temp") { format!("{}°C", s(gpu, "temp")) } else { String::new() }, if n(gpu, "vramTotal") > 0.0 { format!("graphics memory {} / {} GB", (n(gpu, "vramUsed") / 102.4).round() / 10.0, (n(gpu, "vramTotal") / 102.4).round() / 10.0) } else { String::new() }]) } else { String::new() }));
                g.extend(fact("grid", "Memory", if b(s_, "ram") { format!("{} of {} GB in use", s(s_, "ram.used"), s(s_, "ram.total")) } else { String::new() }));
                g.extend(fact("batt", "Battery", if b(s_, "battery") { format!("{}%{}", s(s_, "battery.percent"), if b(s_, "battery.charging") { ", charging" } else if b(s_, "battery.pluggedIn") { ", plugged in" } else { ", on battery" }) } else { String::new() }));
                for dr in arr(s_, "drives") {
                    g.push(r("drive", s(dr, "name")).sub(format!("{} of {} used", gbs(n(dr, "used")), gbs(n(dr, "total")))).value(format!("{}%", (100.0 * n(dr, "used") / n(dr, "total").max(1.0)).round())).el(&p));
                }
                g.extend(fact("update", "Hearth OS", s(s_, "version")));
                g.extend(fact("clock", "On for", up_text(at(s_, "uptime"))));
                rs.add(group(&p, if b(d, "stale") { "Last known (it's offline)" } else { "Now" }, g));
            }
            let mut g = Vec::new();
            let id = s(d, "id");
            if b(s_, "smb") {
                for sh_ in arr(s_, "shares") {
                    let badge = match s(sh_, "badge").as_str() {
                        "home" => "Home",
                        "drive" => "Whole drive",
                        "system" => "System drive",
                        _ => "",
                    };
                    let link = json!({ "device": id, "name": s(d, "name"), "share": s(sh_, "name"), "home": s(s_, "smb.home"), "away": s(s_, "smb.away"), "wake": at(d, "wake") == &Value::Bool(true), "reach": if s(sh_, "reach") == "tailscale" { "tailscale" } else { "home" }, "user": s(s_, "smb.user") });
                    let url = format!("hearthnet://{}", pct(&link.to_string()));
                    g.push(r("folder", format!("Open shared folder {}", s(sh_, "name"))).sub(join_words(&[badge.into(), if b(sh_, "write") { "Read & write".into() } else { "Read only".into() }, "in Hearth Files".into()])).chev().on(Msg::Detached(vec!["hearth-filemanager".into(), url])).el(&p));
                }
            }
            for sh_ in arr(d, "shares") {
                g.push(r("folder", format!("Open {}", s(sh_, "name"))).sub("Its shared folder in Hearth Files (Hearth Link)").chev().on(Msg::Detached(vec!["hearth-filemanager".into(), format!("hearth://{id}/{}/", s(sh_, "id"))])).el(&p));
            }
            if !b(d, "online") && at(d, "wake") == &Value::Bool(true) {
                g.push(r("power", "Wake it").sub("Wake-on-LAN, as Hearth Storage does").on(act_v(H::DevStatus, vec!["wake".into(), id.clone()]).ok(format!("Waking {}…", s(d, "name"))).msg()).el(&p));
            }
            if b(d, "online") {
                g.push(r("play", "Remote desktop").sub("Use it from here, or play its games (Remote play)").chev().on(Msg::Go("remote")).el(&p));
            }
            g.push(r("desktops", "Hearth Link").sub("Pairing, sync, copies and Hearth Storage").chev().on(Msg::Go("link")).el(&p));
            rs.add(group(&p, "Do something with it", g));
        }
    }
    rs.add(message(&p, st));
    rs.el()
}

/// encodeURIComponent.
fn pct(s_: &str) -> String {
    let mut o = String::new();
    for byte in s_.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            o.push(byte as char);
        } else {
            o.push_str(&format!("%{byte:02X}"));
        }
    }
    o
}

pub fn view<'a>(a: &'a App, page: &'static str) -> Option<Element<'a, Msg>> {
    match page {
        "link" => Some(link_page(a)),
        "devices" => Some(devices_page(a)),
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn codes_and_kinds() {
        assert!(short_check("K7M-4QX-92H").contains("10 characters"));
        assert!(short_check("K7M-4QX-92H0").contains("never have 0"));
        assert_eq!(short_fmt("k7m4qx92hc"), "K7M-4QX-92HC");
        assert_eq!(kind_of(&json!({"friend": true})), "friend");
        assert_eq!(kind_of(&json!({"relation": "family", "friend": false})), "family");
        assert_eq!(kind_of(&json!({"relation": "mine"})), "mine");
        assert_eq!(kind_args("ab12", "family"), vec!["make-family", "ab12"]);
        assert_eq!(kind_args("ab12", "friend"), vec!["make-friend", "ab12"]);
        assert_eq!(kind_args("ab12", "mine"), vec!["make-mine", "ab12"]);
        assert_eq!(share_args("ab12", "library", true), vec!["share", "ab12", "library", "on"]);
        assert!(their_line(&json!({"theirShares": {"saves": true}})).contains("its saves"));
        assert_eq!(kind_of(&json!({})), "mine");
        assert_eq!(pair_args(vec!["pair-open".into()], "family"), vec!["pair-open", "family"]);
        assert_eq!(summary_of(&json!({"peers": [{}, {}, {}]})), "3 devices");
    }
}
