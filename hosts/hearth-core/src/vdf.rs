//! Valve's text KeyValues format (.vdf / .acf): Steam's library folders, app manifests, login users and
//! per-user settings. Keys match case-insensitively, the way Steam reads them. Port of library/vdf.js.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Kv {
    Str(String),
    Obj(Vec<(String, Kv)>),
}

impl Kv {
    /// Follows a key path, case-insensitively.
    pub fn get(&self, path: &[&str]) -> Option<&Kv> {
        let mut cur = self;
        for k in path {
            match cur {
                Kv::Obj(items) => cur = &items.iter().find(|(key, _)| key.eq_ignore_ascii_case(k))?.1,
                Kv::Str(_) => return None,
            }
        }
        Some(cur)
    }
    pub fn str(&self, path: &[&str]) -> Option<&str> {
        match self.get(path)? { Kv::Str(s) => Some(s), _ => None }
    }
    pub fn obj(&self, path: &[&str]) -> Option<&Vec<(String, Kv)>> {
        match self.get(path)? { Kv::Obj(o) => Some(o), _ => None }
    }
    /// A number value ("123"), 0 when missing or not a number.
    pub fn num(&self, path: &[&str]) -> i64 { self.str(path).and_then(|s| s.trim().parse().ok()).unwrap_or(0) }
    pub fn to_map(&self) -> BTreeMap<String, Kv> {
        match self { Kv::Obj(o) => o.iter().cloned().collect(), _ => BTreeMap::new() }
    }
}

enum Tok { Text(String), Open, Close }

struct Lexer<'a> { s: &'a [u8], i: usize }

impl<'a> Lexer<'a> {
    fn next(&mut self) -> Option<Tok> {
        let s = self.s;
        while self.i < s.len() {
            let c = s[self.i];
            if c.is_ascii_whitespace() { self.i += 1; continue; }
            if c == b'/' && s.get(self.i + 1) == Some(&b'/') {
                while self.i < s.len() && s[self.i] != b'\n' { self.i += 1; }
                continue;
            }
            if c == b'{' { self.i += 1; return Some(Tok::Open); }
            if c == b'}' { self.i += 1; return Some(Tok::Close); }
            let mut out = Vec::new();
            if c == b'"' {
                self.i += 1;
                while self.i < s.len() && s[self.i] != b'"' {
                    if s[self.i] == b'\\' && self.i + 1 < s.len() {
                        let n = s[self.i + 1];
                        out.push(match n { b'n' => b'\n', b't' => b'\t', other => other });
                        self.i += 2;
                    } else { out.push(s[self.i]); self.i += 1; }
                }
                self.i += 1; // closing quote
            } else {
                // Unquoted token: up to whitespace or a brace.
                while self.i < s.len() && !s[self.i].is_ascii_whitespace() && s[self.i] != b'{' && s[self.i] != b'}' { out.push(s[self.i]); self.i += 1; }
            }
            return Some(Tok::Text(String::from_utf8_lossy(&out).into_owned()));
        }
        None
    }
}

fn object(lx: &mut Lexer) -> Vec<(String, Kv)> {
    let mut items = Vec::new();
    loop {
        let key = match lx.next() { Some(Tok::Text(t)) => t, Some(Tok::Open) => continue, _ => return items };
        match lx.next() {
            Some(Tok::Open) => { let o = object(lx); items.push((key, Kv::Obj(o))); }
            Some(Tok::Text(v)) => items.push((key, Kv::Str(v))),
            _ => return items,
        }
    }
}

pub fn parse(text: &str) -> Kv {
    let mut lx = Lexer { s: text.as_bytes(), i: 0 };
    Kv::Obj(object(&mut lx))
}

pub fn load(path: &std::path::Path) -> Option<Kv> {
    std::fs::read(path).ok().map(|b| parse(&String::from_utf8_lossy(&b)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_steam_files() {
        let kv = parse(r#"
            // a comment
            "libraryfolders" { "0" { "path" "/home/sam/.local/share/Steam" "apps" { "620" "123" } }
                               "1" { "path" "/mnt/games/SteamLibrary" } }
            "AppState" { "appid" "620" "name" "Portal 2" "StateFlags" "4" "esc" "a\"b\\c" }"#);
        assert_eq!(kv.str(&["LibraryFolders", "1", "PATH"]), Some("/mnt/games/SteamLibrary"));
        assert_eq!(kv.num(&["appstate", "StateFlags"]), 4);
        assert_eq!(kv.str(&["AppState", "esc"]), Some("a\"b\\c"));
        assert_eq!(kv.str(&["AppState", "missing"]), None);
        assert_eq!(kv.obj(&["libraryfolders"]).map(|o| o.len()), Some(2));
    }
}
