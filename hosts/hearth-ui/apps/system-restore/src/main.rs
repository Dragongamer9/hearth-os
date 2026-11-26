//! System Restore, in pure Rust (Mantel's RestoreView.qml, the approved mockup, ported; docs/SYSTEM-RESTORE.md):
//! the restore points on the left (date, what, type) with "Running now" first, "What will change" on the right with
//! "See every changed setting", and Make a restore point now / Restore and restart / Cancel at the bottom; the
//! confirm, every-changed-setting, working and done dialogs.
//!
//! Everything that changes something runs hearth_restore.py (Mantel's shell folder), which keeps a copy of today's
//! settings first and only uses Hearth's own `hearth-admin rollback` for the version. Opening it writes nothing.
//! Mouse, keyboard and the controller (as keys): Up / Down pick a point, Right goes to "See every changed setting",
//! Down past the list goes to the buttons, Left / Right move between them, Enter presses, Esc closes.

use hearth_ui::app::{self, win};
use hearth_ui::run;
use hearth_ui::time;
use hearth_ui::ui::{self, btn, check, chip, gap, glyph, hline, t, t1, tm, ts, Kind, TitleMsgs, G};
use hearth_ui::Pal;
use iced::keyboard::{key::Named, Event as KEvent, Key};
use iced::widget::{column, container, mouse_area, row, scrollable, Column};
use iced::{mouse, Color, Element, Length, Padding, Subscription, Task};
use serde_json::Value;

const SHORT: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    Loading,
    Idle,
    Confirm,
    Working,
    Changes,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Zone {
    List,
    Older,
    Aside,
    Footer,
}

#[derive(Debug, Clone)]
enum Msg {
    /// Describe it (hearth_ui::describe; every Hearth app has it)
    Describe(hearth_ui::describe::DMsg),
    Theme,
    Overview(run::Output),
    Snapped(run::Output),
    ChangesRead(run::Output),
    Applied(run::Output),
    Pick(usize),
    ToggleOlder,
    MakePoint,
    AskRestore,
    SeeChanges,
    Dlg(&'static str),
    Close,
    Reboot,
    Key(KEvent),
    Drag,
    Maximize,
    Minimize,
    Resize(iced::window::Direction),
}

struct App {
    pub describe: hearth_ui::describe::Describe,
    p: Pal,
    helper: String,
    ov: Option<Value>,
    phase: Phase,
    error: String,
    done: String,
    done_restart: bool,
    show_older: bool,
    sel: usize,
    zone: Zone,
    foot: usize,
    dlg: usize,
    change_list: Vec<Value>,
    added_list: Vec<String>,
}

fn s(v: &Value, k: &str) -> String {
    match v.get(k) {
        Some(Value::String(x)) => x.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}
fn n(v: &Value, k: &str) -> i64 {
    v.get(k).and_then(|x| x.as_i64().or_else(|| x.as_f64().map(|f| f as i64))).unwrap_or(0)
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let helper = std::env::var("HEARTH_RESTORE_HELPER").unwrap_or_else(|_| run::shell_helper("hearth_restore.py"));
        let mut a = App {
            describe: hearth_ui::describe::Describe::new("system-restore", "System Restore").with_pages(BLOCKS).with_kit(&KIT),
            p: hearth_ui::theme::load(),
            helper,
            ov: None,
            phase: Phase::Loading,
            error: String::new(),
            done: String::new(),
            done_restart: false,
            show_older: false,
            sel: 1,
            zone: Zone::List,
            foot: 1,
            dlg: 0,
            change_list: Vec::new(),
            added_list: Vec::new(),
        };
        // --show confirm|changes (CI's pictures): the dialog opens once the points are read.
        if app::flag("--older") {
            a.show_older = true;
        }
        let t = a.load();
        (a, t)
    }

    fn load(&self) -> Task<Msg> {
        Task::perform(run::spawn(run::args(&["python3", &self.helper, "overview"])), Msg::Overview)
    }

    fn points(&self) -> Vec<Value> {
        self.ov.as_ref().and_then(|o| o.get("points")).and_then(Value::as_array).cloned().unwrap_or_default()
    }
    fn visible(&self) -> Vec<Value> {
        let p = self.points();
        if self.show_older { p } else { p.into_iter().take(SHORT).collect() }
    }
    fn running(&self) -> String {
        let Some(o) = self.ov.as_ref() else { return String::new() };
        let r = s(o, "running");
        if !r.is_empty() {
            return r;
        }
        o.get("status").and_then(|x| x.get("booted")).map(|b| s(b, "version")).unwrap_or_default()
    }
    fn rows(&self) -> usize {
        1 + self.visible().len()
    }
    fn has_older(&self) -> bool {
        self.points().len() > SHORT
    }
    fn cur(&self) -> Option<Value> {
        let v = self.visible();
        if self.sel >= 1 && self.sel <= v.len() { Some(v[self.sel - 1].clone()) } else { None }
    }
    fn action(&self) -> String {
        self.cur().and_then(|c| c.get("plan").map(|p| s(p, "action"))).unwrap_or_else(|| "none".into())
    }
    fn version_goes_back(&self) -> bool {
        matches!(self.action().as_str(), "rollback" | "queued")
    }
    fn can_restore(&self) -> bool {
        match self.cur() {
            Some(c) => self.phase == Phase::Idle && (self.version_goes_back() || n(&c, "changed") > 0),
            None => false,
        }
    }
    fn go_label(&self) -> &'static str {
        if self.cur().is_none() || self.version_goes_back() {
            "Restore and restart"
        } else if self.action() == "blocked" {
            "Restore settings only"
        } else {
            "Restore settings"
        }
    }
    fn footer(&self) -> &'static str {
        match self.cur() {
            None => "",
            Some(_) if self.version_goes_back() => "Then Hearth restarts (about 2 minutes)",
            Some(c) if n(&c, "changed") > 0 => "No restart needed",
            Some(_) => "Nothing to put back",
        }
    }
    fn dlg_buttons(&self) -> Vec<&'static str> {
        match self.phase {
            Phase::Confirm => if self.version_goes_back() { vec!["go", "settings", "cancel"] } else { vec!["go", "cancel"] },
            Phase::Done => if self.done_restart { vec![] } else { vec!["restart", "ok"] },
            Phase::Changes => vec!["ok"],
            _ => vec![],
        }
    }

    fn make_point(&mut self) -> Task<Msg> {
        if self.phase != Phase::Idle {
            return Task::none();
        }
        self.phase = Phase::Working;
        self.error.clear();
        Task::perform(run::spawn(run::args(&["python3", &self.helper, "snapshot", "--kind", "manual"])), Msg::Snapped)
    }
    fn ask(&mut self) {
        if self.can_restore() {
            self.phase = Phase::Confirm;
            self.dlg = 0;
        }
    }
    fn restore(&mut self, settings_only: bool) -> Task<Msg> {
        let Some(c) = self.cur() else { return Task::none() };
        if self.phase == Phase::Working {
            return Task::none();
        }
        self.phase = Phase::Working;
        self.error.clear();
        let mut cmd = run::args(&["python3", &self.helper, "apply", &s(&c, "id")]);
        if settings_only || !self.version_goes_back() {
            cmd.push("--settings-only".into());
        }
        Task::perform(run::spawn(cmd), Msg::Applied)
    }
    fn see_changes(&mut self) -> Task<Msg> {
        let Some(c) = self.cur() else { return Task::none() };
        self.change_list.clear();
        self.added_list.clear();
        self.phase = Phase::Changes;
        self.dlg = 0;
        Task::perform(run::spawn(run::args(&["python3", &self.helper, "changes", &s(&c, "id")])), Msg::ChangesRead)
    }
    fn pick(&mut self, i: usize) {
        self.sel = i.min(self.rows().saturating_sub(1));
        self.zone = Zone::List;
    }
    fn dlg_press(&mut self, b: &str) -> Task<Msg> {
        match b {
            "go" => self.restore(false),
            "settings" => self.restore(true),
            "restart" => {
                run::detached(&["systemctl", "reboot"]);
                Task::none()
            }
            _ => {
                self.phase = Phase::Idle;
                Task::none()
            }
        }
    }
}

fn update(a: &mut App, m: Msg) -> Task<Msg> {
    match m {
        Msg::Describe(dm) => {
            // one of System Restore's buttons dragged in (drag and drop), pressed
            if let Some(m) = hearth_ui::studio::act(&dm).and_then(|id| block_action(a, id)) {
                return update(a, m);
            }
            return a.describe.update(dm).map(Msg::Describe);
        }
        Msg::Theme => a.p = hearth_ui::theme::load(),
        Msg::Overview(o) => {
            match o.json() {
                Some(d) if d.get("points").map(Value::is_array).unwrap_or(false) => {
                    let len = d["points"].as_array().map(|x| x.len()).unwrap_or(0);
                    a.ov = Some(d);
                    a.sel = a.sel.max(if len > 0 { 1 } else { 0 }).min(len.min(if a.show_older { len } else { SHORT }));
                    a.error.clear();
                }
                _ => a.error = "System Restore couldn't read its restore points.".into(),
            }
            if a.phase == Phase::Loading {
                a.phase = Phase::Idle;
                // CI's pictures: open a dialog once the points are in.
                match app::arg("--show").as_deref() {
                    Some("confirm") => a.ask(),
                    Some("changes") => return a.see_changes(),
                    _ => {}
                }
            }
        }
        Msg::Snapped(o) => {
            a.phase = Phase::Idle;
            match o.json() {
                Some(d) if d.get("ok").and_then(Value::as_bool) != Some(false) => {
                    a.error.clear();
                    a.sel = 1;
                }
                d => a.error = d.map(|d| s(&d, "error")).filter(|e| !e.is_empty()).unwrap_or_else(|| "Couldn't make a restore point.".into()),
            }
            return a.load();
        }
        Msg::ChangesRead(o) => {
            let d = o.json().unwrap_or(Value::Null);
            a.change_list = d.get("changes").and_then(Value::as_array).cloned().unwrap_or_default();
            a.added_list = d.get("added").and_then(Value::as_array).map(|x| x.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
        }
        Msg::Applied(o) => {
            let d = o.json().unwrap_or(Value::Null);
            if d.get("ok").and_then(Value::as_bool) != Some(true) {
                a.phase = Phase::Idle;
                let e = s(&d, "error");
                a.error = if e.is_empty() { "System Restore stopped with a problem. Nothing was changed.".into() } else { e };
                return a.load();
            }
            a.done_restart = d.get("restart").and_then(Value::as_bool) == Some(true);
            let changed = n(&d, "changed");
            a.done = if a.done_restart {
                format!("Restarting into Hearth {}…", d.get("plan").map(|p| s(p, "to")).unwrap_or_default())
            } else {
                format!(
                    "{} A copy of today's settings is kept as a restore point.",
                    if changed == 1 { "1 settings file went back.".to_string() } else { format!("{changed} settings files went back.") }
                )
            };
            a.phase = Phase::Done;
            a.dlg = 0;
            let mut tasks = vec![a.load()];
            if a.done_restart {
                tasks.push(Task::perform(run::after(1800), |_| Msg::Reboot));
            }
            return Task::batch(tasks);
        }
        Msg::Reboot => run::detached(&["systemctl", "reboot"]),
        Msg::Pick(i) => a.pick(i),
        Msg::ToggleOlder => a.show_older = !a.show_older,
        Msg::MakePoint => return a.make_point(),
        Msg::AskRestore => a.ask(),
        Msg::SeeChanges => return a.see_changes(),
        Msg::Dlg(b) => return a.dlg_press(b),
        Msg::Close => return win::close(),
        Msg::Drag => return win::drag(),
        Msg::Maximize => return win::toggle_maximize(),
        Msg::Minimize => return win::minimize(),
        Msg::Resize(d) => return win::resize(d),
        Msg::Key(ev) => return key(a, ev),
    }
    Task::none()
}

fn key(a: &mut App, ev: KEvent) -> Task<Msg> {
    let KEvent::KeyPressed { key, .. } = ev else { return Task::none() };
    let k = match key.as_ref() {
        Key::Named(n) => n,
        _ => return Task::none(),
    };
    let enter = matches!(k, Named::Enter | Named::Space);
    if matches!(a.phase, Phase::Confirm | Phase::Done | Phase::Changes) {
        let bs = a.dlg_buttons();
        match k {
            Named::Escape => {
                if a.phase != Phase::Done || !a.done_restart {
                    a.phase = Phase::Idle;
                }
            }
            Named::ArrowLeft | Named::ArrowUp => a.dlg = a.dlg.saturating_sub(1),
            Named::ArrowRight | Named::ArrowDown | Named::Tab => a.dlg = (a.dlg + 1).min(bs.len().saturating_sub(1)),
            _ if enter && !bs.is_empty() => return a.dlg_press(bs[a.dlg.min(bs.len() - 1)]),
            _ => {}
        }
        return Task::none();
    }
    if matches!(a.phase, Phase::Working | Phase::Loading) {
        return Task::none();
    }
    if k == Named::Escape {
        return win::close();
    }
    match a.zone {
        Zone::List => match k {
            Named::ArrowUp => a.pick(a.sel.saturating_sub(1)),
            Named::ArrowDown => {
                if a.sel + 1 < a.rows() {
                    a.pick(a.sel + 1)
                } else if a.has_older() {
                    a.zone = Zone::Older
                } else {
                    a.zone = Zone::Footer
                }
            }
            Named::ArrowRight | Named::Tab => a.zone = Zone::Aside,
            Named::Enter => {
                a.zone = Zone::Footer;
                a.foot = 1
            }
            _ => {}
        },
        Zone::Older => match k {
            Named::ArrowUp => a.zone = Zone::List,
            Named::ArrowDown | Named::Tab => a.zone = Zone::Footer,
            _ if enter => a.show_older = !a.show_older,
            _ => {}
        },
        Zone::Aside => match k {
            Named::ArrowLeft => a.zone = Zone::List,
            Named::ArrowDown | Named::Tab => a.zone = Zone::Footer,
            Named::Enter => return a.see_changes(),
            _ => {}
        },
        Zone::Footer => match k {
            Named::ArrowUp | Named::Tab => a.zone = Zone::List,
            Named::ArrowLeft => a.foot = a.foot.saturating_sub(1),
            Named::ArrowRight => a.foot = (a.foot + 1).min(2),
            Named::Enter => match a.foot {
                0 => return a.make_point(),
                1 => a.ask(),
                _ => return win::close(),
            },
            _ => {}
        },
    }
    Task::none()
}

// ------------------------------------------------------------------ the page
struct Change {
    t: String,
    s: String,
    g: G,
    bg: Color,
    fg: Color,
}

fn changes(a: &App) -> Vec<Change> {
    let p = &a.p;
    let kind = |k: &str| -> (Color, Color) {
        match k {
            "go" => (Pal::mix(p.panel, p.accent, 0.25), Pal::mix(p.accent, p.text, 0.35)),
            "ok" => (Pal::mix(p.panel, p.good, 0.25), p.good),
            _ => (Pal::mix(p.panel, p.warn, 0.22), p.warn),
        }
    };
    let ch = |t: String, s: String, g: G, k: &str| {
        let (bg, fg) = kind(k);
        Change { t, s, g, bg, fg }
    };
    let Some(cur) = a.cur() else {
        return vec![ch("This is where you are now".into(), "Pick an earlier point to see what would change.".into(), G::Check, "ok")];
    };
    let plan = cur.get("plan").cloned().unwrap_or(Value::Null);
    let running = a.running();
    let from = { let f = s(&plan, "from"); if f.is_empty() { running.clone() } else { f } };
    let mut out = Vec::new();
    let action = a.action();
    if a.version_goes_back() {
        out.push(ch(format!("Hearth {from} → {}", s(&plan, "to")), format!("The whole system goes back. {from} stays in the boot menu, so you can come back."), G::Arrow, "go"));
    } else if action == "none" {
        let v = if running.is_empty() { s(&plan, "to") } else { running.clone() };
        out.push(ch(format!("Hearth {v} stays"), "This point was made on the version you run now.".into(), G::Check, "ok"));
    } else {
        let why = s(&plan, "why");
        out.push(ch(format!("Hearth {running} stays"), if why.is_empty() { "Only the settings can go back.".into() } else { why }, G::Plus, "keep"));
    }
    let changed = n(&cur, "changed");
    if changed > 0 {
        out.push(ch(
            format!("Settings from {}", time::day(n(&cur, "created"))),
            format!("{changed} {} changed since then go back. A copy of today's is kept.", if changed == 1 { "setting" } else { "settings" }),
            G::Arrow,
            "go",
        ));
    } else {
        out.push(ch("Your settings are the same".into(), "Nothing changed since then.".into(), G::Check, "ok"));
    }
    out.push(ch("Apps you installed stay".into(), "Flatpaks, Windows apps and games aren't removed.".into(), G::Plus, "keep"));
    out.push(ch("Your files are not touched".into(), "Documents, pictures, games and saves stay exactly as they are.".into(), G::Check, "ok"));
    out
}

/// System Restore's blocks (hearth_ui::blocks).
const BLOCKS: &[hearth_ui::blocks::PageDef] = &[hearth_ui::blocks::PageDef {
    id: "main",
    title: "System Restore",
    blocks: &[
        hearth_ui::blocks::bl("header", "Heading", 0, 0, 12, 2),
        hearth_ui::blocks::bl("heads", "Column headings", 0, 2, 8, 1),
        hearth_ui::blocks::bl("points", "Restore points", 0, 3, 8, 7),
        hearth_ui::blocks::bl("older", "Show older restore points", 0, 10, 8, 1),
        hearth_ui::blocks::bl("changes", "What will change", 8, 2, 4, 7),
        hearth_ui::blocks::bl("see", "See every changed setting", 8, 9, 4, 2),
        hearth_ui::blocks::bl("make", "Make a restore point now", 0, 11, 4, 1),
        hearth_ui::blocks::bl("status", "What it will do", 4, 11, 4, 1),
        hearth_ui::blocks::bl("go", "Restore and Cancel", 8, 11, 4, 1),
    ],
}];

// ------------------------------------------------------------------ drag and drop (board 3: the same in every app)
use hearth_ui::studio::{tile, Kit};
/// System Restore's blocks window: its buttons, its panes, its info.
const KIT: Kit = Kit {
    name: "System Restore",
    buttons: &[
        tile("make", "Make a restore point now", G::Plus),
        tile("restore", "Restore", G::Restore),
        tile("see", "See every changed setting", G::List),
        tile("older", "Show older restore points", G::Clock),
        tile("newest", "The newest point", G::Clock),
        tile("oldest", "The oldest point", G::Clock),
    ],
    panes: &[tile("picked", "The point picked", G::Restore), tile("changes", "What will change", G::List), tile("points", "Every restore point", G::List)],
    info: &[tile("points", "Restore points", G::List), tile("picked", "The point picked", G::Restore), tile("changes", "Settings that change", G::List)],
    pane_page: "main",
    bars: &[],
    menus: &[],
};

/// One of System Restore's buttons dragged in, pressed.
fn block_action(a: &App, id: &str) -> Option<Msg> {
    Some(match id {
        "make" => Msg::MakePoint,
        "restore" => Msg::AskRestore,
        "see" => Msg::SeeChanges,
        "older" => Msg::ToggleOlder,
        "newest" => Msg::Pick(0),
        "oldest" => Msg::Pick(a.points().len().saturating_sub(1)),
        _ => return None,
    })
}

fn point_name(v: &Value) -> String {
    for k in ["title", "name", "label", "desc"] {
        let x = s(v, k);
        if !x.is_empty() {
            return x;
        }
    }
    s(v, "id")
}

fn studio_values(a: &App) {
    let pts = a.points();
    a.describe.values(vec![("points", pts.len().to_string()), ("picked", pts.get(a.sel).map(point_name).unwrap_or_default()), ("changes", a.change_list.len().to_string())]);
}

/// A pane of System Restore's own: the point picked, what will change, every point.
fn studio_pane<'a>(a: &'a App, which: &str) -> Element<'a, Msg> {
    let p = a.p;
    let pts = a.points();
    let mut c = Column::new().spacing(6.0);
    match which {
        "picked" => {
            if let Some(v) = pts.get(a.sel) {
                c = c.push(ts(point_name(v), 15.0, p.text));
                let when = n(v, "time");
                if when > 0 {
                    c = c.push(t(time::long(when), 12.0, p.muted));
                }
            }
        }
        "changes" => {
            for ch in a.change_list.iter().take(14) {
                c = c.push(t1(ui::fit(&format!("{} → {}", s(ch, "key"), s(ch, "to")), 44), 13.0, p.text));
            }
            if a.change_list.is_empty() {
                c = c.push(t("Pick a restore point to see what changes.", 13.0, p.muted));
            }
        }
        _ => {
            for (i, v) in pts.iter().enumerate().take(14) {
                c = c.push(mouse_area(t1(ui::fit(&point_name(v), 44), 13.0, if i == a.sel { p.accent } else { p.text })).on_press(Msg::Pick(i)).interaction(mouse::Interaction::Pointer));
            }
        }
    }
    scrollable(c).height(Length::Fill).into()
}

fn view(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let title = hearth_ui::describe::title_bar(&p, &a.describe, G::Restore, "System Restore", Some(hearth_ui::arrange::edit_button(&p, Msg::Describe(hearth_ui::arrange::edit_layout()))), TitleMsgs { drag: Msg::Drag, maximize: Msg::Maximize, minimize: None, close: Msg::Close }, Pal::mix(p.bg, Color::BLACK, 0.3), Msg::Describe(hearth_ui::describe::DMsg::Open));
    let header = container(
        column![
            ts("Restore Hearth to an earlier point", 18.0, p.text),
            t("Hearth and its settings go back to how they were. Your files, games and saves are never touched.", 13.0, p.muted),
        ]
        .spacing(3.0),
    )
    .padding(Padding { top: 16.0, right: 22.0, bottom: 16.0, left: 22.0 })
    .width(Length::Fill)
    .style(ui::boxed(p.bg2, 0.0, None));

    // ---- the list
    let heads = container(
        row![
            container(t1("Date and time", 12.0, p.muted)).width(Length::FillPortion(29)),
            container(t1("Description", 12.0, p.muted)).width(Length::FillPortion(49)),
            container(t1("Type", 12.0, p.muted)).width(Length::FillPortion(22)),
        ]
        .padding(Padding { top: 0.0, right: 10.0, bottom: 0.0, left: 10.0 }),
    )
    .center_y(Length::Fixed(28.0));
    let vis = a.visible();
    let running = a.running();
    let mut list = Column::new().spacing(2.0);
    for i in 0..a.rows() {
        let pt = if i >= 1 { vis.get(i - 1).cloned() } else { None };
        let selected = i == a.sel;
        let date = pt.as_ref().map(|x| time::when(n(x, "created"))).unwrap_or_else(|| "Now".into());
        let desc: Element<Msg> = match &pt {
            Some(x) => t1(ui::fit(&s(x, "title"), 48), 13.0, p.text).into(),
            None => row![t1(format!("Hearth {running}"), 13.0, p.text), chip("Running now", Pal::mix(p.panel, p.good, 0.3), p.good)].spacing(8.0).align_y(iced::Center).into(),
        };
        let kind = pt.as_ref().map(|x| s(x, "type")).unwrap_or_default();
        let line = row![
            container(t1(date, 13.0, p.text)).width(Length::FillPortion(29)).clip(true),
            container(desc).width(Length::FillPortion(49)).clip(true),
            container(t1(kind, 13.0, p.muted)).width(Length::FillPortion(22)).clip(true),
        ]
        .align_y(iced::Center)
        .padding(Padding { top: 0.0, right: 10.0, bottom: 0.0, left: 10.0 });
        let bg = if selected { p.sel() } else { Color::TRANSPARENT };
        let border = if selected { Some((if a.zone == Zone::List { p.accent } else { p.line }, 1.0)) } else { None };
        list = list.push(
            mouse_area(container(line).center_y(Length::Fixed(36.0)).width(Length::Fill).style(ui::boxed(bg, p.r(6.0), border)))
                .on_press(Msg::Pick(i))
                .interaction(mouse::Interaction::Pointer),
        );
    }
    let heads = column![heads, hline(p.line)];
    let points = scrollable(list).height(Length::Fill);
    let mut older = Column::new();
    if a.phase != Phase::Loading && a.points().is_empty() {
        older = older.push(t("No restore points yet. Hearth makes one before each update and once a week, or make one now.", 12.0, p.muted));
    }
    if a.has_older() {
        older = older.push(gap(1.0, 10.0));
        older = older.push(check(&p, a.show_older, "Show older restore points", a.zone == Zone::Older, Msg::ToggleOlder));
    }

    // ---- What will change
    let mut aside = Column::new().spacing(12.0).push(ts("What will change", 14.0, p.text));
    for c in changes(a) {
        let dot = container(glyph(c.g, c.fg, 12.0, 3.0)).center_x(Length::Fixed(22.0)).center_y(Length::Fixed(22.0)).style(ui::boxed(c.bg, 11.0, None));
        aside = aside.push(row![dot, column![tm(c.t, 13.0, p.text), t(c.s, 12.0, p.muted)].spacing(2.0).width(Length::Fill)].spacing(10.0));
    }
    let can_see = a.cur().map(|c| n(&c, "changed") > 0).unwrap_or(false);
    let see = container(btn(&p, "See every changed setting", Kind::Normal, a.zone == Zone::Aside, if can_see { Some(Msg::SeeChanges) } else { None })).center_x(Length::Fill);


    let make_label = if a.phase == Phase::Working { "Working…" } else { "Make a restore point now" };
    let make = btn(&p, make_label, Kind::Normal, a.zone == Zone::Footer && a.foot == 0, if a.phase == Phase::Idle { Some(Msg::MakePoint) } else { None });
    let status = container(t1(a.footer(), 12.0, p.muted)).align_right(Length::Fill).padding(Padding { top: 0.0, right: 8.0, bottom: 0.0, left: 0.0 });
    let go = row![
        btn(&p, a.go_label(), Kind::Primary, a.zone == Zone::Footer && a.foot == 1, if a.can_restore() { Some(Msg::AskRestore) } else { None }),
        btn(&p, "Cancel", Kind::Normal, a.zone == Zone::Footer && a.foot == 2, Some(Msg::Close)),
    ]
    .spacing(8.0)
    .align_y(iced::Center);

    // its blocks (hearth_ui::blocks): moved, sized, hidden or restyled; as it was when nothing is
    let blocks = hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "main",
        {
            studio_values(a);
            let mut items: Vec<(&'static str, Element<'_, Msg>)> = vec![
                ("header", header.into()),
                ("heads", heads.into()),
                ("points", points.into()),
                ("older", older.into()),
                ("changes", aside.into()),
                ("see", see.into()),
                ("make", make),
                ("status", status.into()),
                ("go", go.into()),
            ];
            for (id, which) in [("pane:picked", "picked"), ("pane:changes", "changes"), ("pane:points", "points")] {
                if a.describe.wants_pane(which) {
                    items.push((id, studio_pane(a, which)));
                }
            }
            items
        },
        |ps| {
            let left = column![ps.take("heads"), gap(1.0, 4.0), ps.take("points"), ps.take("older")].width(Length::Fill).height(Length::Fill);
            let aside = container(column![ps.take("changes"), iced::widget::Space::new().height(Length::Fill), ps.take("see")].spacing(12.0))
                .padding(16.0)
                .width(Length::Fixed(300.0))
                .height(Length::Fill)
                .style(ui::boxed(p.panel, p.r(10.0), Some((p.line, 1.0))));
            let footer = container(
                row![ps.take("make"), ps.take("status"), ps.take("go")].spacing(8.0).align_y(iced::Center).padding(Padding { top: 0.0, right: 16.0, bottom: 0.0, left: 16.0 }),
            )
            .center_y(Length::Fixed(58.0))
            .width(Length::Fill)
            .style(ui::boxed(p.bg2, 0.0, None));
            let body = row![left, aside].spacing(16.0).padding(Padding { top: 16.0, right: 22.0, bottom: 16.0, left: 22.0 }).height(Length::Fill);
            let mut page = column![ps.take("header"), hline(p.line), body];
            if !a.error.is_empty() {
                page = page.push(container(t(a.error.clone(), 13.0, p.danger)).padding(Padding { top: 0.0, right: 22.0, bottom: 8.0, left: 22.0 }));
            }
            page.push(hline(p.line)).push(footer).into()
        },
        Msg::Describe,
    );
    let page = column![title, blocks];
    let base: Element<Msg> = container(page).width(Length::Fill).height(Length::Fill).style(ui::boxed(p.bg, 0.0, None)).into();

    let base = if matches!(a.phase, Phase::Confirm | Phase::Changes | Phase::Working | Phase::Done) { ui::dialog(base, dialog(a), None) } else { base };
    ui::edges(base, Msg::Resize)
}

fn dialog(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let cur = a.cur();
    let heading = match a.phase {
        Phase::Confirm => format!("Restore to {}?", cur.as_ref().map(|c| time::when(n(c, "created"))).unwrap_or_default()),
        Phase::Changes => "Settings that go back".into(),
        Phase::Working => "Working…".into(),
        _ => if a.done_restart { "Restarting".into() } else { "Done".into() },
    };
    let mut col = Column::new().spacing(12.0).push(ts(heading, 17.0, p.text));
    match a.phase {
        Phase::Changes => {
            let mut l = Column::new().spacing(6.0);
            for c in &a.change_list {
                let key = s(c, "key");
                l = l.push(column![
                    t1(ui::fit(&format!("{}{}", s(c, "file"), if key.is_empty() { String::new() } else { format!(" › {key}") }), 80), 12.0, p.text),
                    t1(ui::fit(&format!("Now: {}   →   Then: {}", s(c, "now"), s(c, "then")), 90), 11.0, p.muted),
                ]);
            }
            if a.change_list.is_empty() {
                l = l.push(t("Reading what changed…", 12.0, p.muted));
            }
            col = col.push(scrollable(l).height(Length::Fixed(300.0)));
            if !a.added_list.is_empty() {
                col = col.push(t(format!("Made since then and kept as they are: {}", a.added_list.join(", ")), 12.0, p.muted));
            }
        }
        Phase::Confirm => {
            let mut msg = String::new();
            if let Some(c) = &cur {
                let plan = c.get("plan").cloned().unwrap_or(Value::Null);
                if a.version_goes_back() {
                    let from = { let f = s(&plan, "from"); if f.is_empty() { a.running() } else { f } };
                    msg += &format!("Hearth goes back to {} and restarts. {from} stays in the boot menu, so you can come back from Settings › Updates & recovery or the boot menu. ", s(&plan, "to"));
                }
                let ch = n(c, "changed");
                if ch > 0 {
                    msg += &format!("{ch} {}; a copy of today's settings is kept first as a restore point. ", if ch == 1 { "setting goes back" } else { "settings go back" });
                }
            }
            msg += "Apps you installed and your files stay.";
            col = col.push(t(msg, 13.0, p.muted));
        }
        Phase::Working => col = col.push(t("Keeping a copy of today's settings, then putting the restore point back. Don't turn the PC off.", 13.0, p.muted)),
        _ => col = col.push(t(a.done.clone(), 13.0, p.muted)),
    }
    let mut bs = row![].spacing(8.0);
    for (i, b) in a.dlg_buttons().into_iter().enumerate() {
        let label = match b {
            "go" => a.go_label(),
            "settings" => "Only the settings",
            "restart" => "Restart now",
            "ok" => "OK",
            _ => "Cancel",
        };
        let kind = if i == 0 && b != "ok" { Kind::Primary } else { Kind::Normal };
        bs = bs.push(btn(&p, label, kind, a.dlg == i, Some(Msg::Dlg(b))));
    }
    col = col.push(container(bs).align_right(Length::Fill));
    ui::dialog_box(&p, if a.phase == Phase::Changes { 640.0 } else { 520.0 }, col)
}

fn subscription(_a: &App) -> Subscription<Msg> {
    Subscription::batch([Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme), iced::keyboard::listen().map(Msg::Key)])
}

/// The app with Describe it's box under it when it's open (every Hearth app has it).
fn shown(a: &App) -> Element<'_, Msg> {
    hearth_ui::describe::with_bar(view(a), &a.p, &a.describe, Msg::Describe)
}

fn main() -> iced::Result {
    if app::handle_cli("hearth-system-restore", "System Restore: Hearth and its settings back to an earlier restore point.\n  --show confirm|changes   open with that dialog (pictures)\n  --older                  show older restore points") {
        return Ok(());
    }
    iced::application(App::new, update, shown)
        .title("System Restore")
        .subscription(subscription)
        .theme(|a: &App| app::iced_theme(&a.p))
        .window(app::window("hearth-restore", (960.0, 640.0), (720.0, 480.0)))
        .default_font(app::INTER)
        .run()
}
