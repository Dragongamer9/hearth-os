// vCard (.vcf) reading and writing: versions 2.1, 3.0 and 4.0 are read; 3.0 is written (what iCloud, Google,
// Nextcloud, Fastmail and phones all take). Properties Hearth Contacts doesn't edit are kept as they were
// (`extra`), so a contact that goes through Hearth Contacts and back loses nothing.
use base64::Engine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Name {
    pub family: String,
    pub given: String,
    pub additional: String,
    pub prefix: String,
    pub suffix: String,
}

/// A phone number, email address or web address with its kind ("cell", "home", "work", "other"...).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Typed {
    pub kind: String,
    pub value: String,
    pub pref: bool,
    /// The vCard group it came in ("item1" in Apple's cards, which ties it to an X-ABLabel), kept for writing back.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub group: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Address {
    pub kind: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub group: String,
    pub pobox: String,
    pub ext: String,
    pub street: String,
    pub locality: String,
    pub region: String,
    pub postcode: String,
    pub country: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Contact {
    pub uid: String,
    /// The name as shown (FN).
    pub display: String,
    pub name: Name,
    pub nickname: String,
    pub org: String,
    pub title: String,
    pub phones: Vec<Typed>,
    pub emails: Vec<Typed>,
    pub addresses: Vec<Address>,
    pub urls: Vec<Typed>,
    /// "YYYY-MM-DD", or "--MM-DD" without a year.
    pub birthday: String,
    /// A data: URL (data:image/jpeg;base64,...), or empty.
    pub photo: String,
    pub note: String,
    /// CATEGORIES.
    pub groups: Vec<String>,
    pub rev: String,
    /// Content lines this app doesn't edit, unfolded, written back as they came.
    pub extra: Vec<String>,
}

impl Contact {
    /// The name to show and sort by: FN, else the parts of N, else the organisation, an email or a phone number.
    pub fn shown_name(&self) -> String {
        if !self.display.trim().is_empty() { return self.display.trim().to_string(); }
        let n = &self.name;
        let parts: Vec<&str> = [n.prefix.as_str(), n.given.as_str(), n.additional.as_str(), n.family.as_str(), n.suffix.as_str()]
            .into_iter().map(str::trim).filter(|s| !s.is_empty()).collect();
        if !parts.is_empty() { return parts.join(" "); }
        if !self.org.trim().is_empty() { return self.org.trim().to_string(); }
        if let Some(e) = self.emails.iter().find(|e| !e.value.trim().is_empty()) { return e.value.trim().to_string(); }
        if let Some(p) = self.phones.iter().find(|p| !p.value.trim().is_empty()) { return p.value.trim().to_string(); }
        String::new()
    }
}

// ------------------------------------------------------------------ reading

#[derive(Debug, Clone)]
struct Line {
    group: String,
    name: String,
    params: Vec<(String, Vec<String>)>,
    value: String,
    raw: String,
}

impl Line {
    fn param(&self, key: &str) -> Option<&Vec<String>> {
        self.params.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v)
    }
    fn types(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (k, vs) in &self.params {
            if k.eq_ignore_ascii_case("TYPE") {
                for v in vs { for t in v.split(',') { let t = t.trim().to_ascii_lowercase(); if !t.is_empty() { out.push(t); } } }
            }
        }
        out
    }
    fn is_pref(&self) -> bool {
        self.types().iter().any(|t| t == "pref") || self.param("PREF").is_some()
    }
}

/// Lines folded onto the next (a line starting with a space or tab continues the one before) are joined. vCard
/// 2.1's quoted-printable values instead end a line with "=" to go on.
fn unfold(text: &str) -> Vec<String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut out: Vec<String> = Vec::new();
    let mut qp_open = false;
    for raw in text.split('\n') {
        let l = raw.strip_suffix('\r').unwrap_or(raw);
        if qp_open {
            if let Some(last) = out.last_mut() {
                last.pop(); // the soft line break's "="
                last.push_str(l);
                qp_open = l.ends_with('=');
                continue;
            }
        }
        if (l.starts_with(' ') || l.starts_with('\t')) && !out.is_empty() {
            out.last_mut().unwrap().push_str(&l[1..]);
        } else if !l.is_empty() {
            out.push(l.to_string());
        }
        let last = out.last().map(String::as_str).unwrap_or("");
        let head = last.split(':').next().unwrap_or("").to_ascii_uppercase();
        qp_open = head.contains("QUOTED-PRINTABLE") && last.ends_with('=');
    }
    out
}

fn parse_line(l: &str) -> Option<Line> {
    // The name and parameters end at the first ':' outside double quotes.
    let mut in_q = false;
    let mut split = None;
    for (i, c) in l.char_indices() {
        match c {
            '"' => in_q = !in_q,
            ':' if !in_q => { split = Some(i); break; }
            _ => {}
        }
    }
    let split = split?;
    let (head, value) = (&l[..split], &l[split + 1..]);
    let mut parts = Vec::new();
    let mut cur = String::new();
    in_q = false;
    for c in head.chars() {
        match c {
            '"' => { in_q = !in_q; }
            ';' if !in_q => { parts.push(std::mem::take(&mut cur)); }
            _ => cur.push(c),
        }
    }
    parts.push(cur);
    let full = parts.remove(0);
    let (group, name) = match full.rsplit_once('.') { Some((g, n)) => (g.to_string(), n.to_ascii_uppercase()), None => (String::new(), full.to_ascii_uppercase()) };
    let mut params = Vec::new();
    for p in parts {
        if p.is_empty() { continue; }
        match p.split_once('=') {
            Some((k, v)) => params.push((k.trim().to_ascii_uppercase(), v.split(',').map(|s| s.to_string()).collect())),
            // vCard 2.1: "TEL;CELL;HOME:" or "PHOTO;JPEG;BASE64:".
            None => {
                let up = p.trim().to_ascii_uppercase();
                match up.as_str() {
                    "BASE64" | "B" | "QUOTED-PRINTABLE" | "8BIT" => params.push(("ENCODING".into(), vec![up])),
                    _ => params.push(("TYPE".into(), vec![p.trim().to_string()])),
                }
            }
        }
    }
    Some(Line { group, name, params, value: value.to_string(), raw: l.to_string() })
}

fn qp_decode(s: &str, charset: Option<&str>) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'=' && i + 2 < b.len() {
            let h = std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok());
            if let Some(v) = h { out.push(v); i += 3; continue; }
        }
        out.push(b[i]);
        i += 1;
    }
    match charset.map(|c| c.to_ascii_uppercase()) {
        Some(c) if c.contains("8859-1") || c.contains("1252") => out.iter().map(|&c| c as char).collect(),
        _ => String::from_utf8_lossy(&out).into_owned(),
    }
}

/// Text values: "\n" is a new line, "\," "\;" "\\" are the characters themselves.
fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('n') | Some('N') => out.push('\n'),
                Some(o) => out.push(o),
                None => out.push('\\'),
            }
        } else { out.push(c); }
    }
    out
}

/// Splits on `sep` where it isn't escaped, then unescapes each piece.
fn split_escaped(s: &str, sep: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            cur.push(c);
            if let Some(n) = it.next() { cur.push(n); }
        } else if c == sep {
            out.push(unescape(&cur));
            cur.clear();
        } else { cur.push(c); }
    }
    out.push(unescape(&cur));
    out
}

fn decoded_value(l: &Line) -> String {
    let enc = l.param("ENCODING").and_then(|v| v.first()).map(|s| s.to_ascii_uppercase());
    if enc.as_deref() == Some("QUOTED-PRINTABLE") {
        let cs = l.param("CHARSET").and_then(|v| v.first()).map(String::as_str);
        return qp_decode(&l.value, cs);
    }
    l.value.clone()
}

fn kind_of(types: &[String], allowed: &[&str], fallback: &str) -> String {
    for t in types {
        let t = t.as_str();
        let t = match t { "mobile" | "iphone" => "cell", "internet" | "pref" | "voice" | "x400" => continue, o => o };
        if allowed.contains(&t) { return t.to_string(); }
    }
    fallback.to_string()
}

/// "19900115", "1990-01-15", "--0115", "--01-15", "1990-01-15T00:00:00Z" → "1990-01-15" or "--01-15".
pub fn normalize_birthday(s: &str) -> String {
    let s = s.trim();
    let date = s.split('T').next().unwrap_or("");
    if let Some(rest) = date.strip_prefix("--") {
        let d: String = rest.chars().filter(|c| c.is_ascii_digit()).collect();
        if d.len() == 4 { return format!("--{}-{}", &d[..2], &d[2..]); }
        return String::new();
    }
    let d: String = date.chars().filter(|c| c.is_ascii_digit()).collect();
    if d.len() == 8 { return format!("{}-{}-{}", &d[..4], &d[4..6], &d[6..]); }
    String::new()
}

fn mime_for(t: &str) -> String {
    let t = t.trim().to_ascii_lowercase();
    if t.contains('/') { return t; }
    match t.as_str() { "png" => "image/png", "gif" => "image/gif", "webp" => "image/webp", _ => "image/jpeg" }.to_string()
}

fn photo_of(l: &Line) -> Option<String> {
    let v = l.value.trim();
    if v.starts_with("data:") { return Some(v.to_string()); }
    let enc = l.param("ENCODING").and_then(|e| e.first()).map(|s| s.to_ascii_uppercase());
    if matches!(enc.as_deref(), Some("B") | Some("BASE64")) {
        let ty = l.types().into_iter().next().unwrap_or_else(|| "jpeg".into());
        let clean: String = v.chars().filter(|c| !c.is_whitespace()).collect();
        // Checked, so a broken photo doesn't become a broken data: URL.
        if base64::engine::general_purpose::STANDARD.decode(&clean).is_err() { return None; }
        return Some(format!("data:{};base64,{}", mime_for(&ty), clean));
    }
    None
}

/// Every contact in a .vcf file (a file can hold one or many).
pub fn parse(text: &str) -> Vec<Contact> {
    let mut out = Vec::new();
    let mut cur: Option<Contact> = None;
    let mut depth = 0;
    for raw in unfold(text) {
        let Some(l) = parse_line(&raw) else { continue };
        if l.name == "BEGIN" && l.value.trim().eq_ignore_ascii_case("VCARD") {
            depth += 1;
            if depth == 1 { cur = Some(Contact::default()); }
            continue;
        }
        if l.name == "END" && l.value.trim().eq_ignore_ascii_case("VCARD") {
            depth -= 1;
            if depth == 0 { if let Some(c) = cur.take() { out.push(c); } }
            if depth < 0 { depth = 0; }
            continue;
        }
        if depth != 1 { continue; }
        let Some(c) = cur.as_mut() else { continue };
        apply(c, &l);
    }
    out
}

fn apply(c: &mut Contact, l: &Line) {
    let v = decoded_value(l);
    let types = l.types();
    match l.name.as_str() {
        "VERSION" | "PRODID" => {}
        "UID" => c.uid = unescape(v.trim()),
        "FN" => c.display = unescape(&v),
        "N" => {
            let p = split_escaped(&v, ';');
            let g = |i: usize| p.get(i).cloned().unwrap_or_default();
            c.name = Name { family: g(0), given: g(1), additional: g(2), prefix: g(3), suffix: g(4) };
        }
        "NICKNAME" => c.nickname = split_escaped(&v, ',').join(", "),
        "ORG" => c.org = split_escaped(&v, ';').into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(", "),
        "TITLE" => c.title = unescape(&v),
        "TEL" => {
            let val = unescape(v.trim().trim_start_matches("tel:"));
            c.phones.push(Typed { kind: kind_of(&types, &["cell", "home", "work", "fax", "pager", "main", "other"], "other"), value: val, pref: l.is_pref(), group: l.group.clone() });
        }
        "EMAIL" => c.emails.push(Typed { kind: kind_of(&types, &["home", "work", "other"], "other"), value: unescape(v.trim()), pref: l.is_pref(), group: l.group.clone() }),
        "URL" => c.urls.push(Typed { kind: kind_of(&types, &["home", "work", "other"], "other"), value: unescape(v.trim()), pref: l.is_pref(), group: l.group.clone() }),
        "ADR" => {
            let p = split_escaped(&v, ';');
            let g = |i: usize| p.get(i).cloned().unwrap_or_default();
            c.addresses.push(Address { kind: kind_of(&types, &["home", "work", "other"], "other"), group: l.group.clone(), pobox: g(0), ext: g(1), street: g(2), locality: g(3), region: g(4), postcode: g(5), country: g(6) });
        }
        "BDAY" => {
            let mut b = normalize_birthday(&v);
            // Apple writes a birthday without a year as that year (X-APPLE-OMIT-YEAR=1604:1604-02-24).
            if let Some(y) = l.param("X-APPLE-OMIT-YEAR").and_then(|y| y.first()) { if b.len() == 10 && b.starts_with(y.as_str()) { b = format!("--{}", &b[5..]); } }
            if b.is_empty() { c.extra.push(l.raw.clone()); } else { c.birthday = b; }
        }
        "PHOTO" => match photo_of(l) {
            Some(p) if c.photo.is_empty() => c.photo = p,
            _ => c.extra.push(l.raw.clone()),
        },
        "NOTE" => { if !c.note.is_empty() { c.note.push('\n'); } c.note.push_str(&unescape(&v).replace("\r\n", "\n")); }
        "CATEGORIES" => for g in split_escaped(&v, ',') { let g = g.trim().to_string(); if !g.is_empty() && !c.groups.contains(&g) { c.groups.push(g); } },
        "REV" => c.rev = v.trim().to_string(),
        // Anything else (and labels for grouped items, such as Apple's item1.X-ABLabel) is kept as it was.
        _ => c.extra.push(l.raw.clone()),
    }
}

// ------------------------------------------------------------------ writing

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            ';' => out.push_str("\\;"),
            ',' => out.push_str("\\,"),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            o => out.push(o),
        }
    }
    out
}

/// Folds a content line at 75 bytes (never inside a UTF-8 character), with CRLF line ends.
fn fold(line: &str, out: &mut String) {
    let mut width = 0;
    for ch in line.chars() {
        let n = ch.len_utf8();
        if width + n > 75 {
            out.push_str("\r\n ");
            width = 1;
        }
        out.push(ch);
        width += n;
    }
    out.push_str("\r\n");
}

fn grp(g: &str) -> String { if g.is_empty() { String::new() } else { format!("{g}.") } }

fn type_param(kind: &str, pref: bool, extra: &[&str]) -> String {
    let mut t: Vec<String> = extra.iter().map(|s| s.to_string()).collect();
    let k = kind.trim().to_ascii_lowercase();
    if !k.is_empty() && k != "other" { t.push(k); }
    if pref { t.push("pref".into()); }
    if t.is_empty() { String::new() } else { format!(";TYPE={}", t.join(",")) }
}

/// One contact as a vCard 3.0.
pub fn write(c: &Contact) -> String {
    let mut lines: Vec<String> = vec!["BEGIN:VCARD".into(), "VERSION:3.0".into(), "PRODID:-//Hearth OS//Hearth Contacts//EN".into()];
    if !c.uid.is_empty() { lines.push(format!("UID:{}", c.uid)); }
    lines.push(format!("FN:{}", escape(&c.shown_name())));
    let n = &c.name;
    lines.push(format!("N:{};{};{};{};{}", escape(&n.family), escape(&n.given), escape(&n.additional), escape(&n.prefix), escape(&n.suffix)));
    if !c.nickname.trim().is_empty() {
        lines.push(format!("NICKNAME:{}", c.nickname.split(',').map(|s| escape(s.trim())).collect::<Vec<_>>().join(",")));
    }
    if !c.org.trim().is_empty() { lines.push(format!("ORG:{}", escape(c.org.trim()))); }
    if !c.title.trim().is_empty() { lines.push(format!("TITLE:{}", escape(c.title.trim()))); }
    for p in c.phones.iter().filter(|p| !p.value.trim().is_empty()) {
        let extra: &[&str] = if p.kind == "cell" || p.kind == "fax" || p.kind == "pager" { &[] } else { &["voice"] };
        lines.push(format!("{}TEL{}:{}", grp(&p.group), type_param(&p.kind, p.pref, extra), escape(p.value.trim())));
    }
    for e in c.emails.iter().filter(|e| !e.value.trim().is_empty()) {
        lines.push(format!("{}EMAIL{}:{}", grp(&e.group), type_param(&e.kind, e.pref, &["internet"]), escape(e.value.trim())));
    }
    for a in &c.addresses {
        let parts = [&a.pobox, &a.ext, &a.street, &a.locality, &a.region, &a.postcode, &a.country];
        if parts.iter().all(|p| p.trim().is_empty()) { continue; }
        lines.push(format!("{}ADR{}:{}", grp(&a.group), type_param(&a.kind, false, &[]), parts.iter().map(|p| escape(p)).collect::<Vec<_>>().join(";")));
    }
    for u in c.urls.iter().filter(|u| !u.value.trim().is_empty()) {
        lines.push(format!("{}URL{}:{}", grp(&u.group), type_param(&u.kind, u.pref, &[]), u.value.trim()));
    }
    if !c.birthday.is_empty() {
        // vCard 3.0 has no year-less date; "--MM-DD" is what Apple and Google write anyway.
        lines.push(format!("BDAY:{}", c.birthday));
    }
    if let Some(rest) = c.photo.strip_prefix("data:") {
        if let Some((mime, data)) = rest.split_once(";base64,") {
            let ty = mime.rsplit('/').next().unwrap_or("jpeg").to_ascii_uppercase();
            lines.push(format!("PHOTO;ENCODING=b;TYPE={}:{}", ty, data));
        }
    }
    if !c.note.is_empty() { lines.push(format!("NOTE:{}", escape(&c.note))); }
    if !c.groups.is_empty() { lines.push(format!("CATEGORIES:{}", c.groups.iter().map(|g| escape(g)).collect::<Vec<_>>().join(","))); }
    lines.extend(c.extra.iter().cloned());
    if !c.rev.is_empty() { lines.push(format!("REV:{}", c.rev)); }
    lines.push("END:VCARD".into());
    let mut out = String::new();
    for l in lines { fold(&l, &mut out); }
    out
}

/// Several contacts in one .vcf file.
pub fn write_all<'a>(cs: impl IntoIterator<Item = &'a Contact>) -> String {
    cs.into_iter().map(write).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unfolds_and_reads_a_3_0_card() {
        let t = "BEGIN:VCARD\r\nVERSION:3.0\r\nUID:abc-1\r\nFN:Ada Lovelace\r\nN:Lovelace;Ada;;Countess;\r\nTEL;TYPE=CELL,pref:+44 20 1234\r\nEMAIL;TYPE=INTERNET,HOME:ada@example.org\r\nADR;TYPE=WORK:;;12 St James\\, Square;London;;SW1;UK\r\nNOTE:Line one\\nLine two with a long text that goes on and on and on so that it\r\n  is folded\r\nCATEGORIES:Friends,Maths\r\nBDAY:1815-12-10\r\nX-CUSTOM;FOO=bar:kept\r\nEND:VCARD\r\n";
        let cs = parse(t);
        assert_eq!(cs.len(), 1);
        let c = &cs[0];
        assert_eq!(c.uid, "abc-1");
        assert_eq!(c.name.family, "Lovelace");
        assert_eq!(c.name.prefix, "Countess");
        assert_eq!(c.phones[0], Typed { kind: "cell".into(), value: "+44 20 1234".into(), pref: true, group: String::new() });
        assert_eq!(c.emails[0].kind, "home");
        assert_eq!(c.addresses[0].street, "12 St James, Square");
        assert_eq!(c.addresses[0].kind, "work");
        assert_eq!(c.note, "Line one\nLine two with a long text that goes on and on and on so that it is folded");
        assert_eq!(c.groups, vec!["Friends", "Maths"]);
        assert_eq!(c.birthday, "1815-12-10");
        assert_eq!(c.extra, vec!["X-CUSTOM;FOO=bar:kept"]);
    }

    #[test]
    fn reads_2_1_quoted_printable_and_bare_types() {
        let t = "BEGIN:VCARD\nVERSION:2.1\nN;CHARSET=UTF-8;ENCODING=QUOTED-PRINTABLE:M=C3=BCller;J=C3=BCrgen\nTEL;CELL;HOME:0170 123\nNOTE;ENCODING=QUOTED-PRINTABLE:first=0D=0Asecond =\nthird\nEND:VCARD\n";
        let c = &parse(t)[0];
        assert_eq!(c.name.family, "Müller");
        assert_eq!(c.name.given, "Jürgen");
        assert_eq!(c.phones[0].kind, "cell");
        assert_eq!(c.note, "first\nsecond third");
    }

    #[test]
    fn reads_4_0_photos_and_yearless_birthdays() {
        let t = "BEGIN:VCARD\nVERSION:4.0\nFN:Bo\nPHOTO:data:image/png;base64,iVBORw0KGgo=\nBDAY:--0315\nTEL;VALUE=uri;TYPE=work:tel:+1-555-0100\nEND:VCARD\n";
        let c = &parse(t)[0];
        assert_eq!(c.photo, "data:image/png;base64,iVBORw0KGgo=");
        assert_eq!(c.birthday, "--03-15");
        assert_eq!(c.phones[0].value, "+1-555-0100");
        assert_eq!(c.phones[0].kind, "work");
    }

    #[test]
    fn several_cards_in_one_file() {
        let t = "BEGIN:VCARD\nVERSION:3.0\nFN:A\nEND:VCARD\nBEGIN:VCARD\nVERSION:3.0\nFN:B\nEND:VCARD\n";
        let cs = parse(t);
        assert_eq!(cs.iter().map(|c| c.display.as_str()).collect::<Vec<_>>(), vec!["A", "B"]);
    }

    #[test]
    fn round_trip_keeps_everything() {
        let c = Contact {
            uid: "u-1".into(), display: "Zoë O'Brien; Jr".into(),
            name: Name { family: "O'Brien".into(), given: "Zoë".into(), suffix: "Jr".into(), ..Default::default() },
            nickname: "Zo".into(), org: "Hearth, Inc".into(), title: "Boss".into(),
            phones: vec![Typed { kind: "cell".into(), value: "+353 1 555".into(), pref: true, group: String::new() }, Typed { kind: "work".into(), value: "01 222".into(), pref: false, group: String::new() }],
            emails: vec![Typed { kind: "work".into(), value: "zo@example.ie".into(), pref: false, group: String::new() }],
            addresses: vec![Address { kind: "home".into(), street: "1 Long Road;\nFlat 2".into(), locality: "Dublin".into(), country: "Ireland".into(), ..Default::default() }],
            urls: vec![Typed { kind: "other".into(), value: "https://example.ie/a,b".into(), pref: false, group: String::new() }],
            birthday: "--07-04".into(), photo: format!("data:image/jpeg;base64,{}", "QUJD".repeat(60)),
            note: "Multi\nline, with; marks \\ and ünïcödé that is long enough to be folded somewhere in the middle of ✓ characters".into(),
            groups: vec!["Family".into(), "Work, old".into()], rev: "20261022T101010Z".into(),
            extra: vec!["X-SOCIALPROFILE;TYPE=twitter:https://x.example/zo".into(), "item1.X-ABLABEL:_$!<Anniversary>!$_".into()],
        };
        let text = write(&c);
        for l in text.split("\r\n") { assert!(l.len() <= 75, "line too long: {l}"); }
        let back = parse(&text);
        assert_eq!(back.len(), 1);
        assert_eq!(back[0], c);
        // And again: writing what was read gives the same text.
        assert_eq!(write(&back[0]), text);
    }

    #[test]
    fn shown_name_falls_back() {
        let mut c = Contact::default();
        c.emails.push(Typed { kind: "home".into(), value: "x@y.z".into(), pref: false, group: String::new() });
        assert_eq!(c.shown_name(), "x@y.z");
        c.org = "Org".into();
        assert_eq!(c.shown_name(), "Org");
        c.name.given = "Gi".into();
        assert_eq!(c.shown_name(), "Gi");
    }
}
