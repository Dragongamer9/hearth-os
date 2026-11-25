//! hearth-ideas-app: Hearth Ideas (docs/mockups/checklists/hearth-ideas.md; boards ideas-1 to ideas-3, and the inbox,
//! desktop-layouts board 10). The ideas people sent from Describe it, the same idea from several people as one row,
//! and what the owner does with each: Later, No, put it on the roadmap, or Build it with his AI team (the Hearth AI
//! Command Center gets the brief: `hearth-ai-command-center --idea=<file>`), watch it being built (its branch, CI,
//! pictures and pull request), and Ship it with his own version number (never picked here).
//!
//! On the owner's devices only: everything is read and written with his own `gh` sign-in (the ideas are issues in
//! the private Dragongamer9/hearth-logs). Nothing is saved on the device but the brief file handed to the Command
//! Center.
//!
//! `--open N` opens idea #N; `--page built|ship` with it opens its Being built page or the Ship box.
//! $HEARTH_IDEAS_FIXTURE: a JSON file of gh answers (keyed by the API path) read instead of GitHub (CI's pictures);
//! nothing is written then.

mod data;

use data::{Ask, Build, Idea, State, Step, BASE, LOGS, REPO};
use hearth_ui::app::{self, win};
use hearth_ui::run;
use hearth_ui::ui::{self, gap, glyph, t, tb, tm, TitleMsgs, G};
use hearth_ui::Pal;
use iced::widget::{button, column, container, image, mouse_area, row, scrollable, text_editor, text_input, Column, Space};
use iced::{mouse, Background, Border, Color, Element, Length, Padding, Shadow, Subscription, Task, Theme};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Page {
    List,
    Idea,
    Built,
}

/// What a gh answer is for.
#[derive(Debug, Clone)]
enum K {
    List,
    Comments(u64),
    Branch(u64),
    Runs(u64),
    Pulls(u64),
    Files(u64),
    Reviews(u64),
    Shots(u64),
    ShotsAt(u64),
    Shot(u64, String),
    Release,
    Roadmap(u64),
    /// a write; then the list (and this idea) are read again
    Wrote(u64, &'static str),
}

#[derive(Debug, Clone)]
enum Msg {
    /// Describe it (hearth_ui::describe; every Hearth app has it)
    Describe(hearth_ui::describe::DMsg),
    Theme,
    Tick,
    Got(K, run::Output),
    Pick(State),
    ByApp(String),
    Open(u64),
    Back,
    Set(u64, State),
    Accept(u64),
    BuildIt(u64),
    OpenCc(u64),
    Stop(u64),
    ToggleChange,
    Lead(&'static str),
    Checks(&'static str),
    MockupFirst,
    ShowBuilt(u64),
    ShipOpen(u64),
    ShipHow(bool),
    Version(String),
    Notes(text_editor::Action),
    Tell,
    Matches,
    ShipNow,
    Publish,
    NotYet,
    OpenPr(u64),
    Sort,
    Drag,
    Max,
    Min,
    Close,
    Resize(iced::window::Direction),
    Nothing,
}

const AIS: &[(&str, &str)] = &[("claude", "Claude"), ("chatgpt", "ChatGPT"), ("gemini", "Gemini"), ("grok", "Grok"), ("codex", "Codex")];

fn ai_name(id: &str) -> &str {
    AIS.iter().find(|a| a.0 == id).map(|a| a.1).unwrap_or(id)
}

struct Ship {
    number: u64,
    quick: bool,
    version: String,
    notes: text_editor::Content,
    tell: bool,
    matches: bool,
    /// 0 not yet, 1 shipping, 2 shipped (Publish is the second press), 3 published
    stage: u8,
}

struct App {
    pub describe: hearth_ui::describe::Describe,
    p: Pal,
    ideas: Vec<Idea>,
    loading: bool,
    problem: String,
    page: Page,
    list: State,
    by_app: String,
    most_asked: bool,
    open: u64,
    asks: HashMap<u64, Vec<Ask>>,
    sent: HashMap<u64, (String, String, String, String)>,
    builds: HashMap<u64, Build>,
    shots: HashMap<u64, Vec<(String, String)>>,
    shot: HashMap<u64, (String, String, image::Handle)>,
    roadmap: Option<(u64, String)>,
    lead: &'static str,
    checks: &'static str,
    change: bool,
    mockup_first: bool,
    busy: String,
    said: String,
    ship: Option<Ship>,
    os: String,
    now: i64,
}

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn fixture() -> Option<Value> {
    let f = std::env::var("HEARTH_IDEAS_FIXTURE").ok()?;
    serde_json::from_str(&std::fs::read_to_string(f).ok()?).ok()
}

/// `gh api ARGS…` (the owner's own sign-in), or the fixture's answer for its path.
fn gh(args: Vec<String>, k: K) -> Task<Msg> {
    if let Some(fx) = fixture() {
        let path = args.iter().find(|a| !a.starts_with('-') && a.contains('/')).cloned().unwrap_or_default();
        let writes = args.iter().any(|a| a == "-X" || a == "--input" || a.starts_with("-f"));
        let out = if writes { json!({}) } else { fx.get(&path).cloned().unwrap_or(Value::Null) };
        let ok = !out.is_null();
        return Task::done(Msg::Got(k, run::Output { ok, code: if ok { 0 } else { 1 }, out: out.to_string(), err: if ok { String::new() } else { format!("{path}: not in the fixture") } }));
    }
    let mut c = vec!["gh".to_string(), "api".into()];
    c.extend(args);
    Task::perform(run::spawn(c), move |o| Msg::Got(k.clone(), o))
}
fn a(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

/// A folder only this person can read, for the brief and the bodies gh sends.
fn own_dir() -> std::path::PathBuf {
    let d = std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir).join("hearth-ideas");
    let _ = std::fs::create_dir_all(&d);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&d, std::fs::Permissions::from_mode(0o700));
    }
    d
}
fn write_file(name: &str, body: &str) -> Option<String> {
    let f = own_dir().join(name);
    std::fs::write(&f, body).ok()?;
    Some(f.to_string_lossy().to_string())
}

impl App {
    fn new() -> (App, Task<Msg>) {
        let open = app::arg("--open").and_then(|n| n.parse().ok()).unwrap_or(0);
        let page = match (open, app::arg("--page").as_deref()) {
            (0, _) => Page::List,
            (_, Some("built")) | (_, Some("ship")) => Page::Built,
            _ => Page::Idea,
        };
        let a = App {
            describe: hearth_ui::describe::Describe::new("ideas", "Hearth Ideas").with_pages(BLOCKS),
            p: hearth_ui::theme::load(),
            ideas: Vec::new(),
            loading: true,
            problem: String::new(),
            page,
            list: State::New,
            by_app: String::new(),
            most_asked: true,
            open,
            asks: HashMap::new(),
            sent: HashMap::new(),
            builds: HashMap::new(),
            shots: HashMap::new(),
            shot: HashMap::new(),
            roadmap: None,
            lead: "claude",
            checks: "chatgpt",
            change: false,
            mockup_first: false,
            busy: String::new(),
            said: String::new(),
            ship: None,
            os: String::new(),
            now: now(),
        };
        let mut t = vec![load_list(), gh(a_release(), K::Release)];
        if open > 0 {
            t.push(load_idea(open));
        }
        (a, Task::batch(t))
    }

    fn idea(&self, n: u64) -> Option<&Idea> {
        self.ideas.iter().find(|i| i.number == n)
    }

    /// The idea's lists: its state, or Ready once CI is green with a pull request open (shown, never written).
    fn state_of(&self, i: &Idea) -> State {
        if i.state == State::Building && self.builds.get(&i.number).map(Build::ready).unwrap_or(false) {
            State::Ready
        } else {
            i.state
        }
    }
    fn count(&self, s: State) -> usize {
        self.ideas.iter().filter(|i| self.state_of(i) == s).count()
    }
    fn shown(&self) -> Vec<&Idea> {
        let mut v: Vec<&Idea> = self
            .ideas
            .iter()
            .filter(|i| if self.by_app.is_empty() { self.state_of(i) == self.list } else { app_of(i) == self.by_app && !matches!(self.state_of(i), State::Shipped | State::No) })
            .collect();
        if !self.most_asked {
            v.sort_by(|a, b| b.updated.cmp(&a.updated));
        }
        v
    }
}

fn a_release() -> Vec<String> {
    a(&[&format!("repos/{REPO}/releases/latest")])
}
fn load_list() -> Task<Msg> {
    gh(a(&[&format!("repos/{LOGS}/issues?labels=idea&state=all&per_page=100")]), K::List)
}
fn load_idea(n: u64) -> Task<Msg> {
    gh(a(&[&format!("repos/{LOGS}/issues/{n}/comments?per_page=100")]), K::Comments(n))
}

/// Which app or part an idea is for, in words (the sidebar's By app).
fn app_of(i: &Idea) -> String {
    let l = hearth_layout::feature::part_label(&i.part);
    if l.ends_with("(Mantel)") { "Mantel".into() } else { l }
}

/// The issue's labels with this state's (the others' state labels taken off).
fn labels_with(i: &Idea, s: State) -> Vec<String> {
    let mut l: Vec<String> = i.labels.iter().filter(|x| !x.starts_with("state:")).cloned().collect();
    if !s.label().is_empty() {
        l.push(s.label().to_string());
    }
    l
}
fn set_state(i: &Idea, s: State, why: &'static str) -> Task<Msg> {
    let mut args = a(&["-X", "PATCH", &format!("repos/{LOGS}/issues/{}", i.number)]);
    for l in labels_with(i, s) {
        args.push("-f".into());
        args.push(format!("labels[]={l}"));
    }
    args.push("-f".into());
    args.push(format!("state={}", if matches!(s, State::Shipped | State::No) { "closed" } else { "open" }));
    gh(args, K::Wrote(i.number, why))
}
fn comment(n: u64, body: &str) -> Task<Msg> {
    gh(a(&["-X", "POST", &format!("repos/{LOGS}/issues/{n}/comments"), "-f", &format!("body={body}")]), K::Wrote(n, "comment"))
}

/// Everything about one being built, read again (each answer asks for the next).
fn load_build(n: u64, branch: &str) -> Task<Msg> {
    Task::batch([
        gh(a(&[&format!("repos/{REPO}/branches/{branch}")]), K::Branch(n)),
        gh(a(&[&format!("repos/{REPO}/pulls?head=Dragongamer9:{branch}&base={BASE}&state=all")]), K::Pulls(n)),
        gh(a(&[&format!("repos/{REPO}/contents/?ref={}", data::shots_branch(branch))]), K::Shots(n)),
        gh(a(&[&format!("repos/{REPO}/commits/{}", data::shots_branch(branch))]), K::ShotsAt(n)),
    ])
}

/// The Command Center gets the idea (electron/ideas.cjs in hearth-ai-command-center).
fn to_command_center(app: &App, n: u64) -> Result<String, String> {
    let i = app.idea(n).ok_or("That idea isn't in the list any more.")?;
    let asks = app.asks.get(&n).cloned().unwrap_or_default();
    let (lead, checks, branch) = match app.sent.get(&n) {
        Some((l, c, b, _)) if !b.is_empty() => (lower_ai(l), lower_ai(c), b.clone()),
        _ => (app.lead.to_string(), app.checks.to_string(), data::branch_for(&data::title_of(i))),
    };
    let brief = data::brief(i, &asks, &branch, &lead, &checks, app.mockup_first);
    let f = write_file(&format!("idea-{n}.json"), &brief.to_string()).ok_or("The brief couldn't be written.")?;
    if fixture().is_none() {
        std::process::Command::new("hearth-ai-command-center")
            .arg(format!("--idea={f}"))
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|_| "The Hearth AI Command Center isn't installed.".to_string())?;
    }
    Ok(branch)
}
fn lower_ai(name: &str) -> String {
    AIS.iter().find(|a| a.1 == name).map(|a| a.0.to_string()).unwrap_or_else(|| name.to_lowercase())
}

fn b64_decode(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0;
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => continue,
        } as u32;
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    out
}
fn b64_encode(b: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut o = String::with_capacity(b.len() * 4 / 3 + 4);
    for ch in b.chunks(3) {
        let n = (ch[0] as u32) << 16 | (*ch.get(1).unwrap_or(&0) as u32) << 8 | *ch.get(2).unwrap_or(&0) as u32;
        for k in 0..4 {
            if k <= ch.len() {
                o.push(T[(n >> (18 - 6 * k) & 63) as usize] as char);
            } else {
                o.push('=');
            }
        }
    }
    o
}

/// The roadmap with the idea's line under "### Ideas people asked for" in "## Later" (made there if it isn't yet).
fn roadmap_add(text: &str, line: &str) -> String {
    const H: &str = "### Ideas people asked for";
    if text.contains(line) {
        return text.to_string();
    }
    if let Some(at) = text.find(H) {
        let after = at + H.len();
        let nl = text[after..].find('\n').map(|x| after + x + 1).unwrap_or(text.len());
        return format!("{}{line}\n{}", &text[..nl], &text[nl..]);
    }
    if let Some(at) = text.find("\n## Later") {
        let start = at + 1;
        let nl = text[start..].find('\n').map(|x| start + x + 1).unwrap_or(text.len());
        return format!("{}\n{H}\n\n{line}\n{}", &text[..nl], &text[nl..]);
    }
    format!("{text}\n{H}\n\n{line}\n")
}

fn update(a: &mut App, m: Msg) -> Task<Msg> {
    a.now = now();
    match m {
        Msg::Describe(dm) => return a.describe.update(dm).map(Msg::Describe),
        Msg::Theme => a.p = hearth_ui::theme::load(),
        Msg::Tick => {
            // being built: read it again (once a minute, only while one is open)
            if a.page == Page::Built {
                if let Some((_, _, b, _)) = a.sent.get(&a.open).cloned() {
                    return load_build(a.open, &b);
                }
            }
        }
        Msg::Got(k, o) => return got(a, k, o),
        Msg::Pick(s) => {
            a.list = s;
            a.by_app.clear();
            a.page = Page::List;
        }
        Msg::ByApp(x) => {
            a.by_app = x;
            a.page = Page::List;
        }
        Msg::Sort => a.most_asked = !a.most_asked,
        Msg::Open(n) => {
            a.open = n;
            a.change = false;
            a.said.clear();
            let st = a.idea(n).map(|i| a.state_of(i)).unwrap_or(State::New);
            a.page = if matches!(st, State::Building | State::Ready) { Page::Built } else { Page::Idea };
            return load_idea(n);
        }
        Msg::Back => {
            a.page = Page::List;
            a.ship = None;
        }
        Msg::Set(n, s) => {
            if let Some(i) = a.idea(n).cloned() {
                a.busy = format!("{n}");
                return set_state(&i, s, s.name());
            }
        }
        Msg::Accept(n) => return roadmap(a, n),
        Msg::BuildIt(n) => {
            let Some(i) = a.idea(n).cloned() else { return Task::none() };
            match to_command_center(a, n) {
                Ok(branch) => {
                    a.said = format!("Sent to your AI team. {} leads, {} checks; it's made on {branch}.", ai_name(a.lead), ai_name(a.checks));
                    a.sent.insert(n, (ai_name(a.lead).into(), ai_name(a.checks).into(), branch.clone(), String::new()));
                    a.page = Page::Built;
                    let line = format!("Sent to your AI team: {} leads, {} checks · branch `{branch}`{}", ai_name(a.lead), ai_name(a.checks), if a.mockup_first { " · a mockup first" } else { "" });
                    return Task::batch([comment(n, &line), set_state(&i, State::Building, "Being built"), load_build(n, &branch)]);
                }
                Err(e) => a.said = e,
            }
        }
        Msg::OpenCc(n) => {
            if let Err(e) = to_command_center(a, n) {
                a.said = e;
            }
        }
        Msg::Stop(n) => {
            if let Some(i) = a.idea(n).cloned() {
                a.said = "Stopped: it's in Later, with its branch kept. Build it again any time.".into();
                a.page = Page::Idea;
                return Task::batch([comment(n, "Stopped from Hearth Ideas (its branch is kept)."), set_state(&i, State::Later, "Later")]);
            }
        }
        Msg::ToggleChange => a.change = !a.change,
        Msg::Lead(x) => {
            a.lead = x;
            if a.checks == x {
                a.checks = AIS.iter().map(|a| a.0).find(|y| *y != x).unwrap_or("chatgpt");
            }
        }
        Msg::Checks(x) => {
            if x != a.lead {
                a.checks = x;
            }
        }
        Msg::MockupFirst => a.mockup_first = !a.mockup_first,
        Msg::ShowBuilt(n) => {
            a.open = n;
            a.page = Page::Built;
        }
        Msg::ShipOpen(n) => {
            let i = a.idea(n).cloned();
            let files = a.builds.get(&n).and_then(|b| b.pr.as_ref()).map(|p| p.files.clone()).unwrap_or_default();
            let (apps, other) = data::updates(&files);
            let first = i.as_ref().map(|i| i.first.words.clone()).unwrap_or_default();
            let title = i.as_ref().map(data::title_of).unwrap_or_default();
            a.ship = Some(Ship { number: n, quick: !apps.is_empty() && !other, version: String::new(), notes: text_editor::Content::with_text(&format!("{title}: {first}")), tell: true, matches: false, stage: 0 });
        }
        Msg::ShipHow(q) => {
            if let Some(s) = a.ship.as_mut() {
                s.quick = q;
            }
        }
        Msg::Version(v) => {
            if let Some(s) = a.ship.as_mut() {
                s.version = v.chars().filter(|c| c.is_ascii_digit() || *c == '.').take(16).collect();
            }
        }
        Msg::Notes(act) => {
            if let Some(s) = a.ship.as_mut() {
                s.notes.perform(act);
            }
        }
        Msg::Tell => {
            if let Some(s) = a.ship.as_mut() {
                s.tell = !s.tell;
            }
        }
        Msg::Matches => {
            if let Some(s) = a.ship.as_mut() {
                s.matches = !s.matches;
            }
        }
        Msg::NotYet => a.ship = None,
        Msg::ShipNow => return ship(a),
        Msg::Publish => {
            if let Some(s) = a.ship.as_mut() {
                if s.stage == 2 {
                    s.stage = 3;
                    let n = s.number;
                    a.said = "Publishing the Store's list: devices see it within the hour.".into();
                    if fixture().is_some() {
                        return Task::none();
                    }
                    return Task::perform(run::spawn(a_owned(&["gh", "workflow", "run", "store.yml", "-R", REPO, "--ref", "main"])), move |o| Msg::Got(K::Wrote(n, "published"), o));
                }
            }
        }
        Msg::OpenPr(n) => {
            if let Some(p) = a.builds.get(&n).and_then(|b| b.pr.as_ref()) {
                run::detached(&["xdg-open", &format!("https://github.com/{REPO}/pull/{}", p.number)]);
            }
        }
        Msg::Drag => return win::drag(),
        Msg::Max => return win::toggle_maximize(),
        Msg::Min => return win::minimize(),
        Msg::Close => return win::close(),
        Msg::Resize(d) => return win::resize(d),
        Msg::Nothing => {}
    }
    Task::none()
}
fn a_owned(v: &[&str]) -> Vec<String> {
    a(v)
}

/// Accept / Add to the roadmap: the line goes into docs/ROADMAP.md on bughunt (read first, then written with its sha).
fn roadmap(a: &mut App, n: u64) -> Task<Msg> {
    a.busy = format!("{n}");
    a.said = "Adding it to the roadmap…".into();
    gh(a_owned(&[&format!("repos/{REPO}/contents/docs/ROADMAP.md?ref={BASE}")]), K::Roadmap(n))
}

/// Ship it: the pull request into bughunt, then the quick update(s) built with the owner's number (or it waits for the
/// next Hearth OS update), and the people who asked told. Publish is a second press.
fn ship(a: &mut App) -> Task<Msg> {
    let Some(s) = a.ship.as_mut() else { return Task::none() };
    if s.stage != 0 || !data::version_ok(&s.version) || !s.matches {
        return Task::none();
    }
    let n = s.number;
    let Some(i) = a.ideas.iter().find(|i| i.number == n).cloned() else { return Task::none() };
    let Some(b) = a.builds.get(&n).cloned() else { return Task::none() };
    let Some(pr) = b.pr.clone() else { return Task::none() };
    s.stage = 1;
    let ver = s.version.clone();
    let notes = s.notes.text().trim().replace('\n', " | ");
    let (apps, _) = data::updates(&pr.files);
    let os = if a.os.is_empty() { "1.17".to_string() } else { a.os.clone() };
    let quick = s.quick && !apps.is_empty();
    let tell = s.tell;
    let mut steps: Vec<Vec<String>> = Vec::new();
    if !pr.merged {
        steps.push(a_owned(&["gh", "api", "-X", "PUT", &format!("repos/{REPO}/pulls/{}/merge", pr.number), "-f", "merge_method=merge"]));
    }
    if quick {
        for app in &apps {
            steps.push(a_owned(&["gh", "workflow", "run", "quick-update.yml", "-R", REPO, "--ref", BASE, "-f", &format!("app={app}"), "-f", &format!("version={ver}"), "-f", &format!("os={os}"), "-f", &format!("notes={notes}")]));
        }
    }
    let how = if quick { format!("a quick update ({}) for Hearth OS {os}", data::app_words(&apps)) } else { "the next Hearth OS update".to_string() };
    let body = format!("Shipped as {ver} in {how}: {notes}{}", if tell { "\n\nTell who asked: yes (their Hearth Store shows it with the update)." } else { "" });
    steps.push(a_owned(&["gh", "api", "-X", "POST", &format!("repos/{LOGS}/issues/{n}/comments"), "-f", &format!("body={body}")]));
    let mut patch = a_owned(&["gh", "api", "-X", "PATCH", &format!("repos/{LOGS}/issues/{n}"), "-f", "state=closed"]);
    for l in labels_with(&i, State::Shipped) {
        patch.push("-f".into());
        patch.push(format!("labels[]={l}"));
    }
    steps.push(patch);
    a.said = format!("Shipping {ver}…");
    if fixture().is_some() {
        return Task::done(Msg::Got(K::Wrote(n, "shipped"), run::Output { ok: true, code: 0, out: String::new(), err: String::new() }));
    }
    // one after another; the first that fails stops the rest
    Task::perform(
        async move {
            for c in steps {
                let o = run::spawn(c).await;
                if !o.ok {
                    return o;
                }
            }
            run::Output { ok: true, code: 0, out: String::new(), err: String::new() }
        },
        move |o| Msg::Got(K::Wrote(n, "shipped"), o),
    )
}

fn got(a: &mut App, k: K, o: run::Output) -> Task<Msg> {
    let j = o.json().unwrap_or(Value::Null);
    match k {
        K::List => {
            a.loading = false;
            if !o.ok {
                a.problem = if o.err.contains("auth login") || o.err.contains("not logged") {
                    "Sign in to GitHub first: open a terminal and run gh auth login.".into()
                } else if o.err.contains("No such file") || o.err.contains("gh:") {
                    "Hearth Ideas uses GitHub's own command, gh. Install it from the Hearth Store's Developer tools.".into()
                } else {
                    format!("The ideas couldn't be read: {}", o.why())
                };
                return Task::none();
            }
            a.problem.clear();
            a.ideas = data::ideas(&j);
            // the ones being built: their progress, so the lists can say which are ready
            let mut t = Vec::new();
            for i in a.ideas.iter().filter(|i| i.state == State::Building) {
                if !a.asks.contains_key(&i.number) {
                    t.push(load_idea(i.number));
                }
            }
            return Task::batch(t);
        }
        K::Comments(n) => {
            if o.ok {
                a.asks.insert(n, data::asks(&j));
                if let Some(s) = data::sent(&j) {
                    let b = s.2.clone();
                    a.sent.insert(n, s);
                    return load_build(n, &b);
                }
            }
        }
        K::Branch(n) => {
            let b = a.builds.entry(n).or_default();
            if o.ok {
                b.sha = j.pointer("/commit/sha").and_then(Value::as_str).unwrap_or("").to_string();
                b.commit_at = j.pointer("/commit/commit/committer/date").and_then(Value::as_str).unwrap_or("").to_string();
                if !b.sha.is_empty() {
                    return gh(a_owned(&[&format!("repos/{REPO}/commits/{}/check-runs?per_page=100", b.sha)]), K::Runs(n));
                }
            }
        }
        K::Runs(n) => {
            if o.ok {
                a.builds.entry(n).or_default().runs = data::runs_of(&j);
            }
        }
        K::Pulls(n) => {
            if o.ok {
                let pr = data::pr_of(&j);
                let num = pr.as_ref().map(|p| p.number).unwrap_or(0);
                a.builds.entry(n).or_default().pr = pr;
                if num > 0 {
                    return Task::batch([
                        gh(a_owned(&[&format!("repos/{REPO}/pulls/{num}/files?per_page=100")]), K::Files(n)),
                        gh(a_owned(&[&format!("repos/{REPO}/pulls/{num}/reviews")]), K::Reviews(n)),
                    ]);
                }
            }
        }
        K::Files(n) => {
            if let (true, Some(p)) = (o.ok, a.builds.entry(n).or_default().pr.as_mut()) {
                p.files = j.as_array().map(|v| v.iter().filter_map(|f| f.get("filename").and_then(Value::as_str).map(str::to_string)).collect()).unwrap_or_default();
                // the Ship box, if it opened first: a quick update when its files allow one
                let (apps, other) = data::updates(&p.files);
                if let Some(s) = a.ship.as_mut().filter(|s| s.number == n && s.stage == 0) {
                    s.quick = !apps.is_empty() && !other;
                }
            }
        }
        K::Reviews(n) => {
            if let (true, Some(p)) = (o.ok, a.builds.entry(n).or_default().pr.as_mut()) {
                p.reviewed = j.as_array().map(|v| !v.is_empty()).unwrap_or(false);
            }
        }
        K::Shots(n) => {
            if o.ok {
                let mut v: Vec<(String, String)> = j
                    .as_array()
                    .map(|v| v.iter().filter_map(|f| f.get("name").and_then(Value::as_str)).filter(|x| x.ends_with(".jpg") || x.ends_with(".png")).map(|x| (x.to_string(), String::new())).collect())
                    .unwrap_or_default();
                v.sort();
                a.builds.entry(n).or_default().shots = v.clone();
                a.shots.insert(n, v.clone());
                // the newest-named picture first (CI names them in order)
                if let (Some((name, _)), Some((_, _, b, _))) = (v.last(), a.sent.get(&n)) {
                    if a.shot.get(&n).map(|s| &s.0 != name).unwrap_or(true) {
                        return gh(a_owned(&[&format!("repos/{REPO}/contents/{name}?ref={}", data::shots_branch(b))]), K::Shot(n, name.clone()));
                    }
                }
            }
        }
        K::ShotsAt(n) => {
            if o.ok {
                let at = j.pointer("/commit/committer/date").and_then(Value::as_str).unwrap_or("").to_string();
                if let Some(v) = a.shots.get_mut(&n) {
                    for s in v.iter_mut() {
                        s.1 = at.clone();
                    }
                }
                if let Some(s) = a.shot.get_mut(&n) {
                    s.1 = at;
                }
            }
        }
        K::Shot(n, name) => {
            if o.ok {
                let bytes = b64_decode(j.get("content").and_then(Value::as_str).unwrap_or(""));
                if !bytes.is_empty() {
                    let at = a.shots.get(&n).and_then(|v| v.first()).map(|s| s.1.clone()).unwrap_or_default();
                    a.shot.insert(n, (name, at, image::Handle::from_bytes(bytes)));
                }
            }
        }
        K::Release => {
            if o.ok {
                a.os = data::os_of(j.get("tag_name").and_then(Value::as_str).unwrap_or(""));
            }
        }
        K::Roadmap(n) => {
            a.busy.clear();
            let Some(i) = a.idea(n).cloned() else { return Task::none() };
            if !o.ok {
                a.said = format!("The roadmap couldn't be read: {}", o.why());
                return Task::none();
            }
            let text = String::from_utf8_lossy(&b64_decode(j.get("content").and_then(Value::as_str).unwrap_or(""))).to_string();
            let sha = j.get("sha").and_then(Value::as_str).unwrap_or("").to_string();
            if text.is_empty() || sha.is_empty() {
                a.said = "The roadmap couldn't be read.".into();
                return Task::none();
            }
            let line = data::roadmap_line(&i, a.asks.get(&n).map(Vec::as_slice).unwrap_or(&[]));
            let new = roadmap_add(&text, &line);
            a.roadmap = Some((n, line));
            if new == text {
                a.said = "It's on the roadmap already.".into();
                return Task::none();
            }
            let body = json!({
                "message": format!("Roadmap: {} (an idea people asked for, {LOGS}#{n})", data::title_of(&i)),
                "content": b64_encode(new.as_bytes()),
                "sha": sha,
                "branch": BASE,
                "committer": {"name": "Dragongamer9", "email": "Dragongamer9@users.noreply.github.com"},
                "author": {"name": "Dragongamer9", "email": "Dragongamer9@users.noreply.github.com"},
            });
            let Some(f) = write_file(&format!("roadmap-{n}.json"), &body.to_string()) else {
                a.said = "The roadmap change couldn't be written.".into();
                return Task::none();
            };
            return Task::batch([gh(a_owned(&["-X", "PUT", &format!("repos/{REPO}/contents/docs/ROADMAP.md"), "--input", &f]), K::Wrote(n, "On the roadmap")), set_state(&i, State::Later, "Later")]);
        }
        K::Wrote(n, why) => {
            a.busy.clear();
            if !o.ok {
                a.said = format!("That didn't work: {}", o.why());
                if why == "shipped" {
                    if let Some(s) = a.ship.as_mut() {
                        s.stage = 0;
                    }
                }
                return Task::none();
            }
            match why {
                "shipped" => {
                    if let Some(s) = a.ship.as_mut() {
                        s.stage = 2;
                    }
                    a.said = "Shipped into bughunt and the quick update is being built (Actions › Quick update). Press Publish when you're ready.".into();
                }
                "published" => a.said = "The Store's list is being published.".into(),
                "On the roadmap" => a.said = "Added to docs/ROADMAP.md on bughunt, under Later › Ideas people asked for.".into(),
                "comment" => {}
                s => {
                    if a.said.is_empty() || !a.said.starts_with("Sent") {
                        a.said = format!("Moved to {s}.");
                    }
                }
            }
            return Task::batch([load_list(), load_idea(n)]);
        }
    }
    Task::none()
}

// ---------------------------------------------------------------- the window

fn section<'a>(p: &Pal, s: impl Into<String>) -> Element<'a, Msg> {
    tb(s.into().to_uppercase(), 11.0, p.muted).into()
}

fn btn<'a>(p: &Pal, label: impl Into<String>, kind: ui::Kind, on: Option<Msg>) -> Element<'a, Msg> {
    ui::btn(p, label.into(), kind, false, on)
}

fn sidebar<'a>(a: &'a App) -> Element<'a, Msg> {
    let p = a.p;
    let item = |s: State| -> Element<'a, Msg> {
        let on = a.by_app.is_empty() && a.list == s && a.page == Page::List || (a.page != Page::List && a.idea(a.open).map(|i| a.state_of(i) == s).unwrap_or(false) && a.page == Page::Built && s != State::New);
        let n = a.count(s);
        let badge: Element<'a, Msg> = if s == State::New && n > 0 {
            container(tb(n.to_string(), 11.0, p.ink)).padding(Padding { top: 1.0, right: 7.0, bottom: 1.0, left: 7.0 }).style(ui::boxed(p.accent, 9.0, None)).into()
        } else {
            t(n.to_string(), 13.0, p.muted).into()
        };
        mouse_area(
            container(row![tm(s.name(), 14.0, p.text), Space::new().width(Length::Fill), badge].align_y(iced::Center))
                .padding(Padding { top: 9.0, right: 12.0, bottom: 9.0, left: 12.0 })
                .width(Length::Fill)
                .style(ui::boxed(if on { p.hover() } else { Color::TRANSPARENT }, p.r(6.0), None)),
        )
        .on_press(Msg::Pick(s))
        .interaction(mouse::Interaction::Pointer)
        .into()
    };
    let mut c = Column::new().spacing(2.0);
    for s in State::ALL {
        c = c.push(item(s));
    }
    // By app (board 10)
    let mut apps: Vec<(String, usize)> = Vec::new();
    for i in a.ideas.iter().filter(|i| !matches!(a.state_of(i), State::Shipped | State::No)) {
        let x = app_of(i);
        match apps.iter_mut().find(|y| y.0 == x) {
            Some(y) => y.1 += 1,
            None => apps.push((x, 1)),
        }
    }
    if !apps.is_empty() {
        c = c.push(gap(1.0, 14.0)).push(container(section(&p, "By app")).padding(Padding { top: 0.0, right: 0.0, bottom: 6.0, left: 12.0 }));
        for (x, n) in apps {
            let on = a.by_app == x && a.page == Page::List;
            c = c.push(
                mouse_area(container(tm(format!("{x} · {n}"), 14.0, p.text)).padding(Padding { top: 8.0, right: 12.0, bottom: 8.0, left: 12.0 }).width(Length::Fill).style(ui::boxed(if on { p.hover() } else { Color::TRANSPARENT }, p.r(6.0), None)))
                    .on_press(Msg::ByApp(x))
                    .interaction(mouse::Interaction::Pointer),
            );
        }
    }
    // Edit layout, always findable (the owner, Nov 24): the page in front, edited in place
    c = c.push(gap(1.0, 14.0)).push(
        mouse_area(
            container(row![glyph(G::Pencil, p.muted, 14.0, 1.8), tm("Edit layout", 14.0, p.muted)].spacing(8.0).align_y(iced::Center))
                .padding(Padding { top: 9.0, right: 12.0, bottom: 9.0, left: 12.0 })
                .width(Length::Fill),
        )
        .on_press(Msg::Describe(hearth_ui::arrange::edit_layout()))
        .interaction(mouse::Interaction::Pointer),
    );
    let foot = t("On your devices only. Uses your own GitHub sign-in and your AI team (the Command Center). Nothing goes out without your version number.", 12.0, p.muted);
    container(column![c, Space::new().height(Length::Fill), container(foot).padding(Padding { top: 0.0, right: 12.0, bottom: 8.0, left: 12.0 })])
        .width(Length::Fixed(258.0))
        .height(Length::Fill)
        .padding(Padding { top: 16.0, right: 12.0, bottom: 14.0, left: 12.0 })
        .style(ui::boxed(Pal::mix(p.bg, Color::BLACK, 0.18), 0.0, None))
        .into()
}

fn kind_chip<'a>(p: &Pal, kind: &'static str) -> Element<'a, Msg> {
    let (fg, line) = match kind {
        "Needs something new" => (p.warn, p.warn),
        "Has a sketch" => (p.text, p.line),
        _ => (p.good, p.good),
    };
    container(t(kind, 12.0, fg).wrapping(iced::widget::text::Wrapping::None)).padding(Padding { top: 2.0, right: 10.0, bottom: 2.0, left: 10.0 }).style(ui::boxed(Pal::a(line, 0.08), 11.0, Some((Pal::a(line, 0.7), 1.0)))).into()
}

fn list_page(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let title = if a.by_app.is_empty() {
        match a.list {
            State::New => "New ideas".to_string(),
            s => s.name().to_string(),
        }
    } else {
        a.by_app.clone()
    };
    let sort = btn(&p, if a.most_asked { "Most asked ▾" } else { "Newest ▾" }, ui::Kind::Normal, Some(Msg::Sort));
    let head = row![tb(title, 30.0, p.text), Space::new().width(Length::Fill), sort].align_y(iced::Center);
    let mut c = Column::new().spacing(14.0);
    if a.loading {
        c = c.push(t("Reading the ideas…", 14.0, p.muted));
    } else if !a.problem.is_empty() {
        c = c.push(ui::card(&p, t(a.problem.clone(), 14.0, p.text)).padding(18.0));
    }
    let shown = a.shown();
    if !a.loading && a.problem.is_empty() && shown.is_empty() {
        c = c.push(t("Nothing here.", 14.0, p.muted));
    }
    for (k, i) in shown.into_iter().enumerate() {
        let n = i.number;
        let asks = a.asks.get(&n).map(Vec::as_slice).unwrap_or(&[]);
        let kind = data::kind_of(i, asks);
        let st = a.state_of(i);
        let mut line = vec![app_of_long(i)];
        line.push(match kind {
            "Needs something new" => "needs something new".to_string(),
            "Has a sketch" => "has a sketch".to_string(),
            _ => if i.closest.is_empty() { "a feature people made from Hearth's parts · ready to add as it is".to_string() } else { format!("{} · made from Hearth's parts · ready to add as it is", i.closest) },
        });
        let d = data::day(&i.created);
        if !d.is_empty() && k == 0 {
            line.push(format!("first asked {d}"));
        }
        let mut info = column![row![
            mouse_area(tb(data::title_of(i), 17.0, p.text)).on_press(Msg::Open(n)).interaction(mouse::Interaction::Pointer),
            if kind == "Needs something new" { Element::from(gap(1.0, 1.0)) } else { kind_chip(&p, kind) }
        ]
        .spacing(10.0)
        .align_y(iced::Center), t(line.join(" · "), 13.0, p.muted)]
        .spacing(3.0);
        if k == 0 && !i.first.words.is_empty() {
            let more = i.count.saturating_sub(1);
            info = info.push(t(format!("\"{}\"{}", i.first.words, if more > 0 { format!(" · and {more} more like it") } else { String::new() }), 13.0, p.text));
        }
        let mut btns = row![].spacing(8.0).align_y(iced::Center);
        match st {
            State::New | State::Later => {
                if kind != "Needs something new" {
                    btns = btns.push(btn(&p, "See it", ui::Kind::Normal, Some(Msg::Open(n))));
                }
                btns = btns.push(btn(&p, "Accept", ui::Kind::Primary, (a.busy.is_empty()).then_some(Msg::Accept(n))));
                if st != State::Later {
                    btns = btns.push(btn(&p, "Later", ui::Kind::Normal, Some(Msg::Set(n, State::Later))));
                }
                btns = btns.push(btn(&p, "No", ui::Kind::Normal, Some(Msg::Set(n, State::No))));
            }
            State::Building => btns = btns.push(btn(&p, "See how it's going", ui::Kind::Normal, Some(Msg::ShowBuilt(n)))),
            State::Ready => btns = btns.push(btn(&p, "Ship it…", ui::Kind::Primary, Some(Msg::ShowBuilt(n)))),
            State::Shipped | State::No => btns = btns.push(btn(&p, "See it", ui::Kind::Normal, Some(Msg::Open(n)))),
        }
        let count = column![tb(i.count.to_string(), 28.0, if k == 0 { p.accent } else { p.text }), t("asked", 12.0, p.muted)].align_x(iced::Center).width(Length::Fixed(64.0));
        let first = k == 0;
        c = c.push(
            container(row![count, info.width(Length::Fill), btns].spacing(18.0).align_y(iced::Center))
                .padding(Padding { top: 18.0, right: 20.0, bottom: 18.0, left: 14.0 })
                .width(Length::Fill)
                .style(ui::boxed(p.panel, p.r(10.0), Some((if first { Pal::a(p.accent, 0.8) } else { p.line }, 1.0)))),
        );
    }
    let mut note = Column::new().spacing(14.0);
    let mut has_note = false;
    if a.list == State::New && a.by_app.is_empty() {
        note = note.push(t("Accepting doesn't change anything for anyone by itself: it goes on the roadmap, and comes in an update with your version number.", 13.0, p.muted));
        has_note = true;
    }
    if !a.said.is_empty() {
        note = note.push(t(a.said.clone(), 13.0, p.text));
        has_note = true;
    }
    // its blocks (hearth_ui::blocks "list"): as it was when nothing is customised
    hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "list",
        vec![("title", head.into()), ("ideas", c.into()), ("note", note.into())],
        move |ps| {
            let mut all = Column::new().spacing(14.0).push(ps.take("title")).push(ps.take("ideas"));
            if has_note {
                all = all.push(ps.take("note"));
            }
            scrollable(all.padding(Padding { top: 0.0, right: 12.0, bottom: 0.0, left: 0.0 })).height(Length::Fill).into()
        },
        Msg::Describe,
    )
}

fn app_of_long(i: &Idea) -> String {
    hearth_layout::feature::part_label(&i.part)
}

fn quote<'a>(p: &Pal, words: &str) -> Element<'a, Msg> {
    container(t(format!("\"{words}\""), 15.0, p.text)).padding(Padding { top: 13.0, right: 16.0, bottom: 13.0, left: 16.0 }).width(Length::Fill).style(ui::boxed(p.panel, p.r(8.0), Some((p.line, 1.0)))).into()
}

fn pill<'a>(p: &Pal, s: impl Into<String>) -> Element<'a, Msg> {
    container(tm(s.into(), 13.0, p.text).wrapping(iced::widget::text::Wrapping::None)).padding(Padding { top: 4.0, right: 11.0, bottom: 4.0, left: 11.0 }).style(ui::boxed(Color::TRANSPARENT, 13.0, Some((p.line, 1.0)))).into()
}

fn idea_page(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let Some(i) = a.idea(a.open) else {
        return column![back(a, State::New), t(if a.loading { "Reading the ideas…" } else { "That idea isn't in the list." }, 14.0, p.muted)].spacing(14.0).into();
    };
    let n = i.number;
    let asks = a.asks.get(&n).cloned().unwrap_or_default();
    let kind = data::kind_of(i, &asks);
    let st = a.state_of(i);
    let mut head = row![tb(data::title_of(i), 34.0, p.text), kind_chip(&p, kind)].spacing(16.0).align_y(iced::Center);
    head = head.push(Space::new().width(Length::Fill));
    if matches!(st, State::New | State::Later | State::No) {
        if st != State::Later {
            head = head.push(btn(&p, "Later", ui::Kind::Normal, Some(Msg::Set(n, State::Later))));
        }
        if st != State::No {
            head = head.push(btn(&p, "No", ui::Kind::Normal, Some(Msg::Set(n, State::No))));
        }
    }
    let newest = asks.last().map(|x| x.at.clone()).unwrap_or_else(|| i.updated.clone());
    let meta = format!("{} · {} {} asked · first {} · newest {}", app_of_long(i), i.count, if i.count == 1 { "person" } else { "people" }, data::day(&i.created), data::ago(&newest, a.now));

    // what people asked: the same idea, in their words
    let mut left = Column::new().spacing(10.0).push(section(&p, "What people asked · the same idea, in their words"));
    let all: Vec<&Ask> = std::iter::once(&i.first).chain(asks.iter()).collect();
    let mut seen: Vec<String> = Vec::new();
    for x in all.iter() {
        let key = x.words.to_lowercase();
        if seen.contains(&key) || seen.len() >= 3 {
            continue;
        }
        seen.push(key);
        left = left.push(quote(&p, &x.words));
    }
    let more = (i.count as usize).saturating_sub(seen.len());
    let sketches = all.iter().filter(|x| x.sketch).count();
    if more > 0 || sketches > 0 {
        let mut s = Vec::new();
        if more > 0 {
            s.push(format!("and {more} more"));
        }
        if sketches > 0 {
            s.push(format!("{sketches} with a sketch"));
        }
        left = left.push(t(s.join(" · "), 13.0, p.muted));
    }
    if !i.closest.is_empty() {
        let kept = all.iter().filter(|x| x.kept).count();
        left = left.push(gap(1.0, 8.0)).push(section(&p, "What Describe it made instead"));
        let mark = container(container(Space::new()).width(Length::Fixed(10.0)).height(Length::Fixed(10.0)).style(ui::boxed(Color::BLACK, 5.0, None))).center_x(Length::Fixed(38.0)).center_y(Length::Fixed(38.0)).style(ui::boxed(p.accent, p.r(8.0), None));
        left = left.push(
            container(row![mark, column![tm(i.closest.clone(), 14.0, p.text), t(format!("Kept by {kept} of them"), 12.0, p.muted)].spacing(1.0)].spacing(12.0).align_y(iced::Center))
                .padding(14.0)
                .width(Length::Fill)
                .style(ui::boxed(p.panel, p.r(8.0), Some((p.line, 1.0)))),
        );
    }

    // build it
    let branch = data::branch_for(&data::title_of(i));
    let label = |s: &'static str| t(s, 13.0, p.muted);
    let mut card = column![
        tb("Build it with your AI team", 18.0, p.text),
        t("The Command Center gets the idea, what people asked, the sketches and the closest feature, and makes it on its own branch in hearth-os. CI builds it and takes its pictures; you see them here before anything ships.", 14.0, p.text),
        row![label("Repo"), pill(&p, REPO), gap(10.0, 1.0), label("Branch"), pill(&p, branch)].spacing(10.0).align_y(iced::Center),
        row![label("Lead"), pill(&p, ai_name(a.lead)), gap(10.0, 1.0), label("Checks"), pill(&p, ai_name(a.checks)), Space::new().width(Length::Fill), mouse_area(tm("Change ▾", 13.0, p.muted)).on_press(Msg::ToggleChange).interaction(mouse::Interaction::Pointer)].spacing(10.0).align_y(iced::Center),
    ]
    .spacing(14.0);
    if a.change {
        let mut r1 = row![label("Lead:")].spacing(6.0).align_y(iced::Center);
        let mut r2 = row![label("Checks:")].spacing(6.0).align_y(iced::Center);
        for (id, name) in AIS {
            r1 = r1.push(ui::btn(&p, *name, if a.lead == *id { ui::Kind::Primary } else { ui::Kind::Normal }, false, Some(Msg::Lead(id))));
            r2 = r2.push(ui::btn(&p, *name, if a.checks == *id { ui::Kind::Primary } else { ui::Kind::Normal }, false, (a.lead != *id).then_some(Msg::Checks(id))));
        }
        card = card.push(column![r1, r2].spacing(6.0));
    }
    let can_build = matches!(st, State::New | State::Later | State::No);
    card = card.push(
        row![
            ui::btn(&p, if a.mockup_first { "✓ Draw a mockup first" } else { "Draw a mockup first" }, ui::Kind::Normal, false, Some(Msg::MockupFirst)),
            Space::new().width(Length::Fill),
            btn(&p, "Build it", ui::Kind::Primary, can_build.then_some(Msg::BuildIt(n)))
        ]
        .align_y(iced::Center),
    );
    let build_card = container(card).padding(22.0).width(Length::Fill).style(ui::boxed(p.panel, p.r(10.0), Some((p.accent, 1.0))));
    let on_map = a.roadmap.as_ref().map(|r| r.0 == n).unwrap_or(false);
    let road = container(
        column![
            tb("Or put it on the roadmap", 17.0, p.text),
            t("Adds it to docs/ROADMAP.md with the people's words, to build later.", 13.0, p.muted),
            row![Space::new().width(Length::Fill), btn(&p, if on_map { "On the roadmap" } else { "Add to the roadmap" }, ui::Kind::Normal, (!on_map && a.busy.is_empty()).then_some(Msg::Accept(n)))]
        ]
        .spacing(10.0),
    )
    .padding(22.0)
    .width(Length::Fill)
    .style(ui::boxed(p.panel, p.r(10.0), Some((p.line, 1.0))));
    let right = column![section(&p, "Build it"), build_card, road].spacing(12.0);

    let said = !a.said.is_empty();
    hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "idea",
        vec![
            ("back", back(a, st)),
            ("head", head.into()),
            ("meta", t(meta, 14.0, p.muted).into()),
            ("about", left.width(Length::FillPortion(57)).into()),
            ("build", right.width(Length::FillPortion(43)).into()),
            ("said", t(a.said.clone(), 13.0, p.text).into()),
        ],
        move |ps| {
            let mut c = column![ps.take("back"), ps.take("head"), ps.take("meta"), gap(1.0, 4.0), row![ps.take("about"), ps.take("build")].spacing(22.0)].spacing(8.0);
            if said {
                c = c.push(gap(1.0, 6.0)).push(ps.take("said"));
            }
            scrollable(c.padding(Padding { top: 0.0, right: 12.0, bottom: 0.0, left: 0.0 })).height(Length::Fill).into()
        },
        Msg::Describe,
    )
}

fn back(a: &App, st: State) -> Element<'_, Msg> {
    let to = if a.by_app.is_empty() { st.back_to().to_string() } else { a.by_app.clone() };
    mouse_area(tm(format!("‹ {to}"), 14.0, a.p.muted)).on_press(Msg::Back).interaction(mouse::Interaction::Pointer).into()
}

fn step_icon<'a>(p: &Pal, s: Step) -> Element<'a, Msg> {
    let (bg, inner): (Color, Element<'a, Msg>) = match s {
        Step::Done => (Pal::mix(p.good, Color::BLACK, 0.25), glyph(G::Check, Color::WHITE, 12.0, 2.4)),
        Step::Running => (p.accent, tb("…", 12.0, Color::WHITE).into()),
        Step::Failed => (p.danger, glyph(G::Close, Color::WHITE, 9.0, 2.4)),
        Step::Waiting => (Color::TRANSPARENT, t("·", 12.0, p.muted).into()),
    };
    container(inner).center_x(Length::Fixed(26.0)).center_y(Length::Fixed(26.0)).style(ui::boxed(bg, 13.0, if s == Step::Waiting { Some((p.line, 1.0)) } else { None })).into()
}

fn built_page(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let Some(i) = a.idea(a.open) else {
        return t(if a.loading { "Reading the ideas…" } else { "That idea isn't in the list." }, 14.0, p.muted).into();
    };
    let n = i.number;
    let b = a.builds.get(&n).cloned().unwrap_or_default();
    let (lead, checks, branch, at) = a.sent.get(&n).cloned().unwrap_or_default();
    let ready = b.ready();
    let short = branch.trim_start_matches("idea/").replace('-', " ");
    let chip = container(t(if ready { "Ready to ship" } else { "Building" }, 12.0, if ready { p.good } else { p.accent }))
        .padding(Padding { top: 2.0, right: 10.0, bottom: 2.0, left: 10.0 })
        .style(ui::boxed(Color::TRANSPARENT, 11.0, Some((if ready { p.good } else { p.accent }, 1.0))));
    let mut head = row![tb(format!("Being built: {}", if short.is_empty() { data::title_of(i) } else { short }), 30.0, p.text), chip, Space::new().width(Length::Fill), btn(&p, "Open in the Command Center", ui::Kind::Normal, Some(Msg::OpenCc(n)))].spacing(14.0).align_y(iced::Center);
    if ready {
        head = head.push(btn(&p, "Ship it…", ui::Kind::Primary, Some(Msg::ShipOpen(n))));
    }
    head = head.push(btn(&p, "Stop", ui::Kind::Normal, Some(Msg::Stop(n))));

    let sent_line = format!("{} leads, {} checks{}", if lead.is_empty() { ai_name(a.lead) } else { lead.as_str() }, if checks.is_empty() { ai_name(a.checks) } else { checks.as_str() }, if at.is_empty() { String::new() } else { format!(" · {}", data::clock(&at)) });
    let pr_line = match &b.pr {
        Some(pr) => format!("#{} to {BASE}{}", pr.number, if pr.reviewed { ", looked at" } else { ", for you to look at" }),
        None => format!("to {BASE}, for you to look at"),
    };
    let steps: Vec<(Step, &str, String)> = vec![
        (Step::Done, "Idea sent to your AI team", sent_line),
        (if b.sha.is_empty() { Step::Running } else { Step::Done }, "Branch made", format!("{branch} from {BASE}")),
        (b.ci(), "Built and tested in GitHub Actions", b.ci_line()),
        (b.shots_step(), "Pictures being taken", if b.shots.is_empty() { "next to the mockup and Mantel as it is now".to_string() } else { format!("{} so far, next to the mockup and as it is now", b.shots.len()) }),
        (b.pr_step(), "Pull request", pr_line),
        (if ready { Step::Done } else { Step::Waiting }, "Ready to ship", "you pick the version number".to_string()),
    ];
    let mut sc = Column::new().spacing(26.0);
    for (s, title, sub) in steps {
        sc = sc.push(row![step_icon(&p, s), column![tm(title, 15.0, p.text), t(sub, 12.0, p.muted)].spacing(1.0)].spacing(14.0).align_y(iced::Center));
    }
    let left = container(sc).padding(22.0).width(Length::FillPortion(36)).height(Length::Fill).style(ui::boxed(p.panel, p.r(10.0), Some((p.line, 1.0))));

    let pic: Element<'_, Msg> = match a.shot.get(&n) {
        Some((name, when, h)) => column![
            container(image(h.clone()).content_fit(iced::ContentFit::Contain)).width(Length::Fill).height(Length::Fill).center_x(Length::Fill).center_y(Length::Fill),
            t(format!("{name} · CI{}", if when.is_empty() { String::new() } else { format!(", {}", data::clock(when)) }), 12.0, p.muted)
        ]
        .spacing(8.0)
        .into(),
        None => container(t(if b.sha.is_empty() { "Its pictures show here once CI has taken them." } else { "No pictures yet: CI takes them once it's built." }, 13.0, p.muted)).center_x(Length::Fill).center_y(Length::Fill).into(),
    };
    let pics = container(pic).padding(18.0).width(Length::Fill).height(Length::Fill).style(ui::boxed(Pal::mix(p.bg, Color::BLACK, 0.25), p.r(10.0), Some((p.line, 1.0))));
    let says = b.says();
    let mut right = column![section(&p, "Its pictures so far"), pics].spacing(10.0).width(Length::FillPortion(64));
    if !says.is_empty() {
        right = right.push(t(format!("The AI team says: \"{says}\""), 13.0, p.muted));
    }
    let said = !a.said.is_empty();
    hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "built",
        vec![("back", back(a, a.state_of(i))), ("head", head.into()), ("steps", left.into()), ("pictures", right.into()), ("said", t(a.said.clone(), 13.0, p.text).into())],
        move |ps| {
            let mut c = column![ps.take("back"), ps.take("head"), gap(1.0, 6.0), row![ps.take("steps"), ps.take("pictures")].spacing(20.0).height(Length::Fill)].spacing(10.0);
            if said {
                c = c.push(ps.take("said"));
            }
            c.height(Length::Fill).into()
        },
        Msg::Describe,
    )
}

fn ship_box<'a>(a: &'a App) -> Option<Element<'a, Msg>> {
    let p = a.p;
    let s = a.ship.as_ref()?;
    let i = a.idea(s.number)?;
    let b = a.builds.get(&s.number).cloned().unwrap_or_default();
    let (apps, other) = data::updates(&b.pr.as_ref().map(|x| x.files.clone()).unwrap_or_default());
    let good = |txt: String| -> Element<'a, Msg> { container(t(format!("✓ {txt}"), 12.0, p.good)).padding(Padding { top: 3.0, right: 10.0, bottom: 3.0, left: 10.0 }).style(ui::boxed(Pal::a(p.good, 0.06), 12.0, Some((Pal::a(p.good, 0.7), 1.0)))).into() };
    let plain = |txt: String| -> Element<'a, Msg> { container(t(txt, 12.0, p.text)).padding(Padding { top: 3.0, right: 10.0, bottom: 3.0, left: 10.0 }).style(ui::boxed(Color::TRANSPARENT, 12.0, Some((p.line, 1.0)))).into() };
    let cpu = b.pr.as_ref().and_then(|x| x.body.split("At rest: ").nth(1)).and_then(|r| r.split('%').next()).map(|v| v.trim().to_string()).filter(|v| v.parse::<f32>().is_ok());
    let matches: Element<'a, Msg> = mouse_area(if s.matches { good("Pictures match the mockup".into()) } else { plain("Pictures match the mockup? Look, then tick".into()) }).on_press(Msg::Matches).interaction(mouse::Interaction::Pointer).into();
    let pr: Element<'a, Msg> = mouse_area(plain(match &b.pr {
        Some(x) => format!("Pull request #{}{}", x.number, if x.reviewed { " · looked at" } else { " · not looked at yet" }),
        None => "No pull request yet".into(),
    }))
    .on_press(Msg::OpenPr(s.number))
    .interaction(mouse::Interaction::Pointer)
    .into();
    let chips = row![
        if b.ci() == Step::Done { good("CI green".into()) } else { plain(b.ci_line()) },
        matches,
        match cpu {
            Some(c) => good(format!("At rest: {c}% CPU")),
            None => plain("At rest: not measured".into()),
        },
        pr
    ]
    .spacing(8.0);
    let lbl = |x: &'static str| container(t(x, 15.0, p.muted)).width(Length::Fixed(200.0));
    let seg = |label: &'static str, on: bool, enabled: bool, m: Msg| -> Element<'a, Msg> {
        let fg = if on { p.ink } else if enabled { p.text } else { p.muted };
        button(tm(label, 15.0, fg))
            .padding(Padding { top: 8.0, right: 14.0, bottom: 8.0, left: 14.0 })
            .on_press_maybe(enabled.then_some(m))
            .style(move |_: &Theme, _| button::Style { background: Some(Background::Color(if on { p.accent } else { Color::TRANSPARENT })), text_color: fg, border: Border { radius: p.r(4.0).into(), ..Border::default() }, shadow: Shadow::default(), snap: true })
            .into()
    };
    let quick_ok = !apps.is_empty() && !other;
    let how = container(row![seg("A quick update (Hearth Store)", s.quick, quick_ok, Msg::ShipHow(true)), seg("In the next Hearth OS update", !s.quick, true, Msg::ShipHow(false))]).padding(3.0).style(ui::boxed(Color::TRANSPARENT, p.r(6.0), Some((p.line, 1.0))));
    let os = if a.os.is_empty() { "1.17".to_string() } else { a.os.clone() };
    let what = if apps.is_empty() { "Nothing a quick update can carry: it goes in the next Hearth OS update".to_string() } else if other { format!("{} and Hearth OS's own files (the next Hearth OS update)", data::app_words(&apps)) } else { data::app_words(&apps) };
    let version_ok = data::version_ok(&s.version);
    let vin = container(text_input("yours, like 1.17.2", &s.version).on_input(Msg::Version).size(15.0).padding(Padding { top: 9.0, right: 12.0, bottom: 9.0, left: 12.0 }).width(Length::Fixed(160.0)))
        .style(ui::boxed(Color::TRANSPARENT, p.r(6.0), Some((p.accent, 1.5))));
    let notes = text_editor(&s.notes).on_action(Msg::Notes).height(Length::Fixed(62.0)).size(14.0).padding(12.0);
    let people = i.count;
    let can_ship = s.stage == 0 && version_ok && s.matches && b.pr.is_some();
    let ship_label = match s.stage {
        1 => "Shipping…",
        2 | 3 => "Shipped",
        _ => "Ship it",
    };
    let mut buttons = row![Space::new().width(Length::Fill), btn(&p, "Not yet", ui::Kind::Normal, Some(Msg::NotYet)), btn(&p, ship_label, ui::Kind::Primary, can_ship.then_some(Msg::ShipNow))].spacing(10.0).align_y(iced::Center);
    if s.stage >= 2 && s.quick {
        buttons = buttons.push(btn(&p, if s.stage == 3 { "Published" } else { "Publish" }, ui::Kind::Primary, (s.stage == 2).then_some(Msg::Publish)));
    }
    let hint = if !version_ok { "Ship it lights up once the version number is filled in" } else if !s.matches { "Tick Pictures match the mockup once you've looked at them" } else { "" };
    let ready_chip = container(t("Ready to ship", 12.0, p.good)).padding(Padding { top: 3.0, right: 10.0, bottom: 3.0, left: 10.0 }).style(ui::boxed(Pal::a(p.good, 0.06), 12.0, Some((Pal::a(p.good, 0.7), 1.0))));
    let body = column![
        row![ready_chip, tb(format!("Ship {}", a.sent.get(&s.number).map(|x| x.2.trim_start_matches("idea/").replace('-', " ")).filter(|x| !x.is_empty()).unwrap_or_else(|| data::title_of(i).to_lowercase())), 26.0, p.text)].spacing(12.0).align_y(iced::Center),
        chips,
        row![lbl("Goes out as"), how].align_y(iced::Center),
        row![lbl("What updates"), t(what, 15.0, p.text)].align_y(iced::Center),
        row![lbl("For"), t(format!("Hearth OS {os} (and {os}.x)"), 15.0, p.text)].align_y(iced::Center),
        row![lbl("Version number"), vin, gap(10.0, 1.0), t("Hearth never picks it", 13.0, p.muted)].align_y(iced::Center),
        row![lbl("What's new"), notes].align_y(iced::Center),
        row![lbl("Tell who asked"), ui::check(&p, s.tell, format!("The {people} people who asked, in their Hearth Store"), false, Msg::Tell)].align_y(iced::Center),
        container(column![
            tb("What happens when you press Ship", 14.0, p.text),
            t(format!("1. The pull request goes into {BASE} · 2. The quick update is built from it (Actions › Quick update) · 3. It's added to the Store's list · 4. Publish sends it to devices: a second press, here, when you're ready."), 13.0, p.text)
        ].spacing(6.0))
        .padding(16.0)
        .width(Length::Fill)
        .style(ui::boxed(Pal::mix(p.panel, Color::BLACK, 0.12), p.r(8.0), Some((p.line, 1.0)))),
        buttons,
        row![Space::new().width(Length::Fill), t(if a.said.is_empty() || s.stage == 0 { hint.to_string() } else { a.said.clone() }, 12.0, p.muted)],
    ]
    .spacing(16.0);
    Some(container(body).padding(32.0).width(Length::Fixed(900.0)).style(ui::boxed(p.panel, p.r(12.0), Some((p.line, 1.0)))).into())
}

/// Hearth Ideas' blocks (hearth_ui::blocks).
use hearth_ui::blocks::{bl, bls, PageDef};
const BLOCKS: &[PageDef] = &[
    PageDef { id: "window", title: "Hearth Ideas", blocks: &[bl("side", "The side", 0, 0, 2, 12), bl("page", "The page", 2, 0, 10, 12)] },
    PageDef { id: "list", title: "The ideas", blocks: &[bl("title", "Title and sort", 0, 0, 12, 1), bls("ideas", "The ideas", 0, 1, 12, 10), bl("note", "The note", 0, 11, 12, 1)] },
    PageDef {
        id: "idea",
        title: "An idea",
        blocks: &[
            bl("back", "Back", 0, 0, 12, 1),
            bl("head", "Its name", 0, 1, 12, 1),
            bl("meta", "Who asked and when", 0, 2, 12, 1),
            bls("about", "What people asked", 0, 3, 7, 8),
            bls("build", "Build it", 7, 3, 5, 8),
            bl("said", "What happened", 0, 11, 12, 1),
        ],
    },
    PageDef {
        id: "built",
        title: "Being built",
        blocks: &[bl("back", "Back", 0, 0, 12, 1), bl("head", "Its name and buttons", 0, 1, 12, 1), bl("steps", "The steps", 0, 2, 4, 10), bl("pictures", "Its pictures", 4, 2, 8, 9), bl("said", "What happened", 4, 11, 8, 1)],
    },
];

fn view(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let main: Element<'_, Msg> = match a.page {
        Page::List => list_page(a),
        Page::Idea => idea_page(a),
        Page::Built => built_page(a),
    };
    // its blocks (hearth_ui::blocks): the side and the page, moved, sized, hidden or restyled; as it was when nothing is
    let mut base: Element<'_, Msg> = hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "window",
        vec![("side", sidebar(a)), ("page", container(main).padding(Padding { top: 28.0, right: 34.0, bottom: 24.0, left: 34.0 }).width(Length::Fill).height(Length::Fill).into())],
        |ps| row![ps.take("side"), ps.take("page")].into(),
        Msg::Describe,
    );
    if let Some(dlg) = ship_box(a) {
        base = ui::dialog(base, dlg, None);
    }
    let bar = hearth_ui::describe::title_bar(&p, &a.describe, G::Bell, "Hearth Ideas", None, TitleMsgs { drag: Msg::Drag, maximize: Msg::Max, minimize: Some(Msg::Min), close: Msg::Close }, Pal::mix(p.bg, Color::BLACK, 0.3), Msg::Describe(hearth_ui::describe::DMsg::Open));
    let all = container(column![bar, base]).width(Length::Fill).height(Length::Fill).style(ui::boxed(p.bg, p.window_corner, Some((p.line, 1.0))));
    ui::edges(all.into(), Msg::Resize)
}

fn subscription(a: &App) -> Subscription<Msg> {
    let mut s = vec![Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme)];
    // only while one is being built and open: once a minute
    if a.page == Page::Built && fixture().is_none() {
        s.push(iced::time::every(std::time::Duration::from_secs(60)).map(|_| Msg::Tick));
    }
    Subscription::batch(s)
}

/// The app with Describe it's box under it when it's open (every Hearth app has it).
fn shown(a: &App) -> Element<'_, Msg> {
    hearth_ui::describe::with_bar(view(a), &a.p, &a.describe, Msg::Describe)
}

fn main() -> iced::Result {
    if app::handle_cli(
        "hearth-ideas-app",
        "Hearth Ideas: the ideas people sent from Describe it, and building them with your AI team.\n  --open N            idea #N\n  --page built|ship   with --open: its Being built page, or the Ship box",
    ) {
        return Ok(());
    }
    let w = app::window("hearth-ideas", (1400.0, 860.0), (960.0, 600.0));
    iced::application(
        || {
            let (mut a, t) = App::new();
            let mut t = t;
            if app::arg("--page").as_deref() == Some("ship") && a.open > 0 {
                // the Ship box opens once the pull request is read (its files say what updates)
                let n = a.open;
                a.page = Page::Built;
                t = t.chain(Task::done(Msg::ShipOpen(n)));
            }
            (a, t)
        },
        update,
        shown,
    )
    .title(|_: &App| "Hearth Ideas".to_string())
    .subscription(subscription)
    .theme(|a: &App| app::iced_theme(&a.p))
    .window(w)
    .default_font(app::INTER)
    .run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_both_ways() {
        for s in ["", "a", "ab", "abc", "abcd", "Hearth Ideas ✓"] {
            assert_eq!(String::from_utf8(b64_decode(&b64_encode(s.as_bytes()))).unwrap(), s);
        }
        assert_eq!(b64_decode("aGk=\n"), b"hi");
    }

    #[test]
    fn the_roadmap_gets_one_line_under_later() {
        let r = "# Roadmap\n\n## Planned for 1.18\n- x\n\n## Later\n- y\n\n## 1.17 (released)\n";
        let a = roadmap_add(r, "- [ ] Instant replay");
        assert!(a.contains("## Later\n\n### Ideas people asked for\n\n- [ ] Instant replay\n- y"), "{a}");
        let b = roadmap_add(&a, "- [ ] Two folders");
        assert!(b.contains("### Ideas people asked for\n- [ ] Two folders\n\n- [ ] Instant replay"), "{b}");
        assert_eq!(roadmap_add(&b, "- [ ] Two folders"), b, "never twice");
    }

    #[test]
    fn labels_change_only_the_state() {
        let mut i = data::ideas(&json!([{"number": 3, "title": "Idea · bar · x · 1 asked", "labels": [{"name": "idea"}, {"name": "idea:bar"}, {"name": "state:later"}], "body": ""}])).remove(0);
        i.state = State::Later;
        assert_eq!(labels_with(&i, State::Building), ["idea", "idea:bar", "state:building"]);
        assert_eq!(labels_with(&i, State::New), ["idea", "idea:bar"]);
    }
}
