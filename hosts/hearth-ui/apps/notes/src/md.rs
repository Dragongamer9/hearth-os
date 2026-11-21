//! Markdown as Hearth Notes shows it: each line (or fenced code block) is a block the live preview draws, and the
//! one the cursor is on is shown raw. Obsidian's extras are understood: `[[links]]` (with `|label` and `#heading`),
//! `![[embeds]]`, `#tags`, `- [ ]` / `- [x]` / `- [/]` checklists, front matter.

/// One block of a note (a line, or a fenced code block, or the front matter).
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Heading { level: u8, text: String },
    Para(String),
    /// A list item ("- ", "* ", "1. "): its indent (spaces), its mark, its text.
    Bullet { indent: usize, mark: String, text: String },
    /// A checklist item: ' ' to do, 'x' done, '/' doing.
    Task { indent: usize, state: char, text: String },
    Code { lang: String, lines: Vec<String> },
    /// `![[picture.png]]` or `![alt](picture.png)` on a line of its own.
    Image { target: String, alt: String },
    Quote(String),
    Rule,
    Blank,
    Front(Vec<(String, String)>),
}

/// A block and the lines it covers (first..=last, counted from 0).
#[derive(Debug, Clone, PartialEq)]
pub struct Parsed {
    pub block: Block,
    pub first: usize,
    pub last: usize,
}

fn fence(line: &str) -> Option<String> {
    let t = line.trim_start();
    t.strip_prefix("```").map(|r| r.trim().to_string())
}

/// `- [ ] x` → (indent, state, text)
pub fn task(line: &str) -> Option<(usize, char, String)> {
    let indent = line.len() - line.trim_start().len();
    let t = line.trim_start();
    let rest = t.strip_prefix("- [").or_else(|| t.strip_prefix("* [")).or_else(|| t.strip_prefix("+ ["))?;
    let mut ch = rest.chars();
    let state = ch.next()?;
    let after = ch.as_str();
    let after = after.strip_prefix(']')?;
    if !matches!(state, ' ' | 'x' | 'X' | '/') {
        return None;
    }
    if !(after.is_empty() || after.starts_with(' ')) {
        return None;
    }
    Some((indent, if state == 'X' { 'x' } else { state }, after.trim_start().to_string()))
}

/// `- x`, `* x`, `1. x` → (indent, mark, text)
pub fn bullet(line: &str) -> Option<(usize, String, String)> {
    let indent = line.len() - line.trim_start().len();
    let t = line.trim_start();
    for m in ["- ", "* ", "+ "] {
        if let Some(r) = t.strip_prefix(m) {
            return Some((indent, m.trim().to_string(), r.to_string()));
        }
    }
    if t == "-" || t == "*" {
        return Some((indent, t.to_string(), String::new()));
    }
    let digits: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
    if !digits.is_empty() && digits.len() < 4 {
        let r = &t[digits.len()..];
        for m in [". ", ") "] {
            if let Some(rest) = r.strip_prefix(m) {
                return Some((indent, format!("{digits}{}", m.trim()), rest.to_string()));
            }
        }
    }
    None
}

fn heading(line: &str) -> Option<(u8, String)> {
    let n = line.chars().take_while(|c| *c == '#').count();
    if (1..=6).contains(&n) {
        let r = &line[n..];
        if r.is_empty() || r.starts_with(' ') {
            return Some((n as u8, r.trim().trim_end_matches('#').trim().to_string()));
        }
    }
    None
}

/// A picture on a line of its own: `![[x.png]]`, `![[x.png|300]]`, `![alt](x.png)`.
pub fn image_line(line: &str) -> Option<(String, String)> {
    let t = line.trim();
    if let Some(r) = t.strip_prefix("![[").and_then(|r| r.strip_suffix("]]")) {
        let target = r.split('|').next().unwrap_or("").trim().to_string();
        if is_picture(&target) {
            return Some((target, String::new()));
        }
        return None;
    }
    if let Some(r) = t.strip_prefix("![") {
        let close = r.find("](")?;
        let alt = r[..close].to_string();
        let url = r[close + 2..].strip_suffix(')')?;
        let url = url.split_whitespace().next().unwrap_or("").to_string();
        if is_picture(&url) {
            return Some((url, alt));
        }
    }
    None
}

pub fn is_picture(p: &str) -> bool {
    let l = p.to_lowercase();
    [".png", ".jpg", ".jpeg", ".webp", ".gif", ".bmp"].iter().any(|e| l.ends_with(e))
}

fn rule(line: &str) -> bool {
    let t: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    t.len() >= 3 && (t.chars().all(|c| c == '-') || t.chars().all(|c| c == '*') || t.chars().all(|c| c == '_'))
}

/// The note's lines as blocks.
pub fn parse(text: &str) -> Vec<Parsed> {
    let lines: Vec<&str> = text.split('\n').collect();
    let lines: Vec<&str> = if text.ends_with('\n') { lines[..lines.len() - 1].to_vec() } else { lines };
    parse_lines(&lines.iter().map(|s| s.trim_end_matches('\r').to_string()).collect::<Vec<_>>())
}

pub fn parse_lines(lines: &[String]) -> Vec<Parsed> {
    let mut out = Vec::new();
    let mut i = 0;
    // Front matter: "---" on the first line, closed by "---" within 60 lines.
    if lines.first().map(|l| l.trim() == "---").unwrap_or(false) {
        if let Some(end) = lines.iter().skip(1).take(60).position(|l| l.trim() == "---") {
            let end = end + 1;
            let mut kv = Vec::new();
            for l in &lines[1..end] {
                if let Some((k, v)) = l.split_once(':') {
                    if !k.trim().is_empty() && !k.starts_with(' ') {
                        kv.push((k.trim().to_string(), v.trim().to_string()));
                    }
                }
            }
            out.push(Parsed { block: Block::Front(kv), first: 0, last: end });
            i = end + 1;
        }
    }
    while i < lines.len() {
        let l = &lines[i];
        if let Some(lang) = fence(l) {
            let mut j = i + 1;
            let mut body = Vec::new();
            while j < lines.len() && fence(&lines[j]).is_none() {
                body.push(lines[j].clone());
                j += 1;
            }
            let last = if j < lines.len() { j } else { lines.len() - 1 };
            out.push(Parsed { block: Block::Code { lang, lines: body }, first: i, last });
            i = last + 1;
            continue;
        }
        let block = line_block(l);
        out.push(Parsed { block, first: i, last: i });
        i += 1;
    }
    out
}

/// One line on its own (not in a code block).
pub fn line_block(l: &str) -> Block {
    if l.trim().is_empty() {
        Block::Blank
    } else if let Some((level, text)) = heading(l) {
        Block::Heading { level, text }
    } else if let Some((indent, state, text)) = task(l) {
        Block::Task { indent, state, text }
    } else if let Some((target, alt)) = image_line(l) {
        Block::Image { target, alt }
    } else if rule(l) {
        Block::Rule
    } else if let Some((indent, mark, text)) = bullet(l) {
        Block::Bullet { indent, mark, text }
    } else if let Some(q) = l.trim_start().strip_prefix('>') {
        Block::Quote(q.trim_start().to_string())
    } else {
        Block::Para(l.to_string())
    }
}

// ------------------------------------------------------------------ inline
#[derive(Debug, Clone, PartialEq)]
pub enum Span {
    Text(String),
    Bold(String),
    Italic(String),
    Strike(String),
    Code(String),
    /// `[[target|label]]`: the label shown (the target's name when there's none).
    Wiki { target: String, label: String },
    Embed(String),
    Url { label: String, url: String },
    Tag(String),
}

fn tag_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '/' | '.')
}

/// A tag at the start of `s` (after its '#'): letters, digits, `_ - / .`, not only digits, no trailing dot.
fn tag_at(s: &str) -> Option<String> {
    let t: String = s.chars().take_while(|c| tag_char(*c)).collect();
    let t = t.trim_end_matches(['.', '/', '-']).to_string();
    if t.is_empty() || t.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(t)
}

fn wiki_parts(inner: &str) -> (String, String) {
    let (t, label) = match inner.split_once('|') {
        Some((t, l)) => (t.trim().to_string(), l.trim().to_string()),
        None => (inner.trim().to_string(), String::new()),
    };
    let label = if label.is_empty() { t.clone() } else { label };
    (t, label)
}

/// A line's inline marks.
pub fn spans(s: &str) -> Vec<Span> {
    let mut out: Vec<Span> = Vec::new();
    let mut buf = String::new();
    let mut i = 0;
    let push_text = |out: &mut Vec<Span>, buf: &mut String| {
        if !buf.is_empty() {
            out.push(Span::Text(std::mem::take(buf)));
        }
    };
    while i < s.len() {
        let rest = &s[i..];
        let prev = if i == 0 { ' ' } else { s[..i].chars().next_back().unwrap_or(' ') };
        if rest.starts_with('`') {
            if let Some(end) = rest[1..].find('`') {
                push_text(&mut out, &mut buf);
                out.push(Span::Code(rest[1..1 + end].to_string()));
                i += end + 2;
                continue;
            }
        }
        if rest.starts_with("![[") {
            if let Some(end) = rest[3..].find("]]") {
                push_text(&mut out, &mut buf);
                out.push(Span::Embed(rest[3..3 + end].split('|').next().unwrap_or("").trim().to_string()));
                i += end + 5;
                continue;
            }
        }
        if rest.starts_with("[[") {
            if let Some(end) = rest[2..].find("]]") {
                push_text(&mut out, &mut buf);
                let (target, label) = wiki_parts(&rest[2..2 + end]);
                out.push(Span::Wiki { target, label });
                i += end + 4;
                continue;
            }
        }
        if rest.starts_with('[') {
            if let Some(close) = rest.find("](") {
                if let Some(end) = rest[close + 2..].find(')') {
                    let label = &rest[1..close];
                    if !label.contains('[') {
                        push_text(&mut out, &mut buf);
                        out.push(Span::Url { label: label.to_string(), url: rest[close + 2..close + 2 + end].to_string() });
                        i += close + 3 + end;
                        continue;
                    }
                }
            }
        }
        let mut matched = false;
        for (mark, kind) in [("**", 0u8), ("__", 0), ("~~", 2)] {
            if rest.starts_with(mark) && rest.len() > 4 {
                if let Some(end) = rest[2..].find(mark) {
                    if end > 0 {
                        push_text(&mut out, &mut buf);
                        let inner = rest[2..2 + end].to_string();
                        out.push(if kind == 0 { Span::Bold(inner) } else { Span::Strike(inner) });
                        i += end + 4;
                        matched = true;
                        break;
                    }
                }
            }
        }
        if matched {
            continue;
        }
        if (rest.starts_with('*') || (rest.starts_with('_') && !prev.is_alphanumeric())) && rest.len() > 2 && !rest[1..].starts_with(' ') {
            let m = &rest[..1];
            if let Some(end) = rest[1..].find(m) {
                let after = rest[1 + end + 1..].chars().next().unwrap_or(' ');
                if end > 0 && !rest[1..1 + end].ends_with(' ') && (m == "*" || !after.is_alphanumeric()) {
                    push_text(&mut out, &mut buf);
                    out.push(Span::Italic(rest[1..1 + end].to_string()));
                    i += end + 2;
                    continue;
                }
            }
        }
        if rest.starts_with('#') && (prev.is_whitespace() || prev == '(' || i == 0) {
            if let Some(tag) = tag_at(&rest[1..]) {
                push_text(&mut out, &mut buf);
                i += 1 + tag.len();
                out.push(Span::Tag(tag));
                continue;
            }
        }
        let c = rest.chars().next().unwrap_or(' ');
        buf.push(c);
        i += c.len_utf8();
    }
    push_text(&mut out, &mut buf);
    out
}

/// The text without its marks (search snippets, word counts).
pub fn plain(s: &str) -> String {
    spans(s)
        .into_iter()
        .map(|sp| match sp {
            Span::Text(t) | Span::Bold(t) | Span::Italic(t) | Span::Strike(t) | Span::Code(t) => t,
            Span::Wiki { label, .. } => label,
            Span::Embed(t) => t,
            Span::Url { label, .. } => label,
            Span::Tag(t) => format!("#{t}"),
        })
        .collect()
}

/// Lines outside code blocks and the front matter (where links and tags count).
fn prose_lines(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for p in parse(text) {
        match p.block {
            Block::Code { .. } | Block::Front(_) => {}
            _ => {
                if let Some(l) = text.split('\n').nth(p.first) {
                    out.push((p.first, l.trim_end_matches('\r').to_string()));
                }
            }
        }
    }
    out
}

/// Every `[[link]]` (target, embed?) outside code.
pub fn links(text: &str) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    for (_, l) in prose_lines(text) {
        for sp in spans(&l) {
            match sp {
                Span::Wiki { target, .. } => out.push((target, false)),
                Span::Embed(t) => out.push((t, true)),
                _ => {}
            }
        }
        if let Some((t, _)) = image_line(&l) {
            if !l.trim_start().starts_with("![[") {
                out.push((t, true));
            }
        }
    }
    out
}

/// Every #tag (once each, as first written), outside code and headings' leading #s; plus front matter `tags:`.
pub fn tags(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut add = |t: String| {
        if !t.is_empty() && !out.iter().any(|o| o.eq_ignore_ascii_case(&t)) {
            out.push(t);
        }
    };
    for p in parse(text) {
        if let Block::Front(kv) = &p.block {
            for (k, v) in kv {
                if k.eq_ignore_ascii_case("tags") {
                    for t in v.trim_matches(['[', ']']).split([',', ' ']) {
                        add(t.trim().trim_start_matches('#').trim_matches('"').to_string());
                    }
                }
            }
        }
    }
    for (_, l) in prose_lines(text) {
        let body = match heading(&l) {
            Some((_, t)) => t,
            None => l.clone(),
        };
        for sp in spans(&body) {
            if let Span::Tag(t) = sp {
                add(t);
            }
        }
    }
    out
}

/// The headings: (level, text, line).
pub fn headings(text: &str) -> Vec<(u8, String, usize)> {
    parse(text)
        .into_iter()
        .filter_map(|p| match p.block {
            Block::Heading { level, text } => Some((level, plain(&text), p.first)),
            _ => None,
        })
        .collect()
}

/// The checklist: (line, state, text).
pub fn tasks(text: &str) -> Vec<(usize, char, String)> {
    parse(text)
        .into_iter()
        .filter_map(|p| match p.block {
            Block::Task { state, text, .. } => Some((p.first, state, text)),
            _ => None,
        })
        .collect()
}

/// A `[[` still open at the end of what's typed: the part after it (the autocomplete's query).
pub fn open_link(s: &str) -> Option<String> {
    let at = s.rfind("[[")?;
    let after = &s[at + 2..];
    if after.contains("]]") || after.contains('\n') {
        return None;
    }
    Some(after.to_string())
}

/// `s` with its open `[[query` completed as `[[target]]`.
pub fn complete_link(s: &str, target: &str) -> String {
    match s.rfind("[[") {
        Some(at) => format!("{}[[{}]]", &s[..at], target),
        None => s.to_string(),
    }
}

/// The note's title: its first # heading, else its name.
pub fn title(text: &str, name: &str) -> String {
    headings(text).into_iter().find(|h| h.0 == 1).map(|h| h.1).unwrap_or_else(|| name.to_string())
}

pub fn words(text: &str) -> usize {
    let count = |s: &str| plain(s).split_whitespace().filter(|w| w.chars().any(|c| c.is_alphanumeric())).count();
    let mut n = 0;
    for p in parse(text) {
        n += match p.block {
            Block::Front(_) | Block::Blank | Block::Rule | Block::Image { .. } => 0,
            Block::Code { lines, .. } => lines.iter().map(|l| l.split_whitespace().count()).sum::<usize>(),
            Block::Heading { text, .. } | Block::Para(text) | Block::Quote(text) | Block::Bullet { text, .. } | Block::Task { text, .. } => count(&text),
        };
    }
    n
}

/// The note's properties: its front matter, else "Status:", "Belongs to:", "Release:" … in its first lines.
pub fn properties(text: &str) -> Vec<(String, String)> {
    for p in parse(text) {
        if let Block::Front(kv) = p.block {
            return kv;
        }
    }
    let mut out = Vec::new();
    for l in text.lines().take(14) {
        for part in l.split(" · ") {
            let part = part.trim();
            for key in ["Status", "Belongs to", "Release", "Tags", "Owner", "Approved"] {
                if let Some(v) = part.strip_prefix(key).and_then(|r| r.strip_prefix(':')) {
                    out.push((key.to_string(), plain(v.trim())));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_as_the_editor_shows_them() {
        let t = "---\ntags: [a, b]\n---\n# Hearth Store\n\nStatus: **approved**\n- [x] Home\n- [ ] Tried on the Ally\n- [/] Doing it\n- plain\n1. first\n![[hearth/assets/store-1.png]]\n```bash\ncd x\npython3 make.py\n```\n> quote\n---\n";
        let b = parse(t);
        assert!(matches!(&b[0].block, Block::Front(kv) if kv[0].0 == "tags"));
        assert_eq!(b[1].block, Block::Heading { level: 1, text: "Hearth Store".into() });
        assert_eq!(b[2].block, Block::Blank);
        assert!(matches!(&b[4].block, Block::Task { state: 'x', text, .. } if text == "Home"));
        assert!(matches!(&b[5].block, Block::Task { state: ' ', .. }));
        assert!(matches!(&b[6].block, Block::Task { state: '/', .. }));
        assert!(matches!(&b[7].block, Block::Bullet { mark, .. } if mark == "-"));
        assert!(matches!(&b[8].block, Block::Bullet { mark, .. } if mark == "1."));
        assert!(matches!(&b[9].block, Block::Image { target, .. } if target == "hearth/assets/store-1.png"));
        match &b[10].block {
            Block::Code { lang, lines } => {
                assert_eq!(lang, "bash");
                assert_eq!(lines.len(), 2);
                assert_eq!((b[10].first, b[10].last), (12, 15));
            }
            x => panic!("{x:?}"),
        }
        assert_eq!(b[11].block, Block::Quote("quote".into()));
        assert_eq!(b[12].block, Block::Rule);
    }

    #[test]
    fn inline_marks_links_and_tags() {
        let s = spans("See **bold** and *it* `code` [[hearth/assets/1.16-store-shots|shots]] #1.16 #mockup, [web](https://x.y) ~~old~~");
        assert!(s.contains(&Span::Bold("bold".into())));
        assert!(s.contains(&Span::Italic("it".into())));
        assert!(s.contains(&Span::Code("code".into())));
        assert!(s.contains(&Span::Wiki { target: "hearth/assets/1.16-store-shots".into(), label: "shots".into() }));
        assert!(s.contains(&Span::Tag("1.16".into())));
        assert!(s.contains(&Span::Tag("mockup".into())));
        assert!(s.contains(&Span::Url { label: "web".into(), url: "https://x.y".into() }));
        assert!(s.contains(&Span::Strike("old".into())));
        // Not tags: a heading's #, C#, a number, a # in the middle of a word.
        assert!(tags("# Title\nC# and #5 and a#b, but #real").eq(&vec!["real".to_string()]));
        assert_eq!(plain("a [[x|y]] **b**"), "a y b");
        assert_eq!(spans("snake_case_name"), vec![Span::Text("snake_case_name".into())]);
    }

    #[test]
    fn links_skip_code_and_autocomplete() {
        let t = "[[a]] and ![[pic.png]]\n```\n[[not-a-link]]\n```\n`[[nor-this]]` [[b#Part|B]]";
        assert_eq!(links(t), vec![("a".into(), false), ("pic.png".into(), true), ("b#Part".into(), false)]);
        assert_eq!(open_link("The pictures: [[hearth/ass"), Some("hearth/ass".into()));
        assert_eq!(open_link("done [[x]] more"), None);
        assert_eq!(complete_link("see [[hea", "hearth/assets/1.16-store-shots"), "see [[hearth/assets/1.16-store-shots]]");
        assert_eq!(headings("# A\ntext\n## B\n```\n# no\n```").iter().map(|h| h.1.clone()).collect::<Vec<_>>(), vec!["A", "B"]);
        assert_eq!(tasks("- [ ] one\n- [x] two\n").len(), 2);
        assert_eq!(title("intro\n# The title\n", "README"), "The title");
        assert_eq!(words("# Hi there\n- [x] **one** two"), 4);
        assert_eq!(properties("# X\n\nStatus: **approved** (Nov 19) · Belongs to: **1.16**\n").len(), 2);
    }
}
