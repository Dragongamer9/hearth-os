//! What Hearth Ideas reads, as plain data (no window): the ideas people sent from Describe it, which the logs service
//! keeps as GitHub issues labelled "idea" in the owner's private Hearth-Organization/hearth-logs (counter/src/index.js
//! ideaToGitHub: one issue per idea, "Asked again …" comments for each person after the first); where each one is
//! (its `state:` label); and how one being built is getting on (its branch, CI, pictures and pull request in
//! Dragongamer9/hearth-os). Everything here is read with the owner's own `gh` sign-in; nothing is kept on the device.

use serde_json::Value;

pub const LOGS: &str = "Hearth-Organization/hearth-logs";
pub const REPO: &str = "Dragongamer9/hearth-os";
pub const BASE: &str = "bughunt";

/// Where an idea is: the sidebar's lists (board ideas-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    New,
    Later,
    Building,
    Ready,
    Shipped,
    No,
}

impl State {
    pub const ALL: [State; 6] = [State::New, State::Later, State::Building, State::Ready, State::Shipped, State::No];
    pub fn name(self) -> &'static str {
        match self {
            State::New => "New",
            State::Later => "Later",
            State::Building => "Being built",
            State::Ready => "Ready to ship",
            State::Shipped => "Shipped",
            State::No => "Said no",
        }
    }
    /// The label on the issue ("" for New: no state label).
    pub fn label(self) -> &'static str {
        match self {
            State::New => "",
            State::Later => "state:later",
            State::Building => "state:building",
            State::Ready => "state:ready",
            State::Shipped => "state:shipped",
            State::No => "state:no",
        }
    }
    pub fn back_to(self) -> &'static str {
        match self {
            State::New => "New ideas",
            State::Later => "Later",
            State::Building => "Being built",
            State::Ready => "Ready to ship",
            State::Shipped => "Shipped",
            State::No => "Said no",
        }
    }
}

/// One person's ask: their words, whether they drew a sketch and kept what Describe it made.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Ask {
    pub words: String,
    pub sketch: bool,
    pub kept: bool,
    pub at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Idea {
    pub number: u64,
    pub key: String,
    pub part: String,
    pub app: String,
    pub needs: Vec<String>,
    /// what Describe it made instead (its name), if anything
    pub closest: String,
    pub first: Ask,
    pub count: u64,
    pub state: State,
    pub created: String,
    pub updated: String,
    pub labels: Vec<String>,
    pub comments: u64,
}

fn s(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or("").to_string()
}

/// The JSON block in an idea issue's body.
fn body_json(body: &str) -> Value {
    let Some(a) = body.find("```json") else { return Value::Null };
    let rest = &body[a + 7..];
    let Some(b) = rest.find("```") else { return Value::Null };
    serde_json::from_str(rest[..b].trim()).unwrap_or(Value::Null)
}

/// "Idea · part · needs · 23 asked" → 23.
fn count_in(title: &str) -> u64 {
    title.rsplit(" · ").next().and_then(|t| t.strip_suffix(" asked")).and_then(|n| n.trim().parse().ok()).unwrap_or(1)
}

pub fn state_of(labels: &[String]) -> State {
    for st in [State::Shipped, State::No, State::Ready, State::Building, State::Later] {
        if labels.iter().any(|l| l == st.label()) {
            return st;
        }
    }
    State::New
}

/// An idea from its issue (`gh api repos/…/issues`); None for anything that isn't one.
pub fn idea(issue: &Value) -> Option<Idea> {
    if issue.get("pull_request").is_some() {
        return None;
    }
    let labels: Vec<String> = issue.get("labels").and_then(Value::as_array).map(|a| a.iter().map(|l| s(l, "name")).collect()).unwrap_or_default();
    if !labels.iter().any(|l| l == "idea") {
        return None;
    }
    let body = s(issue, "body");
    let j = body_json(&body);
    let key = body.rsplit("key: `").next().and_then(|r| r.split('`').next()).filter(|_| body.contains("key: `")).unwrap_or("").to_string();
    let title = s(issue, "title");
    let needs: Vec<String> = j.get("needs").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
    Some(Idea {
        number: issue.get("number").and_then(Value::as_u64)?,
        key,
        part: s(&j, "part"),
        app: s(&j, "app"),
        needs,
        closest: j.get("closest").map(|c| s(c, "name")).unwrap_or_default(),
        first: Ask { words: s(&j, "asked"), sketch: j.get("sketch").and_then(Value::as_bool) == Some(true), kept: j.get("kept").and_then(Value::as_bool) == Some(true), at: s(issue, "created_at") },
        count: count_in(&title),
        state: state_of(&labels),
        created: s(issue, "created_at"),
        updated: s(issue, "updated_at"),
        labels,
        comments: issue.get("comments").and_then(Value::as_u64).unwrap_or(0),
    })
}

pub fn ideas(list: &Value) -> Vec<Idea> {
    let mut v: Vec<Idea> = list.as_array().map(|a| a.iter().filter_map(idea).collect()).unwrap_or_default();
    // the most asked first, then the newest
    v.sort_by(|a, b| b.count.cmp(&a.count).then(b.updated.cmp(&a.updated)));
    v
}

/// The asks after the first, from the "Asked again (N times now): "…" · Hearth OS x · with a sketch" comments.
pub fn asks(comments: &Value) -> Vec<Ask> {
    let mut out = Vec::new();
    for c in comments.as_array().into_iter().flatten() {
        let b = s(c, "body");
        let Some(rest) = b.strip_prefix("Asked again") else { continue };
        let Some(q1) = rest.find(": \"") else { continue };
        let words_and = &rest[q1 + 3..];
        let Some(q2) = words_and.rfind("\" · Hearth OS") else { continue };
        let tail = &words_and[q2..];
        out.push(Ask { words: words_and[..q2].to_string(), sketch: tail.contains("with a sketch"), kept: tail.contains("kept what Describe it made"), at: s(c, "created_at") });
    }
    out
}

/// The Build it comment Hearth Ideas leaves ("Sent to your AI team: Claude leads, ChatGPT checks · branch idea/x"),
/// the newest one: (lead, checks, branch, when).
pub fn sent(comments: &Value) -> Option<(String, String, String, String)> {
    let mut found = None;
    for c in comments.as_array().into_iter().flatten() {
        let b = s(c, "body");
        let Some(rest) = b.strip_prefix("Sent to your AI team: ") else { continue };
        let mut lead = String::new();
        let mut checks = String::new();
        let mut branch = String::new();
        for part in rest.lines().next().unwrap_or("").split(" · ") {
            for bit in part.split(", ") {
                if let Some(x) = bit.strip_suffix(" leads") {
                    lead = x.to_string();
                } else if let Some(x) = bit.strip_suffix(" checks") {
                    checks = x.to_string();
                } else if let Some(x) = bit.strip_prefix("branch ") {
                    branch = x.trim_matches('`').to_string();
                }
            }
        }
        found = Some((lead, checks, branch, s(c, "created_at")));
    }
    found
}

/// The idea's own branch: "idea/" and its first few words.
pub fn branch_for(title: &str) -> String {
    let words: Vec<String> = title
        .to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty() && !["the", "a", "an", "of", "to", "my", "and", "in", "on", "for", "like"].contains(w))
        .take(4)
        .map(str::to_string)
        .collect();
    format!("idea/{}", if words.is_empty() { "new".to_string() } else { words.join("-") })
}

/// Where CI puts an idea's pictures: the branch ci/shots-<its branch, / as ->.
pub fn shots_branch(branch: &str) -> String {
    format!("ci/shots-{}", branch.replace('/', "-"))
}

/// The idea's title: what most of them asked for, short ("Record the last 30 seconds of a game").
pub fn title_of(i: &Idea) -> String {
    let t = i.needs.first().cloned().unwrap_or_else(|| i.first.words.clone());
    let mut t: String = t.trim().trim_end_matches('.').chars().take(70).collect();
    if let Some(c) = t.get(..1) {
        t = c.to_uppercase() + &t[1..];
    }
    t
}

/// What kind of idea it is (the chip by its title).
pub fn kind_of(i: &Idea, asks: &[Ask]) -> &'static str {
    if !i.needs.is_empty() {
        "Needs something new"
    } else if i.first.sketch || asks.iter().any(|a| a.sketch) {
        "Has a sketch"
    } else {
        "Built by people already"
    }
}

/// "Nov 22" from an ISO date.
pub fn day(iso: &str) -> String {
    const M: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let m: usize = iso.get(5..7).and_then(|x| x.parse().ok()).unwrap_or(0);
    let d: u32 = iso.get(8..10).and_then(|x| x.parse().ok()).unwrap_or(0);
    if m == 0 || d == 0 { String::new() } else { format!("{} {d}", M[m - 1]) }
}

/// Seconds since the epoch from an ISO time ("2026-11-22T08:31:00Z"), UTC.
pub fn epoch(iso: &str) -> i64 {
    let n = |a: usize, b: usize| iso.get(a..b).and_then(|x| x.parse::<i64>().ok()).unwrap_or(0);
    let (y, mo, d, h, mi, se) = (n(0, 4), n(5, 7), n(8, 10), n(11, 13), n(14, 16), n(17, 19));
    if y == 0 {
        return 0;
    }
    // days from civil (Howard Hinnant)
    let y2 = if mo <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * (if mo > 2 { mo - 3 } else { mo + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146097 + doe - 719468) * 86400 + h * 3600 + mi * 60 + se
}

/// "2 hours ago".
pub fn ago(iso: &str, now: i64) -> String {
    let t = epoch(iso);
    if t == 0 {
        return String::new();
    }
    let s = (now - t).max(0);
    match s {
        0..=59 => "just now".into(),
        60..=3599 => plural(s / 60, "minute"),
        3600..=86399 => plural(s / 3600, "hour"),
        _ => plural(s / 86400, "day"),
    }
}
fn plural(n: i64, w: &str) -> String {
    format!("{n} {w}{} ago", if n == 1 { "" } else { "s" })
}

/// The time of day in the device's own zone ("8:31 AM"), for the steps.
pub fn clock(iso: &str) -> String {
    let t = epoch(iso);
    if t == 0 {
        return String::new();
    }
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    let tt = t as libc::time_t;
    unsafe { libc::localtime_r(&tt, &mut tm) };
    let h = tm.tm_hour;
    format!("{}:{:02} {}", if h % 12 == 0 { 12 } else { h % 12 }, tm.tm_min, if h < 12 { "AM" } else { "PM" })
}

// ---------------- being built ----------------

/// A step on board ideas-2: done, running or waiting.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Step {
    Done,
    Running,
    Waiting,
    Failed,
}

/// What's known about an idea being built.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Build {
    pub lead: String,
    pub checks: String,
    pub branch: String,
    pub sent_at: String,
    /// the branch's newest commit, and when
    pub sha: String,
    pub commit_at: String,
    /// CI on that commit: (name, status, conclusion)
    pub runs: Vec<(String, String, String)>,
    /// the pictures on its shots branch: (file name, when)
    pub shots: Vec<(String, String)>,
    /// its pull request to bughunt: number, open, merged, reviewed, body
    pub pr: Option<Pr>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Pr {
    pub number: u64,
    pub open: bool,
    pub merged: bool,
    pub reviewed: bool,
    pub body: String,
    pub files: Vec<String>,
}

impl Build {
    pub fn ci(&self) -> Step {
        if self.runs.is_empty() {
            return if self.sha.is_empty() { Step::Waiting } else { Step::Running };
        }
        if self.runs.iter().any(|r| r.1 != "completed") {
            return Step::Running;
        }
        if self.runs.iter().all(|r| matches!(r.2.as_str(), "success" | "skipped" | "neutral")) { Step::Done } else { Step::Failed }
    }
    /// "Mantel in Rust: 14 tests pass · Capture: 9 pass", or how many checks there are.
    pub fn ci_line(&self) -> String {
        if self.runs.is_empty() {
            return if self.sha.is_empty() { "waiting for the branch".into() } else { "waiting for GitHub Actions".into() };
        }
        let ok = self.runs.iter().filter(|r| r.2 == "success").count();
        let bad: Vec<&str> = self.runs.iter().filter(|r| r.1 == "completed" && !matches!(r.2.as_str(), "success" | "skipped" | "neutral")).map(|r| r.0.as_str()).collect();
        let going = self.runs.iter().filter(|r| r.1 != "completed").count();
        let mut parts = vec![format!("{ok} of {} checks pass", self.runs.len())];
        if going > 0 {
            parts.push(format!("{going} running"));
        }
        if !bad.is_empty() {
            parts.push(format!("failed: {}", bad.join(", ")));
        }
        parts.join(" · ")
    }
    pub fn shots_step(&self) -> Step {
        if !self.shots.is_empty() && self.ci() == Step::Done {
            Step::Done
        } else if !self.sha.is_empty() {
            if self.ci() == Step::Failed { Step::Waiting } else { Step::Running }
        } else {
            Step::Waiting
        }
    }
    pub fn pr_step(&self) -> Step {
        match &self.pr {
            Some(p) if p.merged || p.open => Step::Done,
            _ => Step::Waiting,
        }
    }
    /// Ready: CI green, pictures taken, its pull request open (the owner looks at it before shipping).
    pub fn ready(&self) -> bool {
        self.ci() == Step::Done && self.pr.as_ref().map(|p| p.open).unwrap_or(false)
    }
    /// What the AI team says: the pull request's own words, first paragraph.
    pub fn says(&self) -> String {
        let b = self.pr.as_ref().map(|p| p.body.clone()).unwrap_or_default();
        let para: String = b.split("\n\n").map(str::trim).find(|p| !p.is_empty() && !p.starts_with('#')).unwrap_or("").chars().take(400).collect();
        para.replace('\n', " ")
    }
}

pub fn runs_of(check_runs: &Value) -> Vec<(String, String, String)> {
    check_runs.get("check_runs").and_then(Value::as_array).map(|a| a.iter().map(|r| (s(r, "name"), s(r, "status"), s(r, "conclusion"))).collect()).unwrap_or_default()
}

pub fn pr_of(pulls: &Value) -> Option<Pr> {
    let p = pulls.as_array()?.first()?;
    Some(Pr {
        number: p.get("number").and_then(Value::as_u64).unwrap_or(0),
        open: s(p, "state") == "open",
        merged: p.get("merged_at").map(|m| !m.is_null()).unwrap_or(false),
        reviewed: false,
        body: s(p, "body"),
        files: Vec::new(),
    })
}

// ---------------- ship it ----------------

/// The quick updates a pull request's files make (quick-update.yml's app names), and whether anything outside them
/// changed (then it can only go out in the next Hearth OS update).
pub fn updates(files: &[String]) -> (Vec<&'static str>, bool) {
    const APPS: &[(&str, &str)] = &[
        ("capture", "hearth-capture-app"),
        ("files", "hearth-files-app"),
        ("settings", "hearth-settings-app"),
        ("task-manager", "hearth-task-manager"),
        ("notes", "hearth-notes-app"),
        ("g15", "hearth-g15-app"),
        ("family", "hearth-family-app"),
        ("server", "hearth-server-app"),
        ("resmon", "hearth-resmon-app"),
        ("system-restore", "hearth-system-restore"),
        ("reliability", "hearth-reliability-history"),
        ("pad", "hearth-pad-host"),
        ("store", "hearth-store-app"),
        ("ideas", "hearth-ideas-app"),
    ];
    let mut out: Vec<&'static str> = Vec::new();
    let mut other = false;
    let add = |x: &'static str, out: &mut Vec<&'static str>| {
        if !out.contains(&x) {
            out.push(x)
        }
    };
    for f in files {
        if f.starts_with("docs/") || f.starts_with(".github/") || f.ends_with(".md") || f.contains("/test/") || f.contains("/tests/") {
            continue; // not on a device
        }
        if f.starts_with("hosts/linux/desktop/shell/") || f.starts_with("hosts/linux/desktop/theme/") || f.starts_with("hosts/linux/desktop/sounds/") {
            add("mantel", &mut out);
        } else if f.starts_with("hosts/mantel-rs/") {
            add("mantel-rust", &mut out);
        } else if let Some(rest) = f.strip_prefix("hosts/hearth-ui/apps/") {
            let app = rest.split('/').next().unwrap_or("");
            match APPS.iter().find(|a| a.0 == app) {
                Some(a) => add(a.1, &mut out),
                None => other = true,
            }
        } else {
            other = true;
        }
    }
    (out, other)
}

/// The words the Ship box shows for a quick update's app ("Mantel in Rust and Capture").
pub fn app_words(apps: &[&str]) -> String {
    let w: Vec<String> = apps
        .iter()
        .map(|a| match *a {
            "mantel" => "Mantel".to_string(),
            "mantel-rust" => "Mantel in Rust".to_string(),
            x => {
                let x = x.trim_start_matches("hearth-").trim_end_matches("-app").replace('-', " ");
                let mut c = x.chars();
                c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
            }
        })
        .collect();
    match w.len() {
        0 => String::new(),
        1 => w[0].clone(),
        n => format!("{} and {}", w[..n - 1].join(", "), w[n - 1]),
    }
}

/// The owner's version number: digits and dots, like 1.17.2 (quick-update.yml checks the same).
pub fn version_ok(v: &str) -> bool {
    let parts: Vec<&str> = v.split('.').collect();
    (2..=4).contains(&parts.len()) && parts.iter().all(|p| !p.is_empty() && p.len() <= 4 && p.chars().all(|c| c.is_ascii_digit()))
}

/// "1.17" from the newest release's tag ("v1.17", "1.17.0").
pub fn os_of(tag: &str) -> String {
    let t = tag.trim_start_matches('v');
    let p: Vec<&str> = t.split('.').take(2).collect();
    if p.len() == 2 && p.iter().all(|x| !x.is_empty() && x.chars().all(|c| c.is_ascii_digit())) { p.join(".") } else { String::new() }
}

/// What the AI team gets (the file the Command Center reads: hearth-ai-command-center electron/ideas.cjs).
pub fn brief(i: &Idea, asks: &[Ask], branch: &str, lead: &str, checks: &str, mockup_first: bool) -> Value {
    let mut b = String::new();
    b.push_str(&format!("People asked for this in {} ({} of them", hearth_layout::feature::part_label(&i.part), i.count));
    let sketches = usize::from(i.first.sketch) + asks.iter().filter(|a| a.sketch).count();
    if sketches > 0 {
        b.push_str(&format!(", {sketches} with a sketch"));
    }
    b.push_str("). In their words:\n");
    for a in std::iter::once(&i.first).chain(asks.iter()).take(12) {
        b.push_str(&format!("- \"{}\"\n", a.words));
    }
    if !i.needs.is_empty() {
        b.push_str(&format!("\nWhat Hearth can't do yet: {}.\n", i.needs.join("; ")));
    }
    if !i.closest.is_empty() {
        let kept = usize::from(i.first.kept) + asks.iter().filter(|a| a.kept).count();
        b.push_str(&format!("Describe it made \"{}\" instead; {kept} of them kept it.\n", i.closest));
    }
    b.push_str(&format!(
        "\nThe idea is issue #{} in {LOGS}. Put CI's pictures of it on the branch {} (jpg or png, named after what they show), next to its mockup and how it looks now. Follow the repo's CLAUDE.md.",
        i.number,
        shots_branch(branch)
    ));
    serde_json::json!({
        "format": 1,
        "key": format!("hearth-ideas-{}", i.number),
        "title": title_of(i),
        "brief": b,
        "repo": REPO,
        "base": BASE,
        "branch": branch,
        "lead": lead,
        "checks": checks,
        "mockupFirst": mockup_first,
    })
}

/// The lines added to docs/ROADMAP.md by "Add to the roadmap" (under "## Ideas people asked for").
pub fn roadmap_line(i: &Idea, asks: &[Ask]) -> String {
    let words: Vec<String> = std::iter::once(&i.first).chain(asks.iter()).take(3).map(|a| format!("\"{}\"", a.words)).collect();
    format!("- [ ] {} ({}; {} asked; {LOGS}#{}): {}", title_of(i), hearth_layout::feature::part_label(&i.part), i.count, i.number, words.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn issue() -> Value {
        json!({
            "number": 812, "title": "Idea · quickaccess · record the last 30 seconds of a game · 23 asked", "state": "open",
            "labels": [{"name": "idea"}, {"name": "idea:quickaccess"}],
            "created_at": "2026-11-22T08:00:00Z", "updated_at": "2026-11-22T10:00:00Z", "comments": 22,
            "body": "**Asked in:** quickaccess\n\n```json\n{\"format\":1,\"part\":\"quickaccess\",\"asked\":\"Add a button that records the last 30 seconds of my game, like the Xbox does.\",\"needs\":[\"record the last 30 seconds of a game\"],\"closest\":{\"name\":\"A Record button in Quick Access\"},\"sketch\":false,\"hearth\":\"1.17\",\"app\":\"\",\"kept\":true}\n```\n\nkey: `quickaccess:record the last 30 seconds of a game`"
        })
    }

    #[test]
    fn an_issue_is_read() {
        let i = idea(&issue()).unwrap();
        assert_eq!(i.count, 23);
        assert_eq!(i.part, "quickaccess");
        assert_eq!(i.closest, "A Record button in Quick Access");
        assert!(i.first.kept);
        assert_eq!(i.state, State::New);
        assert_eq!(i.key, "quickaccess:record the last 30 seconds of a game");
        assert_eq!(title_of(&i), "Record the last 30 seconds of a game");
        assert_eq!(kind_of(&i, &[]), "Needs something new");
        let mut p = issue();
        p["pull_request"] = json!({});
        assert!(idea(&p).is_none(), "a pull request isn't an idea");
    }

    #[test]
    fn labels_say_where_it_is() {
        let l = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(state_of(&l(&["idea", "state:later"])), State::Later);
        assert_eq!(state_of(&l(&["idea", "state:building", "state:shipped"])), State::Shipped);
        assert_eq!(state_of(&l(&["idea"])), State::New);
    }

    #[test]
    fn asks_and_the_sent_line_are_read_from_comments() {
        let c = json!([
            {"body": "Asked again (2 times now): \"Instant replay like Shadowplay, in Game Mode too\" · Hearth OS 1.17", "created_at": "2026-11-22T09:00:00Z"},
            {"body": "Asked again (3 times now): \"save the last minute\" · Hearth OS 1.17 · with a sketch · kept what Describe it made", "created_at": "2026-11-22T09:30:00Z"},
            {"body": "Sent to your AI team: Claude leads, ChatGPT checks · branch `idea/instant-replay`", "created_at": "2026-11-22T14:31:00Z"},
            {"body": "something else"}
        ]);
        let a = asks(&c);
        assert_eq!(a.len(), 2);
        assert_eq!(a[0].words, "Instant replay like Shadowplay, in Game Mode too");
        assert!(a[1].sketch && a[1].kept && !a[0].sketch);
        let s = sent(&c).unwrap();
        assert_eq!((s.0.as_str(), s.1.as_str(), s.2.as_str()), ("Claude", "ChatGPT", "idea/instant-replay"));
    }

    #[test]
    fn a_branch_name_from_the_title() {
        assert_eq!(branch_for("Record the last 30 seconds of a game"), "idea/record-last-30-seconds");
        assert_eq!(branch_for("!!!"), "idea/new");
        assert_eq!(shots_branch("idea/instant-replay"), "ci/shots-idea-instant-replay");
    }

    #[test]
    fn what_a_pull_request_updates() {
        let f = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        let (a, other) = updates(&f(&["hosts/mantel-rs/src/quick.rs", "hosts/hearth-ui/apps/capture/src/main.rs", "docs/ROADMAP.md", ".github/workflows/x.yml"]));
        assert_eq!(a, ["mantel-rust", "hearth-capture-app"]);
        assert!(!other);
        assert_eq!(app_words(&a), "Mantel in Rust and Capture");
        let (_, other) = updates(&f(&["os/system_files/usr/lib/systemd/system/x.service"]));
        assert!(other, "a system file goes out with Hearth OS");
    }

    #[test]
    fn version_numbers_are_the_owners() {
        assert!(version_ok("1.17.2"));
        assert!(!version_ok(""));
        assert!(!version_ok("1"));
        assert!(!version_ok("1.17.x"));
        assert!(!version_ok("1.17.2; rm"));
        assert_eq!(os_of("v1.17"), "1.17");
        assert_eq!(os_of("1.17.0"), "1.17");
        assert_eq!(os_of("part-x"), "");
    }

    #[test]
    fn times() {
        assert_eq!(epoch("1970-01-02T00:00:00Z"), 86400);
        assert_eq!(ago("2026-11-22T08:00:00Z", epoch("2026-11-22T10:00:00Z")), "2 hours ago");
        assert_eq!(day("2026-11-22T08:00:00Z"), "Nov 22");
    }

    #[test]
    fn the_brief_keeps_the_rules_to_the_command_center() {
        let i = idea(&issue()).unwrap();
        let b = brief(&i, &[], "idea/instant-replay", "claude", "chatgpt", true);
        assert_eq!(b["base"], "bughunt");
        assert_eq!(b["branch"], "idea/instant-replay");
        assert_eq!(b["mockupFirst"], true);
        assert!(b["brief"].as_str().unwrap().contains("ci/shots-idea-instant-replay"));
        assert!(b["brief"].as_str().unwrap().contains("1 of them kept it"));
    }

    #[test]
    fn steps() {
        let mut b = Build { sha: "abc".into(), ..Default::default() };
        assert_eq!(b.ci(), Step::Running);
        b.runs = vec![("build".into(), "completed".into(), "success".into())];
        assert_eq!(b.ci(), Step::Done);
        assert!(!b.ready(), "no pull request yet");
        b.pr = Some(Pr { number: 812, open: true, ..Default::default() });
        assert!(b.ready());
        b.runs.push(("test".into(), "completed".into(), "failure".into()));
        assert_eq!(b.ci(), Step::Failed);
        assert!(b.ci_line().contains("failed: test"));
    }
}
