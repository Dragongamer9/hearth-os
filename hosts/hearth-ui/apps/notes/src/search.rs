//! Search (page 4): every note's text, with filters by folder, tag and date, the matches counted and shown in their
//! lines; and the quick switcher (Ctrl+O): a fuzzy match on the notes' paths and names, then lines that mention it.

use crate::day::Ymd;
use crate::vault::Vault;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Query {
    pub text: String,
    /// Aa: match case.
    pub case: bool,
    /// "": the words as one phrase.
    pub exact: bool,
    /// Only in these folders (path starts).
    pub folders: Vec<String>,
    pub tag: Option<String>,
    /// Only notes from the last N days (a log's day, else when it last changed).
    pub days: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub note: usize,
    pub count: usize,
    /// Up to two lines with where each match is in them (byte ranges).
    pub lines: Vec<(String, Vec<(usize, usize)>)>,
}

fn terms(q: &Query) -> Vec<String> {
    let t = q.text.trim();
    if t.is_empty() {
        return Vec::new();
    }
    let v: Vec<String> = if q.exact { vec![t.trim_matches('"').to_string()] } else { t.split_whitespace().map(str::to_string).collect() };
    if q.case { v } else { v.into_iter().map(|s| s.to_lowercase()).collect() }
}

/// Where `needles` are in `hay` (byte ranges into `hay`), case folded unless `case`.
pub fn find_all(hay: &str, needles: &[String], case: bool) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let folded;
    let h: &str = if case {
        hay
    } else {
        folded = hay.to_lowercase();
        if folded.len() != hay.len() {
            // Lower-casing changed the length (rare letters): match on the original, ASCII-folded.
            return find_ascii(hay, needles);
        }
        &folded
    };
    for n in needles.iter().filter(|n| !n.is_empty()) {
        let mut from = 0;
        while let Some(at) = h[from..].find(n.as_str()) {
            out.push((from + at, from + at + n.len()));
            from += at + n.len();
        }
    }
    out.sort();
    out
}

fn find_ascii(hay: &str, needles: &[String]) -> Vec<(usize, usize)> {
    let h = hay.to_ascii_lowercase();
    let mut out = Vec::new();
    for n in needles {
        let n = n.to_ascii_lowercase();
        if n.is_empty() {
            continue;
        }
        let mut from = 0;
        while let Some(at) = h[from..].find(&n) {
            out.push((from + at, from + at + n.len()));
            from += at + n.len();
        }
    }
    out.sort();
    out
}

fn in_window(v: &Vault, i: usize, today: Ymd, days: u32) -> bool {
    let n = &v.notes[i];
    let d = n.log.unwrap_or_else(|| crate::day::log_day(n.mtime, 0));
    crate::day::between(d, today) < days as i64
}

pub fn search(v: &Vault, q: &Query, today: Ymd) -> Vec<Hit> {
    let ts = terms(q);
    if ts.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (i, n) in v.notes.iter().enumerate() {
        if !q.folders.is_empty() && !q.folders.iter().any(|f| n.rel == *f || n.rel.starts_with(&format!("{}/", f.trim_end_matches('/')))) {
            continue;
        }
        if let Some(t) = &q.tag {
            if !n.tags.iter().any(|x| x.eq_ignore_ascii_case(t)) {
                continue;
            }
        }
        if let Some(d) = q.days {
            if !in_window(v, i, today, d) {
                continue;
            }
        }
        // Every word must be in the note (anywhere); the count is all their matches.
        let whole = find_all(&n.text, &ts, q.case);
        let all_there = ts.iter().all(|t| !find_all(&n.text, std::slice::from_ref(t), q.case).is_empty());
        if !all_there || whole.is_empty() {
            continue;
        }
        let mut lines = Vec::new();
        for l in n.text.lines() {
            let m = find_all(l, &ts, q.case);
            if !m.is_empty() {
                lines.push((l.trim_end().to_string(), m));
                if lines.len() == 2 {
                    break;
                }
            }
        }
        out.push(Hit { note: i, count: whole.len(), lines });
    }
    out.sort_by(|a, b| b.count.cmp(&a.count).then(a.note.cmp(&b.note)));
    out
}

/// A fuzzy match of `q` in `s`: every letter in order. Its score (higher is better) and the matched letters' places.
pub fn fuzzy(q: &str, s: &str) -> Option<(i32, Vec<usize>)> {
    let q: Vec<char> = q.to_lowercase().chars().filter(|c| !c.is_whitespace()).collect();
    if q.is_empty() {
        return Some((0, Vec::new()));
    }
    let sc: Vec<char> = s.to_lowercase().chars().collect();
    let mut at = Vec::new();
    let mut qi = 0;
    let mut score = 0i32;
    let mut last: Option<usize> = None;
    for (i, c) in sc.iter().enumerate() {
        if qi < q.len() && *c == q[qi] {
            at.push(i);
            let start = i == 0 || matches!(sc[i - 1], '/' | '-' | ' ' | '_' | '.');
            score += 10 + if start { 8 } else { 0 } + if last == Some(i.wrapping_sub(1)) { 6 } else { 0 };
            last = Some(i);
            qi += 1;
        }
    }
    if qi < q.len() {
        return None;
    }
    score -= (sc.len() as i32) / 8;
    // A whole-word match of the query scores most.
    if s.to_lowercase().contains(&q.iter().collect::<String>()) {
        score += 30;
    }
    Some((score, at))
}

#[derive(Debug, Clone, PartialEq)]
pub struct Switch {
    pub note: usize,
    /// What's shown: "g15-app / README.md", or the log's name.
    pub name: String,
    pub path: String,
    /// The name's matched letters (char places).
    pub at: Vec<usize>,
    /// A line that mentions it ("…mockups of the G15 drivers…"), for a match in the text only.
    pub line: Option<String>,
}

/// The quick switcher's list: names first, then notes whose text mentions it.
pub fn switcher(v: &Vault, q: &str, max: usize) -> Vec<Switch> {
    let mut by_name: Vec<(i32, Switch)> = Vec::new();
    for (i, n) in v.notes.iter().enumerate() {
        let file = n.rel.rsplit('/').next().unwrap_or(&n.rel).to_string();
        let name = if file.eq_ignore_ascii_case("README.md") { format!("{} / {}", n.label, file) } else { file.clone() };
        let dir = n.rel.rsplit_once('/').map(|x| x.0.to_string()).unwrap_or_default();
        if let Some((s, at)) = fuzzy(q, &name) {
            by_name.push((s + 5, Switch { note: i, name, path: dir, at, line: None }));
        } else if let Some((s, _)) = fuzzy(q, &n.rel) {
            by_name.push((s - 10, Switch { note: i, name, path: dir, at: Vec::new(), line: None }));
        }
    }
    by_name.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.note.cmp(&b.1.note)));
    let mut out: Vec<Switch> = by_name.into_iter().take(max).map(|x| x.1).collect();
    if q.trim().len() >= 2 && out.len() < max {
        let ql = q.trim().to_lowercase();
        for (i, n) in v.notes.iter().enumerate() {
            if out.len() >= max {
                break;
            }
            if out.iter().any(|s| s.note == i) {
                continue;
            }
            if let Some(l) = n.text.lines().find(|l| l.to_lowercase().contains(&ql)) {
                let file = n.rel.rsplit('/').next().unwrap_or(&n.rel).to_string();
                let name = if file.eq_ignore_ascii_case("README.md") { format!("{} / {}", n.label, file) } else { file };
                out.push(Switch { note: i, name, path: n.rel.rsplit_once('/').map(|x| x.0.to_string()).unwrap_or_default(), at: Vec::new(), line: Some(snippet(l, &ql, 60)) });
            }
        }
    }
    out
}

/// "…a part of the line around the match…"
pub fn snippet(line: &str, q: &str, width: usize) -> String {
    let low = line.to_lowercase();
    let at = low.find(q).unwrap_or(0);
    let chars: Vec<char> = line.trim().chars().collect();
    let pos = line[..at.min(line.len())].chars().count().saturating_sub(line.len() - line.trim_start().len());
    let start = pos.saturating_sub(width / 3);
    let end = (start + width).min(chars.len());
    let mut s: String = chars[start..end].iter().collect();
    if start > 0 {
        s = format!("…{s}");
    }
    if end < chars.len() {
        s.push('…');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_text_with_filters_and_counts() {
        let d = tempfile::tempdir().unwrap();
        crate::vault::sample(d.path());
        let v = Vault::scan(d.path());
        let q = Query { text: "hearth-store".into(), ..Default::default() };
        let h = search(&v, &q, (2026, 11, 20));
        assert!(h.len() >= 2, "{h:?}");
        let log = v.find("log/2026-11-19.md").unwrap();
        let hit = h.iter().find(|x| x.note == log).unwrap();
        assert_eq!(hit.count, 1);
        let (line, at) = &hit.lines[0];
        assert_eq!(&line[at[0].0..at[0].1], "hearth-store");
        // Folder and tag filters.
        let q2 = Query { text: "hearth".into(), folders: vec!["log".into()], ..Default::default() };
        assert!(search(&v, &q2, (2026, 11, 20)).iter().all(|x| v.notes[x.note].rel.starts_with("log/")));
        let q3 = Query { text: "store".into(), tag: Some("mockup".into()), ..Default::default() };
        assert_eq!(search(&v, &q3, (2026, 11, 20)).len(), 1);
        // Both words must be there; case only with Aa.
        assert!(search(&v, &Query { text: "Fedora nonsense".into(), ..Default::default() }, (2026, 11, 20)).is_empty());
        assert!(search(&v, &Query { text: "HEARTH STORE".into(), case: true, ..Default::default() }, (2026, 11, 20)).is_empty());
    }

    #[test]
    fn the_quick_switcher_is_fuzzy() {
        let d = tempfile::tempdir().unwrap();
        crate::vault::sample(d.path());
        let v = Vault::scan(d.path());
        let s = switcher(&v, "hstore", 6);
        assert_eq!(s[0].name, "hearth-store / README.md");
        assert_eq!(s[0].path, "hearth/mockups/hearth-store");
        assert!(fuzzy("g15", "g15-app / README.md").unwrap().1 == vec![0, 1, 2]);
        assert!(fuzzy("zzz", "hearth").is_none());
        let s = switcher(&v, "freeware scripts", 6);
        assert!(s.iter().any(|x| x.line.as_deref().map(|l| l.contains("freeware scripts")).unwrap_or(false) || x.name.contains("ts2010")));
        assert_eq!(snippet("the Fedora offline USB (installer ISO)", "usb", 20), "…fline USB (installer…");
    }
}
