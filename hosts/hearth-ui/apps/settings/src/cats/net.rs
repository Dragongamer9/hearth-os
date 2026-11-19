//! Network & internet (Mantel's SetWifi, SetHomeTailscale, SetVpn, SetHotspot, SetProxy and SetNetDetails, in the
//! approved Settings' look). See cats/mod.rs for the shape. Every change is NetworkManager's (nmcli, and Mantel's
//! hearth-network for the rest) or Hearth Home server's (hearth-home-server, Tailscale), only when you press: opening
//! a page only reads. Passwords go to nmcli or the helper on its stdin, never on a command line.
//!
//! Wi-Fi is drawn here; the other pages are their own files (net_*.rs), all with the same small shape.

#[path = "net_kit.rs"]
pub mod kit;
#[path = "net_ts.rs"]
pub mod ts;
#[path = "net_vpn.rs"]
pub mod vpn;
#[path = "net_more.rs"]
pub mod more;
#[path = "net_details.rs"]
pub mod details;

use crate::data;
use crate::shared::Shared;
use crate::widgets::*;
use crate::App;
use hearth_ui::run;
use hearth_ui::ui::{glyph, t, t1, tb, Kind, G};
use crate::widgets::btn;
use hearth_ui::Pal;
use iced::widget::{column, container, row, Column, Row, Space};
use iced::{Element, Length, Subscription, Task};
use kit::*;

pub const PAGES: &[&str] = &["wifi", "hs-tailscale", "vpn", "hotspot", "proxy", "net-details"];

/// A network nearby (nmcli's scan).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Net {
    pub ssid: String,
    pub signal: u32,
    pub secure: bool,
    pub in_use: bool,
    pub security: String,
    pub freq: u32,
}

/// What the Wi-Fi page read.
#[derive(Debug, Clone, Default)]
pub struct Wifi {
    pub has: bool,
    pub on: bool,
    pub nets: Vec<Net>,
    /// Each saved Wi-Fi network's SSID with its connection's UUID.
    pub saved: Vec<(String, String)>,
    pub device: String,
    pub ip: String,
    pub active: String,
    pub wired: String,
}

impl Wifi {
    pub fn uuid(&self, ssid: &str) -> Option<&str> {
        self.saved.iter().find(|s| s.0 == ssid).map(|s| s.1.as_str())
    }
    /// The network you're on: the scan's IN-USE mark, else NetworkManager's active connection on the Wi-Fi device (a
    /// mesh lists one name under several access points, and right after start-up the scan can lag the link).
    pub fn current(&self) -> Option<Net> {
        if let Some(n) = self.nets.iter().find(|n| n.in_use) {
            return Some(n.clone());
        }
        if self.active.is_empty() {
            return None;
        }
        Some(self.nets.iter().find(|n| n.ssid == self.active).cloned().unwrap_or(Net { ssid: self.active.clone(), in_use: true, ..Net::default() }))
    }
}

pub const WIFI: &str = r#"echo '#@radio'; nmcli radio wifi 2>/dev/null
echo '#@saved'; nmcli -t -f UUID,TYPE,NAME connection show 2>/dev/null | while IFS=: read -r u t n; do case "$t" in *wireless*) s=$(nmcli --escape yes -g 802-11-wireless.ssid connection show uuid "$u" 2>/dev/null); printf "%s:%s\n" "$u" "${s:-$n}";; esac; done
echo '#@list'; nmcli -t -f IN-USE,SSID,SIGNAL,SECURITY,FREQ device wifi list --rescan auto 2>/dev/null
echo '#@addr'; for d in $(nmcli -t -f DEVICE,TYPE device 2>/dev/null | awk -F: '$2=="wifi"{print $1}'); do a=$(nmcli -g IP4.ADDRESS device show "$d" 2>/dev/null | head -n1); s=""; if [ "$(nmcli -g GENERAL.STATE device show "$d" 2>/dev/null | cut -d" " -f1)" = 100 ]; then c=$(nmcli -g GENERAL.CONNECTION device show "$d" 2>/dev/null); [ -n "$c" ] && s=$(nmcli -g 802-11-wireless.ssid connection show id "$c" 2>/dev/null); s=${s:-$c}; fi; printf "%s\t%s\t%s\n" "$d" "${a%%/*}" "$s"; [ -n "$a" ] && break; done
echo '#@dev'; nmcli -t -f TYPE,STATE,CONNECTION dev 2>/dev/null"#;

fn lead_num(s: &str) -> u32 {
    s.trim().chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0)
}

pub fn parse_wifi(out: &str) -> Wifi {
    let m = data::sections(out);
    let sec = |k: &str| m.get(k).cloned().unwrap_or_default();
    let mut saved: Vec<(String, String)> = Vec::new();
    for l in sec("saved").lines() {
        let f = data::nm_fields(l);
        if f.len() >= 2 && !f[0].is_empty() && !f[1].is_empty() && !saved.iter().any(|s| s.0 == f[1]) {
            saved.push((f[1].clone(), f[0].clone()));
        }
    }
    // One row per name: a mesh lists the same name once per access point; the one you're on wins.
    let mut nets: Vec<Net> = Vec::new();
    for l in sec("list").lines() {
        let f = data::nm_fields(l);
        if f.len() < 4 || f[1].is_empty() {
            continue;
        }
        let sec3 = f[3].trim().to_string();
        let n = Net { in_use: f[0].trim() == "*", ssid: f[1].clone(), signal: lead_num(&f[2]), secure: !sec3.is_empty() && sec3 != "--", security: if sec3 == "--" { String::new() } else { sec3 }, freq: f.get(4).map(|x| lead_num(x)).unwrap_or(0) };
        match nets.iter_mut().find(|x| x.ssid == n.ssid) {
            Some(x) => {
                if n.in_use && !x.in_use {
                    *x = n;
                }
            }
            None => nets.push(n),
        }
    }
    nets.sort_by(|a, b| b.in_use.cmp(&a.in_use).then(b.signal.cmp(&a.signal)));
    let addr = sec("addr");
    let last = addr.lines().filter(|l| l.contains('\t')).last().unwrap_or("").to_string();
    let mut parts = last.split('\t');
    let device = parts.next().unwrap_or("").to_string();
    let ip = parts.next().unwrap_or("").to_string();
    let active = parts.next().unwrap_or("").trim().to_string();
    let dev = sec("dev");
    let wired = dev.lines().map(data::nm_fields).find(|f| f.first().map(|t| t == "ethernet").unwrap_or(false) && f.get(1).map(|s| s == "connected").unwrap_or(false)).and_then(|f| f.get(2).cloned()).unwrap_or_default();
    Wifi { has: dev.lines().any(|l| l.starts_with("wifi:")), on: sec("radio").trim() == "enabled", nets, saved, device, ip, active, wired }
}

/// nmcli's `device show` (-t): "IP4.ADDRESS[1]:192.168.1.5/24" lines, the first of each.
pub fn parse_props(out: &str) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = Vec::new();
    for l in out.lines() {
        let Some(i) = l.find(':') else { continue };
        if i == 0 {
            continue;
        }
        let k = l[..i].split('[').next().unwrap_or("").to_string();
        let val = l[i + 1..].replace("\\:", ":");
        if !val.is_empty() && !v.iter().any(|x| x.0 == k) {
            v.push((k, val));
        }
    }
    v
}

pub fn band(f: u32) -> &'static str {
    if f >= 5925 {
        "6 GHz"
    } else if f >= 4900 {
        "5 GHz"
    } else if f > 0 {
        "2.4 GHz"
    } else {
        ""
    }
}
fn signal_word(s: u32) -> &'static str {
    if s > 75 {
        "Excellent signal"
    } else if s > 50 {
        "Good signal"
    } else if s > 25 {
        "Fair signal"
    } else {
        "Weak signal"
    }
}
fn bars(s: u32) -> &'static str {
    if s > 75 {
        "▂▄▆█"
    } else if s > 50 {
        "▂▄▆"
    } else if s > 25 {
        "▂▄"
    } else {
        "▂"
    }
}

#[derive(Default)]
pub struct State {
    pub wifi: Wifi,
    pub read: bool,
    pub scanning: bool,
    pub busy: String,
    pub asking: Option<String>,
    pub pw: String,
    pub show_all: bool,
    pub show_props: bool,
    pub props: Vec<(String, String)>,
    pub forget_ask: bool,
    pub hidden_open: bool,
    pub hidden_name: String,
    pub hidden_pw: String,
    pub say: Say,
    pub live: Live,
    pub ts: ts::S,
    pub vpn: vpn::S,
    pub more: more::S,
    pub det: details::S,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Got(run::Output),
    Props(run::Output),
    Did(String, run::Output),
    On(bool),
    Look,
    Pick(String),
    Pw(String),
    Join,
    Disconnect(String),
    ForgetAsk(bool),
    Forget(String),
    ShowProps,
    ShowAll,
    Hidden,
    HName(String),
    HPw(String),
    HJoin,
    Tick,
    Go(&'static str),
    Ts(ts::M),
    Vpn(vpn::M),
    More(more::M),
    Det(details::M),
}

fn read() -> Task<Msg> {
    Task::perform(exec(data::sh(WIFI), String::new(), 30), Msg::Got)
}
fn nm(cmd: Vec<String>, input: String, ok: String) -> Task<Msg> {
    Task::perform(exec(cmd, input, 60), move |o| Msg::Did(ok.clone(), o))
}

pub fn open(sh: &mut Shared, st: &mut State, page: &'static str) -> Task<Msg> {
    let part = std::mem::take(&mut sh.part);
    match page {
        "hs-tailscale" => return ts::open(sh, &mut st.ts, &part).map(Msg::Ts),
        "vpn" => return vpn::open(sh, &mut st.vpn, &part).map(Msg::Vpn),
        "hotspot" | "proxy" => return more::open(sh, &mut st.more, page, &part).map(Msg::More),
        "net-details" => return details::open(sh, &mut st.det, &part).map(Msg::Det),
        _ => {}
    }
    // Folds closed again each time the page opens (Mantel: when you leave it).
    st.show_all = false;
    st.show_props = false;
    st.forget_ask = false;
    st.hidden_open = part == "hidden";
    st.asking = None;
    st.pw.clear();
    st.say.clear();
    st.scanning = true;
    st.live.start("wifi");
    if part == "props" {
        st.show_props = true;
    }
    read()
}

pub fn update(sh: &mut Shared, st: &mut State, m: Msg) -> Task<Msg> {
    match m {
        Msg::Got(o) => {
            st.wifi = parse_wifi(&o.out);
            st.read = true;
            st.scanning = false;
            if st.show_props && st.props.is_empty() {
                return props(st);
            }
        }
        Msg::Props(o) => st.props = parse_props(&o.out),
        Msg::Did(ok, o) => {
            st.busy.clear();
            if o.ok {
                st.say.ok(ok);
            } else {
                let why = o.why();
                st.say.bad(if why.is_empty() { "That didn't work".to_string() } else { why });
            }
            st.scanning = true;
            return read();
        }
        Msg::On(on) => {
            st.wifi.on = on;
            return nm(kit::a(&["nmcli", "radio", "wifi", if on { "on" } else { "off" }]), String::new(), if on { "Wi-Fi on".into() } else { "Wi-Fi off".into() });
        }
        Msg::Look => {
            st.scanning = true;
            return read();
        }
        Msg::Pick(ssid) => {
            let Some(n) = st.wifi.nets.iter().find(|n| n.ssid == ssid).cloned() else { return Task::none() };
            if n.in_use {
                return disconnect(st, &n.ssid);
            }
            if n.secure && st.wifi.uuid(&n.ssid).is_none() {
                st.pw.clear();
                st.asking = if st.asking.as_deref() == Some(ssid.as_str()) { None } else { Some(ssid) };
                return Task::none();
            }
            return connect(st, &n.ssid, String::new());
        }
        Msg::Pw(p) => st.pw = p,
        Msg::Join => {
            if let Some(ssid) = st.asking.take() {
                let pw = std::mem::take(&mut st.pw);
                return connect(st, &ssid, pw);
            }
        }
        Msg::Disconnect(ssid) => return disconnect(st, &ssid),
        Msg::ForgetAsk(on) => st.forget_ask = on,
        Msg::Forget(ssid) => {
            st.forget_ask = false;
            st.show_props = false;
            let cmd = match st.wifi.uuid(&ssid) {
                Some(u) => kit::a(&["nmcli", "connection", "delete", "uuid", u]),
                None => vec!["nmcli".into(), "connection".into(), "delete".into(), "id".into(), ssid.clone()],
            };
            return nm(cmd, String::new(), format!("Forgot {ssid}"));
        }
        Msg::ShowProps => {
            st.show_props = !st.show_props;
            if st.show_props {
                return props(st);
            }
        }
        Msg::ShowAll => st.show_all = !st.show_all,
        Msg::Hidden => st.hidden_open = !st.hidden_open,
        Msg::HName(t) => st.hidden_name = t,
        Msg::HPw(t) => st.hidden_pw = t,
        Msg::HJoin => {
            // A network that doesn't say its name: nmcli connects to it by name, with "hidden yes".
            let name = st.hidden_name.trim().to_string();
            let pw = std::mem::take(&mut st.hidden_pw);
            st.hidden_name.clear();
            if name.is_empty() {
                return Task::none();
            }
            st.busy = name.clone();
            st.say.clear();
            st.hidden_open = false;
            let mut cmd = vec!["nmcli".to_string()];
            if !pw.is_empty() {
                cmd.push("--ask".into());
            }
            cmd.extend(["device", "wifi", "connect"].iter().map(|s| s.to_string()));
            cmd.push(name.clone());
            cmd.extend(["hidden".to_string(), "yes".to_string()]);
            return nm(cmd, if pw.is_empty() { String::new() } else { format!("{pw}\n") }, format!("Connected to {name}"));
        }
        Msg::Tick => {
            if st.live.still(15) && st.asking.is_none() && st.busy.is_empty() {
                return read();
            }
        }
        Msg::Go(id) => sh.go(id),
        Msg::Ts(x) => return ts::update(sh, &mut st.ts, x).map(Msg::Ts),
        Msg::Vpn(x) => return vpn::update(sh, &mut st.vpn, x).map(Msg::Vpn),
        Msg::More(x) => return more::update(sh, &mut st.more, x).map(Msg::More),
        Msg::Det(x) => return details::update(sh, &mut st.det, x).map(Msg::Det),
    }
    Task::none()
}

fn props(st: &State) -> Task<Msg> {
    let dev = if st.wifi.device.is_empty() { "wlan0".to_string() } else { st.wifi.device.clone() };
    Task::perform(exec(vec!["nmcli".into(), "-t".into(), "-f".into(), "IP4.ADDRESS,IP4.GATEWAY,IP4.DNS,IP6.ADDRESS,GENERAL.HWADDR".into(), "device".into(), "show".into(), dev], String::new(), 10), Msg::Props)
}

fn connect(st: &mut State, ssid: &str, pw: String) -> Task<Msg> {
    st.busy = ssid.to_string();
    st.asking = None;
    st.say.clear();
    let ok = format!("Connected to {ssid}");
    // The password goes to nmcli on its input (it asks for it with --ask): on its command line every account on the
    // device could see it.
    if !pw.is_empty() {
        return nm(vec!["nmcli".into(), "--ask".into(), "device".into(), "wifi".into(), "connect".into(), ssid.to_string()], format!("{pw}\n"), ok);
    }
    if let Some(u) = st.wifi.uuid(ssid) {
        return nm(kit::a(&["nmcli", "connection", "up", "uuid", u]), String::new(), ok);
    }
    nm(vec!["nmcli".into(), "device".into(), "wifi".into(), "connect".into(), ssid.to_string()], String::new(), ok)
}

fn disconnect(st: &mut State, ssid: &str) -> Task<Msg> {
    let cmd = match st.wifi.uuid(ssid) {
        Some(u) => kit::a(&["nmcli", "connection", "down", "uuid", u]),
        None => vec!["nmcli".into(), "connection".into(), "down".into(), "id".into(), ssid.to_string()],
    };
    nm(cmd, String::new(), format!("Disconnected from {ssid}"))
}

pub fn view<'a>(a: &'a App, page: &'static str) -> Option<Element<'a, Msg>> {
    let st = &a.net;
    match page {
        "wifi" => Some(wifi(a)),
        "hs-tailscale" => Some(ts::view(&a.sh, &st.ts).map(Msg::Ts)),
        "vpn" => Some(vpn::view(&a.sh, &st.vpn).map(Msg::Vpn)),
        "hotspot" | "proxy" => Some(more::view(&a.sh, &st.more, page).map(Msg::More)),
        "net-details" => Some(details::view(&a.sh, &st.det).map(Msg::Det)),
        _ => None,
    }
}

fn wifi(a: &App) -> Element<'_, Msg> {
    let p = a.sh.p;
    let st = &a.net;
    st.live.seen();
    let w = &st.wifi;
    let mut c = Column::new().spacing(10.0);
    if !st.read {
        return column![note(&p, "Looking at your network…", false), more_links(&p)].spacing(10.0).into();
    }
    if !w.wired.is_empty() {
        c = c.push(info_row(&p, G::Link, "Ethernet", format!("Connected · {}", w.wired)));
    }
    if !w.has {
        return c.push(note(&p, "This PC has no Wi-Fi.", false)).push(more_links(&p)).into();
    }
    let cur = if w.on { w.current() } else { None };
    // ---- the network you're on
    if let Some(n) = &cur {
        let line = dots(&[
            format!("Connected{}", if n.secure { ", secured" } else { ", open" }),
            if n.signal > 0 { format!("{} {}", signal_word(n.signal), bars(n.signal)) } else { String::new() },
            band(n.freq).to_string(),
            w.ip.clone(),
        ]);
        let ic = container(glyph(G::Wifi, p.accent, sz(28.0), 2.0)).center_x(Length::Fixed(sz(56.0))).center_y(Length::Fixed(sz(56.0))).style(hearth_ui::ui::boxed(Pal::a(p.accent, 0.18), p.r(10.0), None));
        let mut acts = Row::new().spacing(8.0).push(btn(&p, if st.show_props { "Hide properties" } else { "Properties" }, Kind::Normal, false, Some(Msg::ShowProps))).push(btn(&p, "Disconnect", Kind::Normal, false, Some(Msg::Disconnect(n.ssid.clone()))));
        if w.uuid(&n.ssid).is_some() {
            acts = acts.push(btn(&p, "Forget", Kind::Normal, false, Some(Msg::ForgetAsk(true))));
        }
        let mut inner = Column::new().spacing(12.0).push(row![ic, column![tb(n.ssid.clone(), sz(20.0), p.text), t(line, sz(13.0), p.muted)].spacing(2.0).width(Length::Fill), acts].spacing(16.0).align_y(iced::Center));
        if st.forget_ask {
            inner = inner.push(row![
                t(format!("Forget {}? This device won't join it by itself any more, and you'll need its password again.", n.ssid), sz(13.0), p.text).width(Length::Fill),
                btn(&p, "Forget", Kind::Primary, false, Some(Msg::Forget(n.ssid.clone()))),
                btn(&p, "Cancel", Kind::Normal, false, Some(Msg::ForgetAsk(false)))
            ].spacing(8.0).align_y(iced::Center));
        }
        if st.show_props {
            let pr = |k: &str| st.props.iter().find(|x| x.0 == k).map(|x| x.1.clone()).unwrap_or_default();
            let rows: Vec<(&str, String)> = vec![
                ("Security", if !n.security.is_empty() { n.security.clone() } else if n.secure { "Secured".into() } else { "Open".into() }),
                ("Band", format!("{}{}", band(n.freq), if n.freq > 0 { format!(" ({} MHz)", n.freq) } else { String::new() })),
                ("Signal", if n.signal > 0 { format!("{}%", n.signal) } else { String::new() }),
                ("IPv4 address", { let v = pr("IP4.ADDRESS"); if v.is_empty() { w.ip.clone() } else { v } }),
                ("Gateway", pr("IP4.GATEWAY")),
                ("DNS", pr("IP4.DNS")),
                ("IPv6 address", pr("IP6.ADDRESS")),
                ("Physical address (MAC)", pr("GENERAL.HWADDR")),
            ];
            let mut g = Column::new().spacing(6.0);
            for (k, v) in rows.into_iter().filter(|r| !r.1.trim().is_empty()) {
                g = g.push(row![container(t(k, sz(13.0), p.muted)).width(Length::Fixed(200.0)), t(v, sz(13.0), p.text)]);
            }
            inner = inner.push(g);
        }
        c = c.push(card(&p, inner));
    }
    let sub = if !w.on { "Off".to_string() } else if let Some(n) = &cur { format!("Connected to {}", n.ssid) } else { "Not connected".into() };
    c = c.push(srow(&p, G::Wifi, "Wi-Fi", sub, switch(&p, w.on, Some(Msg::On(!w.on)))));
    c = c.push(said(&p, &st.say));
    if w.on {
        c = c.push(head(&p, if st.scanning { "Networks nearby · looking…" } else { "Networks nearby" }));
        // A short list (the network you're on, saved ones, then the strongest others, up to 6); the rest behind
        // "Show all networks".
        let first: Vec<&Net> = w.nets.iter().filter(|n| n.in_use || w.uuid(&n.ssid).is_some() || st.busy == n.ssid).collect();
        let others: Vec<&Net> = w.nets.iter().filter(|n| !(n.in_use || w.uuid(&n.ssid).is_some() || st.busy == n.ssid)).collect();
        let room = 6usize.saturating_sub(first.len());
        let shown: Vec<&Net> = if st.show_all { first.iter().chain(others.iter()).copied().collect() } else { first.iter().copied().chain(others.iter().copied().take(room)).collect() };
        for n in &shown {
            let saved = w.uuid(&n.ssid).is_some();
            let sub = if n.in_use {
                format!("Connected{}", if n.secure { ", secured" } else { "" })
            } else if st.busy == n.ssid {
                "Connecting…".into()
            } else if saved {
                "Saved".into()
            } else if n.secure {
                "Secured".into()
            } else {
                "Open: anyone can see what you send".into()
            };
            let mut right = Row::new().spacing(10.0).align_y(iced::Center);
            if n.in_use {
                right = right.push(chipk(&p, "Connected", "ok"));
            }
            right = right.push(t1(bars(n.signal), sz(13.0), p.muted));
            if n.secure {
                right = right.push(glyph(G::Shield, p.muted, 14.0, 1.8));
            }
            c = c.push(xrow(&p, G::Wifi, n.ssid.clone(), sub, Some(right.into()), false, Some(Msg::Pick(n.ssid.clone()))));
            if st.asking.as_deref() == Some(n.ssid.as_str()) {
                c = c.push(indent(row![
                    input(&p, &format!("Password for {}", n.ssid), &st.pw, true, Msg::Pw, Some(Msg::Join)),
                    btn(&p, "Connect", Kind::Primary, false, if st.pw.chars().count() >= 8 { Some(Msg::Join) } else { None })
                ].spacing(8.0).align_y(iced::Center)));
            }
        }
        if w.nets.is_empty() {
            c = c.push(words(&p, if st.scanning { "Looking for networks…" } else { "No networks found. Look again in a moment." }));
        }
        let more_n = w.nets.len().saturating_sub(shown.len());
        if st.show_all && others.len() > room || !st.show_all && more_n > 0 {
            c = c.push(press(&p, G::Wifi, if st.show_all { "Show fewer networks".to_string() } else { format!("Show all networks ({more_n} more)") }, "", Some(Msg::ShowAll)));
        }
        // ---- a network that doesn't say its name
        c = c.push(expander(&p, G::Plus, "Add a hidden network", "A network that doesn't show its name: type it and its password", "", None, st.hidden_open, Msg::Hidden, Some(column![
            input(&p, "Network name (SSID)", &st.hidden_name, false, Msg::HName, None),
            input(&p, "Password (leave empty for an open network)", &st.hidden_pw, true, Msg::HPw, Some(Msg::HJoin)),
            container(btn(&p, "Connect", Kind::Primary, false, if st.hidden_name.trim().is_empty() { None } else { Some(Msg::HJoin) }))
        ].spacing(6.0).into())));
        c = c.push(press(&p, G::Update, "Look again", "", Some(Msg::Look)));
    } else {
        c = c.push(words(&p, "Turn on Wi-Fi to see networks."));
    }
    c.push(more_links(&p)).into()
}

/// Wi-Fi is the category's first page: the others are listed under it.
fn more_links<'a>(p: &Pal) -> Element<'a, Msg> {
    let mut c = Column::new().spacing(8.0).push(head(p, "More network settings"));
    for (id, g, title, sub) in [
        ("hs-tailscale", G::Link, "Tailscale", "Reach your devices from anywhere, privately"),
        ("vpn", G::Shield, "VPN", "WireGuard and OpenVPN from their file, IPsec, L2TP, PPTP, OpenConnect, SSTP and Fortinet by hand"),
        ("hotspot", G::Wifi, "Mobile hotspot", "Share this PC's internet with your other devices over Wi-Fi"),
        ("proxy", G::Server, "Proxy", "A proxy for apps and browsers, by hand or from a setup script"),
        ("net-details", G::Gear, "Connection details", "DNS, IP address, MAC address, 802.1X, mobile broadband, a QR code to share Wi-Fi, duplicate"),
    ] {
        c = c.push(go_row(p, g, title, sub, Msg::Go(id)));
    }
    c.push(Space::new().height(Length::Fixed(4.0))).into()
}

pub fn subscription(st: &State) -> Subscription<Msg> {
    let mut s = Vec::new();
    if st.live.running("wifi") && st.asking.is_none() {
        s.push(iced::time::every(std::time::Duration::from_secs(15)).map(|_| Msg::Tick));
    }
    s.push(ts::subscription(&st.ts).map(Msg::Ts));
    s.push(more::subscription(&st.more).map(Msg::More));
    s.push(vpn::subscription(&st.vpn).map(Msg::Vpn));
    s.push(details::subscription(&st.det).map(Msg::Det));
    Subscription::batch(s)
}

pub fn dialog<'a>(a: &'a App) -> Option<(Element<'a, Msg>, Option<Msg>)> {
    kit::set_current(a.page);
    None
}

pub fn back(st: &mut State) -> bool {
    if kit::showing(&["wifi"]) {
        if st.asking.take().is_some() {
            return true;
        }
        if st.forget_ask {
            st.forget_ask = false;
            return true;
        }
    }
    (kit::showing(&["vpn"]) && vpn::back(&mut st.vpn)) || (kit::showing(&["net-details"]) && details::back(&mut st.det))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_wifi() {
        let out = "#@radio\nenabled\n#@saved\n1111-aa:Home-5G\n#@list\n:Cafe:40:--:2412 MHz\n*:Home-5G:82:WPA2:5180 MHz\n:Home-5G:60:WPA2:2437 MHz\n#@addr\nwlan0\t192.168.1.20\tHome-5G\n#@dev\nwifi:connected:Home-5G\nethernet:unavailable:\n";
        let w = parse_wifi(out);
        assert!(w.has && w.on);
        assert_eq!(w.nets.len(), 2);
        assert_eq!(w.nets[0].ssid, "Home-5G");
        assert_eq!(w.nets[0].freq, 5180);
        assert!(!w.nets[1].secure);
        assert_eq!(w.uuid("Home-5G"), Some("1111-aa"));
        assert_eq!(w.ip, "192.168.1.20");
        assert_eq!(w.current().map(|n| n.ssid), Some("Home-5G".to_string()));
        assert_eq!(band(5180), "5 GHz");
        let pr = parse_props("IP4.ADDRESS[1]:192.168.1.20/24\nGENERAL.HWADDR:AA\\:BB\n");
        assert_eq!(pr[1].1, "AA:BB");
    }
}
