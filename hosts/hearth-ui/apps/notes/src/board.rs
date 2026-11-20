//! The Board (page 2b): a kanban made from every `- [ ]` checklist line and every "Waiting:" line in the vault.
//! Nothing is kept apart from the notes: moving a card ticks its box or edits its line in the note.
//!
//! | Column | A checklist line | Another line |
//! |---|---|---|
//! | Waiting | `- [ ] Waiting: …` | `… Waiting: …` |
//! | To do | `- [ ] …` | `… To do: …` |
//! | Doing | `- [/] …` | `… Doing: …` |
//! | Done this week | `- [x] …` (in a note changed this week) | `… Done: …` |

use crate::day::Ymd;
use crate::vault::Vault;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    Waiting,
    Todo,
    Doing,
    Done,
}

pub const STATUSES: [Status; 4] = [Status::Waiting, Status::Todo, Status::Doing, Status::Done];

impl Status {
    pub fn title(self) -> &'static str {
        match self {
            Status::Waiting => "Waiting",
            Status::Todo => "To do",
            Status::Doing => "Doing",
            Status::Done => "Done this week",
        }
    }
    fn word(self) -> &'static str {
        match self {
            Status::Waiting => "Waiting:",
            Status::Todo => "To do:",
            Status::Doing => "Doing:",
            Status::Done => "Done:",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Card {
    pub note: usize,
    /// Its line in the note (from 0).
    pub line: usize,
    pub text: String,
    pub status: Status,
    /// The line's first tag, else the note's.
    pub tag: String,
    pub project: String,
    /// The log's day, else the day the note last changed.
    pub day: Ymd,
    pub task: bool,
}

const WORDS: [Status; 4] = [Status::Waiting, Status::Todo, Status::Doing, Status::Done];

fn status_word(line: &str) -> Option<(Status, usize)> {
    WORDS.iter().filter_map(|s| line.find(s.word()).map(|at| (*s, at))).min_by_key(|x| x.1)
}

fn clean(s: &str) -> String {
    let t = crate::md::plain(s.trim());
    let t = t.trim().trim_end_matches('.').trim();
    t.to_string()
}

/// Every card, newest notes first.
pub fn cards(v: &Vault, today: Ymd) -> Vec<Card> {
    let mut out = Vec::new();
    for (ni, n) in v.notes.iter().enumerate() {
        let day = n.log.unwrap_or_else(|| crate::day::log_day(n.mtime, 0));
        let recent = crate::day::between(day, today) <= 6;
        let mut in_code = false;
        for (li, line) in n.text.lines().enumerate() {
            if line.trim_start().starts_with("```") {
                in_code = !in_code;
                continue;
            }
            if in_code {
                continue;
            }
            let line_tag = crate::md::spans(line).into_iter().find_map(|s| if let crate::md::Span::Tag(t) = s { Some(t) } else { None });
            let tag = line_tag.or_else(|| n.tags.first().cloned()).unwrap_or_default();
            let mk = |text: String, status: Status, task: bool| Card { note: ni, line: li, text, status, tag: tag.clone(), project: n.project.clone(), day, task };
            if let Some((_, state, text)) = crate::md::task(line) {
                let (status, text) = match state {
                    'x' => (Status::Done, text),
                    '/' => (Status::Doing, text),
                    _ => match text.strip_prefix("Waiting:") {
                        Some(r) => (Status::Waiting, r.to_string()),
                        None => (Status::Todo, text),
                    },
                };
                if status == Status::Done && !recent {
                    continue;
                }
                let t = clean(&text);
                if !t.is_empty() {
                    out.push(mk(t, status, true));
                }
                continue;
            }
            if let Some((status, at)) = status_word(line) {
                if status == Status::Done && !recent {
                    continue;
                }
                let rest = &line[at + status.word().len()..];
                let t = clean(rest);
                if !t.is_empty() {
                    out.push(mk(t, status, false));
                }
            }
        }
    }
    out.sort_by(|a, b| b.day.cmp(&a.day).then(a.note.cmp(&b.note)).then(a.line.cmp(&b.line)));
    out
}

/// The card's line, moved to `to`: its box ticked or its word changed. The rest of the line stays as it was.
pub fn moved(line: &str, task: bool, to: Status) -> String {
    if task {
        let Some((indent, _, text)) = crate::md::task(line) else { return line.to_string() };
        let pad = &line[..indent];
        let mark = line.trim_start().chars().next().unwrap_or('-');
        let body = text.strip_prefix("Waiting:").map(|r| r.trim_start().to_string()).unwrap_or(text);
        return match to {
            Status::Waiting => format!("{pad}{mark} [ ] Waiting: {body}"),
            Status::Todo => format!("{pad}{mark} [ ] {body}"),
            Status::Doing => format!("{pad}{mark} [/] {body}"),
            Status::Done => format!("{pad}{mark} [x] {body}"),
        };
    }
    match status_word(line) {
        Some((s, at)) => format!("{}{}{}", &line[..at], to.word(), &line[at + s.word().len()..]),
        None => line.to_string(),
    }
}

/// A note's text with line `li` moved (the Board's drag, or X on a card).
pub fn move_in(text: &str, li: usize, task: bool, to: Status) -> String {
    let ends = text.ends_with('\n');
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    if let Some(l) = lines.get_mut(li) {
        *l = moved(l, task, to);
    }
    let mut t = lines.join("\n");
    if ends {
        t.push('\n');
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cards_from_checklists_and_waiting_lines() {
        let d = tempfile::tempdir().unwrap();
        crate::vault::sample(d.path());
        std::fs::write(d.path().join("hearth/assets/1.16-store-shots/README.md"), "# Shots\n- [/] labwc on Fedora: Firehold's memory #1.16\n```\n- [ ] not a card\n```\n").unwrap();
        let v = Vault::scan(d.path());
        let c = cards(&v, (2026, 11, 20));
        let find = |t: &str| c.iter().find(|x| x.text == t).unwrap_or_else(|| panic!("no card {t}: {c:?}"));
        assert_eq!(find("the owner's look").status, Status::Waiting);
        assert_eq!(find("Look at labwc's busy minute #1.16").status, Status::Todo);
        assert_eq!(find("Look at labwc's busy minute #1.16").tag, "1.16");
        assert_eq!(find("Push the Hearth Notes mockups #draft").status, Status::Done);
        assert_eq!(find("labwc on Fedora: Firehold's memory #1.16").status, Status::Doing);
        assert_eq!(find("Run the TS2010 freeware scripts on Dad's PC").project, "other");
        assert!(!c.iter().any(|x| x.text == "not a card"), "code blocks don't count");
    }

    #[test]
    fn moving_a_card_ticks_its_box_or_edits_its_line() {
        assert_eq!(moved("- [ ] Push the mockups #draft", true, Status::Done), "- [x] Push the mockups #draft");
        assert_eq!(moved("  - [x] Push", true, Status::Doing), "  - [/] Push");
        assert_eq!(moved("- [ ] Push", true, Status::Waiting), "- [ ] Waiting: Push");
        assert_eq!(moved("- [ ] Waiting: Push", true, Status::Todo), "- [ ] Push");
        assert_eq!(moved("- 03:04 Made: x. Waiting: the owner's look.", false, Status::Done), "- 03:04 Made: x. Done: the owner's look.");
        assert_eq!(move_in("a\n- [ ] b\n", 1, true, Status::Done), "a\n- [x] b\n");
    }
}
