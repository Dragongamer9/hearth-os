//! Bluetooth & devices › Printers, Print queue, Scanners, Scan to PDF and Ink & toner (Mantel's SetPrinters,
//! SetPrintQueue, SetScanners, SetScanPdf and SetInkLevels), with the same helpers: CUPS through hearth-system (your
//! printers, a test page) and Hearth's admin helper (adding one CUPS finds or by its address, the default, removing
//! one), hearth-printq (the queue: hold, go on, cancel; the scanners SANE finds and a test scan), hearth-scan (pages
//! into one PDF in Documents; the pages wait in memory until you save or clear them) and hearth-ink (how full each
//! cartridge is). The notice when ink runs low is Hearth's own setting (desktop.json, through hearth_settings_io).

use crate::cats::net::kit::{self, *};
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::run;
use hearth_ui::ui::{self, btn, glyph, t, tm, Kind, G};
use hearth_ui::Pal;
use iced::widget::{column, container, image, row, Column, Row, Space};
use iced::{Color, Length, Subscription, Task};
use serde_json::{json, Value};

#[derive(Default)]
pub struct S {
    pub page: &'static str,
    pub live: Live,
    pub say: Say,
    // printers
    pub st: Option<Value>,
    pub found: Option<Vec<Value>>,
    pub picked: String,
    pub confirm: String,
    pub admin: &'static str,
    pub what: &'static str,
    pub addr: String,
    // queue
    pub q: Option<Value>,
    pub tick: u64,
    // scanners
    pub scanners: Option<Vec<Value>>,
    pub testing: String,
    pub last_ok: String,
    pub picture: Option<image::Handle>,
    // scan to pdf
    pub device: String,
    pub opts: Option<Value>,
    pub dpi: i64,
    pub mode: String,
    pub source: String,
    pub pages: Vec<(String, image::Handle)>,
    pub name: String,
    pub saved: String,
    pub open: &'static str,
    // ink
    pub ink: Option<Value>,
    /// `hearth-settings scanners:test`: a test scan of the first scanner found, once.
    pub auto_test: bool,
}

#[derive(Debug, Clone)]
pub enum M {
    Printers(Res),
    Pick(String),
    Confirm(String),
    Test(String, String),
    Admin(Vec<String>, &'static str, String),
    AdminDone(&'static str, String, Res),
    Discover,
    Discovered(Res),
    Add(String, String, String),
    Addr(String),
    AddAddr,
    Tool,
    Done(String, Res),
    Go(&'static str),
    Tick,
    Queue(Res),
    Job(&'static str, String, String),
    Scanners(Res),
    ScanTest(String),
    Tested(Res),
    Look,
    Pdf(&'static str),
    Options(Res),
    Pages(Res),
    Device(String),
    Dpi(i64),
    Mode(String),
    Source(String),
    Scan,
    Up(usize),
    Drop(usize),
    Name(String),
    Save,
    SavedPdf(Res),
    OpenPdf,
    Clear,
    Ink(Res),
    InkAgain,
    InkNotify(bool),
    Wrote(run::Output),
}

const HS: &str = "hearth-system";
const PQ: &str = "hearth-printq";
const SC: &str = "hearth-scan";

fn printers() -> Task<M> {
    Task::perform(kit::job(HS, kit::a(&["printers"]), String::new()), M::Printers)
}
fn queue() -> Task<M> {
    Task::perform(kit::job(PQ, kit::a(&["jobs"]), String::new()), M::Queue)
}
fn scanners() -> Task<M> {
    Task::perform(kit::job_for(PQ, kit::a(&["scanners"]), String::new(), 90), M::Scanners)
}
fn ink() -> Task<M> {
    Task::perform(kit::job("hearth-ink", kit::a(&["levels"]), String::new()), M::Ink)
}

fn handle(path: &str) -> Option<image::Handle> {
    std::fs::read(path).ok().map(image::Handle::from_bytes)
}

pub fn open(_sh: &mut Shared, s: &mut S, page: &'static str, part: &str) -> Task<M> {
    s.page = page;
    s.auto_test = page == "scanners" && part == "test";
    s.say.clear();
    s.live.stop();
    match page {
        "printers" => {
            s.picked.clear();
            s.confirm.clear();
            s.live.start("printers");
            printers()
        }
        "print-queue" => {
            s.live.start("print-queue");
            queue()
        }
        "scanners" => {
            s.scanners = None;
            scanners()
        }
        "scan-pdf" => {
            s.scanners = None;
            s.saved.clear();
            if s.dpi == 0 {
                s.dpi = 150;
            }
            Task::batch([Task::perform(kit::job(SC, kit::a(&["pages"]), String::new()), M::Pages), scanners()])
        }
        _ => {
            s.ink = None;
            ink()
        }
    }
}

fn kind_of(uri: &str) -> &'static str {
    if uri.starts_with("usb:") {
        "USB"
    } else if ["dnssd:", "ipp:", "ipps:", "http:", "https:", "socket:", "lpd:"].iter().any(|p| uri.starts_with(p)) {
        "Network"
    } else {
        ""
    }
}
fn state_text(p: &Value) -> String {
    let st = kit::st(p, "state");
    let s = match st.as_str() {
        "printing" => "Printing",
        "disabled" => "Paused",
        _ => "Ready",
    };
    let reason = kit::st(p, "reason");
    let loc = kit::st(p, "location");
    format!("{s}{}{}", if !reason.is_empty() && st != "idle" { format!(" · {reason}") } else { String::new() }, if loc.is_empty() { String::new() } else { format!(" · {loc}") })
}

/// A CUPS printer name from what the printer calls itself (letters, numbers, - _ .), not already used.
pub fn name_for(info: &str, taken: &[String]) -> String {
    let mut n = String::new();
    let mut last_us = false;
    for ch in info.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' || ch == '-' {
            n.push(ch);
            last_us = false;
        } else if !last_us {
            n.push('_');
            last_us = true;
        }
    }
    let n: String = n.trim_start_matches(|c: char| !c.is_ascii_alphanumeric()).trim_end_matches('_').chars().take(60).collect();
    let n = if n.is_empty() { "Printer".to_string() } else { n };
    let mut k = n.clone();
    let mut i = 2;
    while taken.iter().any(|t| t.eq_ignore_ascii_case(&k)) {
        k = format!("{n}_{i}");
        i += 1;
    }
    k
}

fn host_ok(h: &str) -> bool {
    if h.starts_with('[') && h.ends_with(']') {
        return h.len() >= 4 && h.len() <= 47 && h[1..h.len() - 1].chars().all(|c| c.is_ascii_hexdigit() || c == ':');
    }
    !h.is_empty() && h.len() <= 252 && h.chars().next().map(|c| c.is_ascii_alphanumeric()).unwrap_or(false) && h.chars().last().map(|c| c.is_ascii_alphanumeric()).unwrap_or(false) && h.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
}

fn add(s: &mut S, uri: String, info: String, make: String) -> Task<M> {
    let taken: Vec<String> = s.st.as_ref().map(|v| kit::ar(v, "printers").iter().map(|p| kit::st(p, "name")).collect()).unwrap_or_default();
    let label = if !info.is_empty() { info } else if !make.is_empty() { make } else { "Printer".to_string() };
    let name = name_for(&label, &taken);
    let info: String = label.chars().filter(|c| !c.is_control()).take(120).collect();
    run_admin(s, vec!["printer-add".into(), name, uri, "--info".into(), info.clone()], "add", format!("{info} is ready to use."))
}

fn run_admin(s: &mut S, args: Vec<String>, label: &'static str, ok: String) -> Task<M> {
    s.say.clear();
    s.admin = label;
    Task::perform(kit::admin(args), move |r| M::AdminDone(label, ok.clone(), r))
}

fn got_pages(s: &mut S, r: &Res) {
    match r {
        Res::Ok(v) => {
            s.pages = kit::ar(v, "pages").iter().filter_map(|p| { let path = kit::st(p, "path"); handle(&path).map(|h| (path, h)) }).collect();
        }
        Res::Err(e) => s.say.bad(e.clone()),
    }
}

pub fn update(sh: &mut Shared, s: &mut S, m: M) -> Task<M> {
    match m {
        M::Printers(r) => match r {
            Res::Ok(v) => s.st = Some(v),
            Res::Err(e) => s.say.bad(e),
        },
        M::Pick(n) => {
            s.picked = if s.picked == n { String::new() } else { n };
            s.confirm.clear();
        }
        M::Confirm(n) => s.confirm = n,
        M::Test(name, label) => {
            s.what = "test";
            return Task::perform(kit::job(HS, vec!["printer-test".into(), name], String::new()), move |r| M::Done(format!("Test page sent to {label}."), r));
        }
        M::Admin(args, label, ok) => return run_admin(s, args, label, ok),
        M::AdminDone(label, ok, r) => {
            s.admin = "";
            s.say.res(&r, &ok);
            if r.val().is_some() {
                if label == "add" {
                    s.found = None;
                }
                if label == "remove" {
                    s.picked.clear();
                    s.confirm.clear();
                }
            }
            return printers();
        }
        M::Discover => {
            s.say.clear();
            s.admin = "discover";
            return Task::perform(kit::admin(kit::a(&["printer-discover"])), M::Discovered);
        }
        M::Discovered(r) => {
            s.admin = "";
            match r {
                Res::Ok(v) => s.found = Some(kit::ar(&v, "devices").to_vec()),
                Res::Err(e) => s.say.bad(e),
            }
        }
        M::Add(uri, info, make) => return add(s, uri, info, make),
        M::Addr(t) => s.addr = t,
        M::AddAddr => {
            let h = s.addr.trim().to_string();
            if !host_ok(&h) {
                s.say.bad("Type the printer's address, like 192.168.1.20 or printer.local.");
                return Task::none();
            }
            s.addr.clear();
            return add(s, format!("ipp://{h}/ipp/print"), format!("Printer at {h}"), String::new());
        }
        M::Tool => run::detached(&["system-config-printer"]),
        M::Done(ok, r) => {
            s.what = "";
            s.say.res(&r, &ok);
            if s.page == "print-queue" {
                return queue();
            }
        }
        M::Go(id) => sh.go(id),
        M::Tick => {
            s.tick += 1;
            if s.live.running("print-queue") && s.live.still(4) {
                return queue();
            }
            if s.live.running("printers") && s.live.still(15) && s.admin.is_empty() {
                return printers();
            }
        }
        M::Queue(r) => match r {
            Res::Ok(v) => s.q = Some(v),
            Res::Err(e) => {
                s.q = Some(json!({ "printers": [], "jobs": [], "done": [] }));
                s.say.bad(e);
            }
        },
        M::Job(what, id, ok) => {
            let mut args = vec![what.to_string()];
            if !id.is_empty() {
                args.push(id);
            }
            return Task::perform(kit::job(PQ, args, String::new()), move |r| M::Done(ok.clone(), r));
        }
        M::Scanners(r) => {
            let l = match r {
                Res::Ok(v) => kit::ar(&v, "scanners").to_vec(),
                Res::Err(e) => {
                    s.say.bad(e);
                    Vec::new()
                }
            };
            if s.page == "scan-pdf" {
                if let Some(first) = l.first() {
                    let d = kit::st(first, "device");
                    s.scanners = Some(l);
                    return update(sh, s, M::Device(d));
                }
            }
            let first = l.first().map(|x| kit::st(x, "device"));
            s.scanners = Some(l);
            if std::mem::take(&mut s.auto_test) {
                if let Some(d) = first {
                    return update(sh, s, M::ScanTest(d));
                }
            }
        }
        M::ScanTest(dev) => {
            if !s.testing.is_empty() {
                return Task::none();
            }
            s.testing = dev.clone();
            s.say.clear();
            return Task::perform(kit::job_for(PQ, vec!["scan-test".into(), dev], String::new(), 200), M::Tested);
        }
        M::Tested(r) => {
            match r {
                Res::Ok(v) => {
                    s.picture = handle(&kit::st(&v, "file"));
                    s.last_ok = s.testing.clone();
                    s.say.ok("It scanned. This is the test picture.");
                }
                Res::Err(e) => s.say.bad(e),
            }
            s.testing.clear();
        }
        M::Look => {
            if s.testing.is_empty() {
                s.scanners = None;
                return scanners();
            }
        }
        M::Pdf(o) => s.open = if s.open == o { "" } else { o },
        M::Device(d) => {
            s.open = "";
            s.device = d.clone();
            s.opts = None;
            s.source.clear();
            s.mode.clear();
            return Task::perform(kit::job(SC, vec!["options".into(), d], String::new()), M::Options);
        }
        M::Options(r) => match r {
            Res::Ok(v) => {
                let modes = kit::strs(&v, "modes");
                let sources = kit::strs(&v, "sources");
                let res: Vec<i64> = kit::ar(&v, "resolutions").iter().filter_map(|x| x.as_i64()).collect();
                s.mode = if modes.iter().any(|m| m == "Color") { "Color".into() } else { modes.first().cloned().unwrap_or_default() };
                s.source = sources.iter().find(|x| x.to_lowercase().contains("flatbed")).cloned().or_else(|| sources.first().cloned()).unwrap_or_default();
                if !res.contains(&s.dpi) {
                    s.dpi = if res.contains(&300) { 300 } else { res.first().copied().unwrap_or(150) };
                }
                s.opts = Some(v);
            }
            Res::Err(e) => s.say.bad(e),
        },
        M::Pages(r) => {
            s.what = "";
            got_pages(s, &r);
        }
        M::Dpi(d) => {
            s.dpi = d;
            s.open = "";
        }
        M::Mode(m) => {
            s.mode = m;
            s.open = "";
        }
        M::Source(x) => {
            s.source = x;
            s.open = "";
        }
        M::Scan => {
            if !s.what.is_empty() {
                return Task::none();
            }
            s.say.clear();
            s.saved.clear();
            s.what = "scan";
            let feeder = feeder(&s.source);
            return Task::perform(kit::job_for(SC, vec![if feeder { "feeder" } else { "page" }.into(), s.device.clone(), s.dpi.to_string(), s.mode.clone(), s.source.clone()], String::new(), 1800), M::Pages);
        }
        M::Up(i) => return Task::perform(kit::job(SC, vec!["up".into(), i.to_string()], String::new()), M::Pages),
        M::Drop(i) => return Task::perform(kit::job(SC, vec!["remove".into(), i.to_string()], String::new()), M::Pages),
        M::Name(t) => s.name = t,
        M::Save => {
            if s.what.is_empty() {
                s.what = "save";
                return Task::perform(kit::job(SC, vec!["save".into(), s.name.clone()], String::new()), M::SavedPdf);
            }
        }
        M::SavedPdf(r) => {
            s.what = "";
            match r {
                Res::Ok(v) => {
                    s.saved = kit::st(&v, "file");
                    let n = kit::nu(&v, "pages").unwrap_or(0.0) as i64;
                    let base = s.saved.rsplit('/').next().unwrap_or("").to_string();
                    s.say.ok(format!("Saved {base} ({n}{}", if n == 1 { " page)." } else { " pages)." }));
                }
                Res::Err(e) => s.say.bad(e),
            }
        }
        M::OpenPdf => run::detached(&["xdg-open", s.saved.as_str()]),
        M::Clear => {
            s.saved.clear();
            return Task::perform(kit::job(SC, kit::a(&["clear"]), String::new()), M::Pages);
        }
        M::Ink(r) => match r {
            Res::Ok(v) => s.ink = Some(v),
            Res::Err(e) => {
                s.ink = Some(json!({ "printers": [] }));
                s.say.bad(e);
            }
        },
        M::InkAgain => return ink(),
        M::InkNotify(on) => {
            let cmd = sh.desktop_cmd(json!({ "inkNotify": on }));
            return Task::perform(run::spawn(cmd), M::Wrote);
        }
        M::Wrote(o) => sh.result("Couldn't save that", &o),
    }
    Task::none()
}

fn feeder(source: &str) -> bool {
    let l = source.to_lowercase();
    l.contains("adf") || l.contains("feeder") || l.contains("duplex")
}

pub fn view<'a>(sh: &'a Shared, s: &'a S, page: &'static str) -> E<'a, M> {
    s.live.seen();
    match page {
        "printers" => printers_view(sh, s),
        "print-queue" => queue_view(sh, s),
        "scanners" => scanners_view(sh, s),
        "scan-pdf" => pdf_view(sh, s),
        _ => ink_view(sh, s),
    }
}

fn printers_view<'a>(sh: &'a Shared, s: &'a S) -> E<'a, M> {
    let p = sh.p;
    let mut c = Column::new().spacing(10.0);
    let Some(st) = &s.st else { return c.push(words(&p, "Asking the printing service…")).push(said(&p, &s.say)).into() };
    let cups = kit::bo(st, "cups");
    let can = kit::bo(st, "canAdmin");
    if !cups {
        c = c.push(head(&p, "Printing"));
        c = c.push(info_row(&p, G::Info, "The printing service isn't running", "Hearth uses CUPS for printing. It starts by itself when something prints; if this stays, restart the device."));
    }
    let list = kit::ar(st, "printers");
    if !list.is_empty() {
        c = c.push(head(&p, "My printers"));
    }
    for pr in list {
        let name = kit::st(pr, "name");
        let info = kit::st(pr, "info");
        let label = if info.is_empty() { name.clone() } else { info };
        let k = kind_of(&kit::st(pr, "uri"));
        let def = kit::bo(pr, "default");
        c = c.push(val_row(&p, G::Grid, label.clone(), format!("{}{}", state_text(pr), if k.is_empty() { String::new() } else { format!(" · {k}") }), if def { "Default" } else { "" }, Some(M::Pick(name.clone()))));
        if s.picked == name {
            let mut x = Column::new().spacing(8.0);
            x = x.push(press(&p, G::Grid, if s.what == "test" { "Sending…" } else { "Print a test page" }, "", Some(M::Test(name.clone(), label.clone()))));
            if !def && can {
                x = x.push(press(&p, G::Check, "Make it the default", "", Some(M::Admin(vec!["printer-default".into(), name.clone()], "default", format!("{label} is now the default printer.")))));
            }
            if s.confirm != name && can {
                x = x.push(danger_row(&p, G::Close, "Remove this printer…", "", Some(M::Confirm(name.clone()))));
            }
            if s.confirm == name {
                x = x.push(danger_row(&p, G::Close, if s.admin == "remove" { "Removing…".to_string() } else { format!("Yes, remove {label}") }, "", Some(M::Admin(vec!["printer-remove".into(), name.clone()], "remove", "Printer removed.".into()))));
                x = x.push(press(&p, G::Close, "Cancel", "", Some(M::Confirm(String::new()))));
            }
            c = c.push(indent(x));
        }
    }
    if cups && list.is_empty() {
        c = c.push(head(&p, "My printers"));
        c = c.push(words(&p, "No printers yet."));
    }
    // The print queue and scanners (KDE's print queue window and scanner check).
    c = c.push(head(&p, "Queue and scanners"));
    c = c.push(go_row(&p, G::List, "Print queue", "What's waiting to print: hold, go on or cancel", M::Go("print-queue")));
    c = c.push(go_row(&p, G::Grid, "Scanners", "The scanners found, and a test scan", M::Go("scanners")));
    c = c.push(go_row(&p, G::Folder, "Scan to PDF", "Several pages into one PDF", M::Go("scan-pdf")));
    c = c.push(go_row(&p, G::Grid, "Ink & toner", "How full each cartridge is", M::Go("ink-levels")));
    if can && cups {
        c = c.push(head(&p, "Add a printer"));
        c = c.push(press(&p, G::Search, if s.admin == "discover" { "Looking for printers…" } else if s.found.is_none() { "Look for printers" } else { "Look again" }, "On your network and plugged in by USB. Turn the printer on first.", if s.admin.is_empty() { Some(M::Discover) } else { None }));
        let uris: Vec<String> = list.iter().map(|p| kit::st(p, "uri")).collect();
        if let Some(found) = &s.found {
            let new: Vec<&Value> = found.iter().filter(|d| !uris.contains(&kit::st(d, "uri"))).collect();
            for d in &new {
                let (uri, info, make, loc) = (kit::st(d, "uri"), kit::st(d, "info"), kit::st(d, "makeModel"), kit::st(d, "location"));
                let title = if !info.is_empty() { info.clone() } else if !make.is_empty() { make.clone() } else { uri.clone() };
                let k = kind_of(&uri);
                let sub = format!("{}{}{}", if k.is_empty() { "Printer" } else { k }, if !make.is_empty() && make != info { format!(" · {make}") } else { String::new() }, if loc.is_empty() { String::new() } else { format!(" · {loc}") });
                c = c.push(val_row(&p, G::Grid, title, sub, if s.admin == "add" { "" } else { "Add" }, Some(M::Add(uri, info, make))));
            }
            if new.is_empty() && s.admin != "discover" {
                c = c.push(words(&p, "No new printers found. Check it's on and on the same network, or type its address below."));
            }
        }
        c = c.push(input(&p, "Or its network address, like 192.168.1.20", &s.addr, false, M::Addr, Some(M::AddAddr)));
        c = c.push(press(&p, G::Grid, if s.admin == "add" { "Adding…" } else { "Add by address" }, "For network printers that work without a driver (most printers from the last ten years)", Some(M::AddAddr)));
        if kit::bo(st, "configTool") {
            c = c.push(go_row(&p, G::Gear, "More printer settings", "Drivers, paper and other options (Print Settings)", M::Tool));
        }
    }
    c = c.push(said(&p, &s.say));
    if !can {
        c = c.push(words(&p, "Printers are added here on Hearth, the OS."));
    }
    c.into()
}

fn kb(n: f64) -> String {
    if n >= 1048576.0 {
        format!("{:.1} MB", n / 1048576.0)
    } else {
        format!("{} KB", ((n / 1024.0).round() as i64).max(1))
    }
}

fn queue_view<'a>(sh: &'a Shared, s: &'a S) -> E<'a, M> {
    let p = sh.p;
    let mut c = Column::new().spacing(10.0);
    let Some(q) = &s.q else { return c.push(words(&p, "Asking the printing service…")).into() };
    let jobs = kit::ar(q, "jobs");
    let printers = kit::ar(q, "printers");
    for pr in printers {
        let name = kit::st(pr, "name");
        let mine: Vec<&Value> = jobs.iter().filter(|j| kit::st(j, "printer") == name).collect();
        let st = kit::st(pr, "state");
        c = c.push(head(&p, format!("{} · {}{}", name.replace('_', " "), if st == "printing" { "printing" } else if st == "disabled" { "paused" } else { "ready" }, if mine.is_empty() { String::new() } else { format!(" · {} waiting", mine.len()) })));
        if mine.is_empty() {
            c = c.push(info_row(&p, G::Grid, "Nothing waiting", kit::st(pr, "reason")));
        }
        for j in mine {
            let js = kit::st(j, "state");
            let title = { let t0 = kit::st(j, "title"); if t0.is_empty() { format!("Job {}", kit::st(j, "num")) } else { t0 } };
            let sub = kit::dots(&[kit::st(j, "user"), kb(kit::nu(j, "size").unwrap_or(0.0)), kit::st(j, "when")]);
            let (chip, kind) = match js.as_str() {
                "printing" => ("Printing", "ok"),
                "held" => ("Held", "warn"),
                _ => ("Waiting", "ok"),
            };
            c = c.push(chip_row(&p, if js == "printing" { G::Grid } else if js == "held" { G::Stop } else { G::List }, title, sub, chip, kind, None));
            if js == "printing" {
                // The one printing now: a bar that moves along each time the queue is read again.
                let at = (s.tick % 3) as u16;
                c = c.push(row![Space::new().width(Length::FillPortion(1 + at * 3)), container(Space::new()).width(Length::FillPortion(4)).height(Length::Fixed(5.0)).style(ui::boxed(p.accent, 3.0, None)), Space::new().width(Length::FillPortion(7 - at * 3))]);
            }
            if kit::bo(j, "mine") {
                let id = kit::st(j, "id");
                let mut b = Row::new().spacing(8.0);
                if js != "held" {
                    b = b.push(btn(&p, "Hold", Kind::Normal, false, Some(M::Job("hold", id.clone(), "Held.".into()))));
                } else {
                    b = b.push(btn(&p, "Go on", Kind::Normal, false, Some(M::Job("resume", id.clone(), "Going on.".into()))));
                }
                b = b.push(btn(&p, "Cancel", Kind::Normal, false, Some(M::Job("cancel", id.clone(), "Cancelled.".into()))));
                c = c.push(container(b).padding(iced::Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 4.0 }));
            }
        }
    }
    if printers.is_empty() {
        c = c.push(info_row(&p, G::Grid, "No printers", "Add one in Printers"));
    }
    let mine = jobs.iter().filter(|j| kit::bo(j, "mine")).count();
    if mine > 1 {
        c = c.push(danger_row(&p, G::Close, "Cancel all mine", format!("{mine} of your jobs"), Some(M::Job("cancel-all", String::new(), "Cancelled.".into()))));
    }
    let done = kit::ar(q, "done");
    if !done.is_empty() {
        c = c.push(head(&p, "Printed lately"));
        for d in done.iter().take(8) {
            c = c.push(info_row(&p, G::Check, format!("{} · job {}", kit::st(d, "printer").replace('_', " "), kit::st(d, "num")), kit::dots(&[kit::st(d, "user"), kb(kit::nu(d, "size").unwrap_or(0.0)), kit::st(d, "when")])));
        }
    }
    c.push(said(&p, &s.say)).into()
}

fn scanners_view<'a>(sh: &'a Shared, s: &'a S) -> E<'a, M> {
    let p = sh.p;
    let mut c = Column::new().spacing(10.0);
    let title = match &s.scanners {
        None => "Looking for scanners…",
        Some(l) if !l.is_empty() => "Scanners found",
        _ => "No scanners found",
    };
    c = c.push(head(&p, title));
    if let Some(l) = &s.scanners {
        if l.is_empty() {
            c = c.push(info_row(&p, G::Info, "Nothing answered", "Turn the scanner on and plug it in, or put it on the same network. Many printers with a scanner show up here"));
        }
        for sc in l {
            let dev = kit::st(sc, "device");
            let net = dev.starts_with("airscan:") || dev.starts_with("escl:");
            let usb = dev.to_lowercase().contains("usb");
            let sub = format!("{}{}", if net { "On the network · " } else if usb { "USB · " } else { "" }, if s.testing == dev { "Scanning a test… (up to a minute)" } else { "Test scan" });
            let works = s.picture.is_some() && s.testing.is_empty() && s.last_ok == dev;
            c = c.push(chip_row(&p, G::Grid, kit::st(sc, "name"), sub, if works { "Works" } else { "" }, "ok", Some(M::ScanTest(dev.clone()))));
        }
    }
    c = c.push(press(&p, G::Update, "Look again", "", Some(M::Look)));
    c = c.push(go_row(&p, G::Folder, "Scan to PDF", "Several pages into one PDF", M::Go("scan-pdf")));
    if let Some(h) = &s.picture {
        c = c.push(container(image(h.clone()).width(Length::Fill).height(Length::Fixed(312.0))).padding(14.0).width(Length::Fill).style(ui::boxed(p.panel, p.r(12.0), None)));
    }
    c.push(said(&p, &s.say)).into()
}

fn mode_word(m: &str) -> String {
    match m {
        "Color" => "Colour".into(),
        "Gray" => "Greyscale".into(),
        "Lineart" => "Black and white".into(),
        o => o.into(),
    }
}

fn pdf_view<'a>(sh: &'a Shared, s: &'a S) -> E<'a, M> {
    let p = sh.p;
    let mut c = Column::new().spacing(10.0);
    let list = s.scanners.as_deref();
    c = c.push(head(&p, match list {
        None => "Looking for scanners…",
        Some(l) if !l.is_empty() => "Scanner",
        _ => "No scanners found",
    }));
    if let Some(l) = list {
        if l.is_empty() {
            c = c.push(info_row(&p, G::Info, "Nothing answered", "Turn the scanner on and plug it in, or put it on the same network"));
        } else {
            let cur = l.iter().find(|x| kit::st(x, "device") == s.device).map(|x| kit::st(x, "name")).unwrap_or_default();
            c = c.push(dropdown(&p, G::Grid, "Scanner", "", cur, s.open == "scanner", M::Pdf("scanner"), l.iter().map(|x| (kit::st(x, "name"), kit::st(x, "device") == s.device, M::Device(kit::st(x, "device")))).collect()));
        }
    }
    if let Some(o) = &s.opts {
        let res: Vec<i64> = kit::ar(o, "resolutions").iter().filter_map(|x| x.as_i64()).collect();
        c = c.push(dropdown(&p, G::Palette, "Resolution", "150 for reading, 300 for printing again", format!("{} dpi", s.dpi), s.open == "dpi", M::Pdf("dpi"), res.iter().map(|r| (format!("{r} dpi"), *r == s.dpi, M::Dpi(*r))).collect()));
        let modes = kit::strs(o, "modes");
        if modes.len() > 1 {
            c = c.push(dropdown(&p, G::Palette, "Colour", "", mode_word(&s.mode), s.open == "mode", M::Pdf("mode"), modes.iter().map(|m| (mode_word(m), *m == s.mode, M::Mode(m.clone()))).collect()));
        }
        let sources = kit::strs(o, "sources");
        if sources.len() > 1 {
            c = c.push(dropdown(&p, G::List, "From", "", s.source.clone(), s.open == "source", M::Pdf("source"), sources.iter().map(|x| (x.clone(), *x == s.source, M::Source(x.clone()))).collect()));
        }
    }
    let n = s.pages.len();
    c = c.push(head(&p, if n == 0 { "Pages".to_string() } else { format!("{n} {} so far", if n == 1 { "page" } else { "pages" }) }));
    if s.opts.is_some() {
        let fd = feeder(&s.source);
        c = c.push(press(&p, G::Plus, if s.what == "scan" { "Scanning…" } else if fd { "Scan every page in the feeder" } else if n > 0 { "Scan the next page" } else { "Scan a page" }, if fd { "All the paper in the feeder, one page after another" } else { "Put the page on the glass, then press. Repeat for each page" }, Some(M::Scan)));
    }
    if n > 0 {
        let mut grid = Column::new().spacing(12.0);
        for (ci, chunk) in s.pages.chunks(5).enumerate() {
            let mut r = Row::new().spacing(12.0);
            for (j, (_path, h)) in chunk.iter().enumerate() {
                let i = ci * 5 + j;
                let mut acts = Row::new().spacing(4.0).push(Space::new().width(Length::Fill));
                if i > 0 {
                    acts = acts.push(btn(&p, "Up", Kind::Normal, false, Some(M::Up(i))));
                }
                acts = acts.push(btn(&p, "Remove", Kind::Normal, false, Some(M::Drop(i))));
                r = r.push(container(column![
                    container(image(h.clone()).width(Length::Fixed(130.0)).height(Length::Fixed(172.0))).center_x(Length::Fill),
                    tm(format!("Page {}", i + 1), sz(13.0), p.text),
                    acts,
                ].spacing(6.0)).padding(10.0).width(Length::Fixed(170.0)).style(ui::boxed(p.panel2, 10.0, None)));
            }
            grid = grid.push(r);
        }
        c = c.push(grid);
        c = c.push(input(&p, "Name, like Tax return 2026 (or the date)", &s.name, false, M::Name, Some(M::Save)));
        c = c.push(press(&p, G::Folder, if s.what == "save" { "Saving…" } else { "Save as PDF" }, "One PDF of every page, in Documents", Some(M::Save)));
    }
    if !s.saved.is_empty() {
        c = c.push(go_row(&p, G::Folder, "Open the PDF", kit::tilde(&s.saved), M::OpenPdf));
    }
    if n > 0 {
        c = c.push(danger_row(&p, G::Close, "Start again", "Clears the pages here (a saved PDF stays)", Some(M::Clear)));
    }
    c.push(said(&p, &s.say)).into()
}

fn ink_colour(p: &Pal, hex: &str) -> Color {
    let h = hex.trim_start_matches('#');
    match u32::from_str_radix(h, 16) {
        Ok(n) if h.len() == 6 => Color::from_rgb8(((n >> 16) & 255) as u8, ((n >> 8) & 255) as u8, (n & 255) as u8),
        _ => p.muted,
    }
}

fn ink_view<'a>(sh: &'a Shared, s: &'a S) -> E<'a, M> {
    let p = sh.p;
    let mut c = Column::new().spacing(10.0);
    c = c.push(head(&p, "Notices"));
    let on = sh.d_bool("inkNotify", true);
    c = c.push(srow(&p, G::Moon, "Tell me when ink or toner runs low", "One notice per cartridge, again only after it's been filled", switch(&p, on, Some(M::InkNotify(!on)))));
    let Some(st) = &s.ink else { return c.push(info_row(&p, G::Grid, "Asking the printers…", "")).into() };
    let list = kit::ar(st, "printers");
    if list.is_empty() {
        c = c.push(info_row(&p, G::Grid, "No printers", if s.say.text.is_empty() { "Add one in Printers".to_string() } else { s.say.text.clone() }));
    }
    for pr in list {
        let markers = kit::ar(pr, "markers");
        let known: Vec<&Value> = markers.iter().filter(|m| kit::nu(m, "level").unwrap_or(-1.0) >= 0.0).collect();
        let mut x = Column::new().spacing(10.0).push(row![glyph(G::Grid, p.muted, 20.0, 1.8), tm(kit::st(pr, "name").replace('_', " "), sz(16.0), p.text)].spacing(10.0).align_y(iced::Center));
        if markers.is_empty() || known.is_empty() {
            x = x.push(t(if markers.is_empty() { "This printer doesn't say how full it is" } else { "Some left (the printer doesn't give numbers)" }, sz(13.0), p.muted));
        }
        for m in known {
            let level = kit::nu(m, "level").unwrap_or(0.0);
            let low_at = kit::nu(m, "low").filter(|v| *v > 0.0).unwrap_or(15.0).max(1.0);
            let low = level <= low_at;
            let hex = kit::st(m, "color");
            let dot = ink_colour(&p, &hex);
            let fill = if !hex.is_empty() && hex != "000000" && hex != "ffffff" { dot } else { p.accent };
            let mut r = Row::new().spacing(12.0).align_y(iced::Center)
                .push(container(Space::new()).width(Length::Fixed(14.0)).height(Length::Fixed(14.0)).style(ui::boxed(dot, 7.0, Some((p.panel3, 1.0)))))
                .push(container(t(kit::st(m, "name"), sz(13.0), p.text)).width(Length::Fixed(170.0)))
                .push(container(level_bar(&p, (level / 100.0).max(0.02) as f32, fill)).width(Length::Fill))
                .push(container(tm(format!("{} %", level as i64), sz(13.0), if low { p.danger } else { p.text })).width(Length::Fixed(46.0)));
            r = r.push(if low { chipk(&p, "Low", "bad") } else { Space::new().width(Length::Fixed(38.0)).into() });
            x = x.push(r);
        }
        c = c.push(card(&p, x));
    }
    c = c.push(press(&p, G::Update, "Ask again", "", Some(M::InkAgain)));
    c.into()
}

pub fn subscription(s: &S) -> Subscription<M> {
    if s.live.running("print-queue") {
        iced::time::every(std::time::Duration::from_secs(4)).map(|_| M::Tick)
    } else if s.live.running("printers") && s.admin.is_empty() {
        iced::time::every(std::time::Duration::from_secs(15)).map(|_| M::Tick)
    } else {
        Subscription::none()
    }
}

pub fn back(s: &mut S) -> bool {
    if !s.open.is_empty() {
        s.open = "";
        return true;
    }
    if !s.confirm.is_empty() {
        s.confirm.clear();
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn printer_names_and_addresses() {
        assert_eq!(name_for("HP LaserJet Pro M404", &[]), "HP_LaserJet_Pro_M404");
        assert_eq!(name_for("HP LaserJet Pro M404", &["hp_laserjet_pro_m404".into()]), "HP_LaserJet_Pro_M404_2");
        assert_eq!(name_for("***", &[]), "Printer");
        assert!(host_ok("192.168.1.20") && host_ok("printer.local") && host_ok("[fe80::1]"));
        assert!(!host_ok("a b") && !host_ok("-x"));
        assert_eq!(kind_of("usb://HP"), "USB");
        assert_eq!(kb(2048.0), "2 KB");
    }
}
