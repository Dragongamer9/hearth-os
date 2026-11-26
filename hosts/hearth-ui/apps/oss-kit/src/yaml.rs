//! Lutris' YAML (and any simple YAML settings file), read and changed one line at a time: every other line stays
//! exactly as it was written. Two or three levels of keys (section, key, a map's key), scalars quoted when needed.

struct Line {
    indent: usize,
    key: Option<String>,
    value: String,
}

fn lines_of(text: &str) -> Vec<Line> {
    text.lines()
        .map(|l| {
            let indent = l.len() - l.trim_start_matches(' ').len();
            let body = l.trim();
            let key = if body.is_empty() || body.starts_with('#') || body.starts_with("- ") || body == "-" {
                None
            } else {
                split_key(body).map(|(k, _)| k)
            };
            let value = split_key(body).map(|(_, v)| v).unwrap_or_default();
            Line { indent, key, value }
        })
        .collect()
}

/// "key: value" → (key, value); quoted keys too.
fn split_key(body: &str) -> Option<(String, String)> {
    let (k, v) = if let Some(r) = body.strip_prefix('\'') {
        let end = r.find("':")?;
        (r[..end].replace("''", "'"), r[end + 2..].trim().to_string())
    } else if let Some(r) = body.strip_prefix('"') {
        let end = r.find("\":")?;
        (r[..end].to_string(), r[end + 2..].trim().to_string())
    } else {
        let at = body.find(": ").or_else(|| body.strip_suffix(':').map(|s| s.len()))?;
        (body[..at].to_string(), body.get(at + 1..).unwrap_or("").trim().to_string())
    };
    Some((k, v))
}

/// A YAML scalar as written → its text (quotes taken off).
pub fn unquote(v: &str) -> String {
    let v = v.trim();
    if v.len() >= 2 && v.starts_with('\'') && v.ends_with('\'') {
        v[1..v.len() - 1].replace("''", "'")
    } else if v.len() >= 2 && v.starts_with('"') && v.ends_with('"') {
        v[1..v.len() - 1].replace("\\\"", "\"").replace("\\\\", "\\")
    } else {
        v.to_string()
    }
}

/// A text as a YAML scalar (quoted when it has to be).
pub fn quote(s: &str) -> String {
    let plain = !s.is_empty()
        && !s.contains(": ")
        && !s.contains(" #")
        && !s.ends_with(':')
        && !s.starts_with(|c: char| "!&*-?{}[]|>'\"%@`#,".contains(c) || c.is_whitespace())
        && !s.ends_with(char::is_whitespace)
        && !matches!(s.to_lowercase().as_str(), "true" | "false" | "yes" | "no" | "null" | "on" | "off" | "~");
    if plain {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "''"))
    }
}

/// Where each line sits: the keys above it, by indent.
fn paths(ls: &[Line]) -> Vec<Vec<String>> {
    let mut stack: Vec<(usize, String)> = Vec::new();
    let mut out = Vec::new();
    for l in ls {
        match &l.key {
            Some(k) => {
                while stack.last().map(|s| s.0 >= l.indent).unwrap_or(false) {
                    stack.pop();
                }
                stack.push((l.indent, k.clone()));
                out.push(stack.iter().map(|s| s.1.clone()).collect());
            }
            None => out.push(Vec::new()),
        }
    }
    out
}

fn find(ls: &[Line], ps: &[Vec<String>], path: &[&str]) -> Option<usize> {
    (0..ls.len()).find(|&i| ps[i].len() == path.len() && ps[i].iter().zip(path).all(|(a, b)| a == b))
}

/// The last line of `i`'s block (its children, if any).
fn block_end(ls: &[Line], i: usize) -> usize {
    let mut e = i;
    for (j, l) in ls.iter().enumerate().skip(i + 1) {
        let blank = l.key.is_none() && l.value.is_empty() && l.indent == 0;
        if blank {
            continue;
        }
        if l.indent <= ls[i].indent && l.key.is_some() {
            break;
        }
        if l.indent > ls[i].indent || (l.key.is_none() && l.indent >= ls[i].indent) {
            e = j;
        } else {
            break;
        }
    }
    e
}

/// The value at `path` (section.key or section.map.key), quotes taken off.
pub fn yget(text: &str, path: &[&str]) -> Option<String> {
    let ls = lines_of(text);
    let ps = paths(&ls);
    let i = find(&ls, &ps, path)?;
    let v = &ls[i].value;
    (!v.is_empty()).then(|| unquote(v))
}

/// The keys and values under `path` (system.env).
pub fn ymap(text: &str, path: &[&str]) -> Vec<(String, String)> {
    let ls = lines_of(text);
    let ps = paths(&ls);
    let mut v = Vec::new();
    for (i, p) in ps.iter().enumerate() {
        if p.len() == path.len() + 1 && p.iter().zip(path).all(|(a, b)| a == b) {
            v.push((p[path.len()].clone(), unquote(&ls[i].value)));
        }
    }
    v
}

/// The items of a list under `path` ("- item" lines), quotes taken off.
pub fn ylist(text: &str, path: &[&str]) -> Vec<String> {
    let ls = lines_of(text);
    let ps = paths(&ls);
    let Some(i) = find(&ls, &ps, path) else { return Vec::new() };
    let raw: Vec<&str> = text.lines().collect();
    let mut v = Vec::new();
    for (j, l) in raw.iter().enumerate().skip(i + 1) {
        let body = l.trim();
        if body.is_empty() {
            continue;
        }
        let ind = l.len() - l.trim_start_matches(' ').len();
        if let Some(item) = body.strip_prefix("- ") {
            if ind >= ls[i].indent {
                v.push(unquote(item));
                continue;
            }
        }
        if ls[j].key.is_some() && ls[j].indent <= ls[i].indent {
            break;
        }
        if ind <= ls[i].indent {
            break;
        }
    }
    v
}

/// `text` with `path` set to `value` (already a YAML scalar: use [`quote`]), or taken out (None). Every other line
/// stays exactly as it was; missing sections and maps are added where Lutris would have them.
pub fn yset(text: &str, path: &[&str], value: Option<&str>) -> String {
    let raw: Vec<&str> = text.lines().collect();
    let ls = lines_of(text);
    let ps = paths(&ls);
    let mut out: Vec<String> = raw.iter().map(|s| s.to_string()).collect();
    let nl = if text.ends_with('\n') || text.is_empty() { "\n" } else { "" };
    let join = |v: Vec<String>| -> String {
        let s = v.join("\n");
        if s.is_empty() { s } else { format!("{s}{nl}") }
    };
    if let Some(i) = find(&ls, &ps, path) {
        let key = path[path.len() - 1];
        match value {
            Some(v) => {
                out[i] = format!("{}{}: {}", " ".repeat(ls[i].indent), quote_key(key), v);
                let e = block_end(&ls, i);
                if e > i {
                    out.drain(i + 1..=e);
                }
            }
            None => {
                let e = block_end(&ls, i);
                out.drain(i..=e);
                // a map left with nothing in it: taken out too (Lutris writes none)
                if path.len() > 1 {
                    let t = join(out.clone());
                    if ymap(&t, &path[..path.len() - 1]).is_empty() && yget(&t, &path[..path.len() - 1]).is_none() {
                        return yset(&t, &path[..path.len() - 1], None);
                    }
                    return t;
                }
            }
        }
        return join(out);
    }
    let Some(v) = value else { return text.to_string() };
    // the deepest part of the path that's there
    let mut depth = 0;
    let mut at: Option<usize> = None;
    for k in (1..path.len()).rev() {
        if let Some(i) = find(&ls, &ps, &path[..k]) {
            depth = k;
            at = Some(i);
            break;
        }
    }
    let mut add: Vec<String> = Vec::new();
    let base = at.map(|i| ls[i].indent + 2).unwrap_or(0);
    for (n, k) in path.iter().enumerate().skip(depth) {
        let ind = " ".repeat(base + 2 * (n - depth));
        if n + 1 == path.len() {
            add.push(format!("{ind}{}: {v}", quote_key(k)));
        } else {
            add.push(format!("{ind}{}:", quote_key(k)));
        }
    }
    match at {
        Some(i) => {
            // "env: {}" becomes a map
            if ls[i].value == "{}" {
                out[i] = format!("{}{}:", " ".repeat(ls[i].indent), quote_key(path[depth - 1]));
            }
            let e = block_end(&ls, i);
            for (n, l) in add.into_iter().enumerate() {
                out.insert(e + 1 + n, l);
            }
        }
        None => out.extend(add),
    }
    join(out)
}

fn quote_key(k: &str) -> String {
    if k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.') {
        k.to_string()
    } else {
        format!("'{}'", k.replace('\'', "''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const Y: &str = "game:\n  exe: drive_c/BFME2/lotrbfme2ep1.exe\n  prefix: ~/Games/bfme2/prefix\n  arch: win64\nsystem:\n  env:\n    WINEDLLOVERRIDES: d3d9=n\n    DXVK_HUD: '0'\n  prelaunch_command: set-hd-mode.sh\nwine:\n  version: Proton Hearth 9-4\n  dxvk: true\n";

    #[test]
    fn reads_lutris_yaml() {
        assert_eq!(yget(Y, &["wine", "version"]).as_deref(), Some("Proton Hearth 9-4"));
        assert_eq!(yget(Y, &["system", "env", "DXVK_HUD"]).as_deref(), Some("0"));
        assert_eq!(yget(Y, &["system", "env"]), None);
        assert_eq!(ymap(Y, &["system", "env"]).len(), 2);
        assert_eq!(yget(Y, &["game", "nothing"]), None);
    }

    #[test]
    fn writes_one_line_and_keeps_the_rest() {
        let a = yset(Y, &["wine", "dxvk"], Some("false"));
        assert!(a.contains("  dxvk: false\n") && a.contains("prelaunch_command: set-hd-mode.sh"));
        assert_eq!(a.lines().count(), Y.lines().count());
        let b = yset(Y, &["wine", "esync"], Some("true"));
        assert!(b.contains("  dxvk: true\n  esync: true\n"), "{b}");
        let c = yset(Y, &["system", "env", "WINE_LARGE_ADDRESS_AWARE"], Some("'1'"));
        assert!(c.contains("    DXVK_HUD: '0'\n    WINE_LARGE_ADDRESS_AWARE: '1'\n  prelaunch_command"), "{c}");
        assert_eq!(yget(&c, &["system", "env", "WINE_LARGE_ADDRESS_AWARE"]).as_deref(), Some("1"));
        let d = yset(Y, &["system", "env", "WINEDLLOVERRIDES"], None);
        assert!(!d.contains("WINEDLLOVERRIDES") && d.contains("DXVK_HUD"));
        let e = yset(&yset(&d, &["system", "env", "DXVK_HUD"], None), &["system", "prelaunch_command"], Some("x.sh"));
        assert!(!e.contains("env:"), "an empty map goes: {e}");
        let f = yset("", &["system", "gamescope"], Some("true"));
        assert_eq!(f, "system:\n  gamescope: true\n");
        let g = yset("system:\n  env: {}\n", &["system", "env", "A"], Some("b"));
        assert_eq!(g, "system:\n  env:\n    A: b\n");
        assert_eq!(yset(Y, &["x", "y"], None), Y);
    }

    #[test]
    fn lists() {
        let t = "Name: Office\nInstalled_Dependencies:\n- dotnet48\n- 'vcredist2019'\nRunner: wine-ge\nParameters:\n  dxvk: true\n";
        assert_eq!(ylist(t, &["Installed_Dependencies"]), ["dotnet48", "vcredist2019"]);
        assert_eq!(yget(t, &["Runner"]).as_deref(), Some("wine-ge"));
        assert_eq!(yget(t, &["Parameters", "dxvk"]).as_deref(), Some("true"));
        let n = "a:\n  list:\n  - x\n  - y\n  k: v\n";
        assert_eq!(ylist(n, &["a", "list"]), ["x", "y"]);
        assert!(ylist(t, &["Nothing"]).is_empty());
    }

    #[test]
    fn quoting() {
        assert_eq!(quote("set-hd-mode.sh"), "set-hd-mode.sh");
        assert_eq!(quote("true"), "'true'");
        assert_eq!(quote("a: b"), "'a: b'");
        assert_eq!(quote("it's"), "it's");
        assert_eq!(quote("'x"), "'''x'");
        assert_eq!(unquote("'it''s'"), "it's");
    }

}
