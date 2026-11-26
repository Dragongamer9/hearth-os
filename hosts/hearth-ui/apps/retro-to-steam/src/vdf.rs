//! Steam's two VDF formats, as Steam ROM Manager reads and writes them: the binary one (userdata/<id>/config/
//! shortcuts.vdf, the non-Steam games) kept whole (fields Retro to Steam doesn't know are written back as they were),
//! and the text one (config/loginusers.vdf: the names of the accounts that signed in on this PC), read only.

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Map(Vec<(String, Node)>),
    Str(String),
    Int(i32),
    /// another type Steam may write (float 0x03, pointer 0x04, wide string 0x05, colour 0x06, uint64 0x07): its type
    /// and bytes, kept as they were
    Raw(u8, Vec<u8>),
}

impl Node {
    pub fn get(&self, k: &str) -> Option<&Node> {
        match self {
            Node::Map(v) => v.iter().find(|(kk, _)| kk.eq_ignore_ascii_case(k)).map(|x| &x.1),
            _ => None,
        }
    }
    pub fn str(&self, k: &str) -> &str {
        match self.get(k) {
            Some(Node::Str(s)) => s,
            _ => "",
        }
    }
    pub fn int(&self, k: &str) -> Option<i32> {
        match self.get(k) {
            Some(Node::Int(i)) => Some(*i),
            _ => None,
        }
    }
    pub fn entries(&self) -> &[(String, Node)] {
        match self {
            Node::Map(v) => v,
            _ => &[],
        }
    }
}

fn cstr(b: &[u8], i: &mut usize) -> Result<String, String> {
    let start = *i;
    while *i < b.len() && b[*i] != 0 {
        *i += 1;
    }
    if *i >= b.len() {
        return Err("a name runs off the end".into());
    }
    let s = String::from_utf8_lossy(&b[start..*i]).to_string();
    *i += 1;
    Ok(s)
}

fn map(b: &[u8], i: &mut usize, depth: u32) -> Result<Vec<(String, Node)>, String> {
    if depth > 16 {
        return Err("too deep".into());
    }
    let mut out = Vec::new();
    loop {
        let Some(&t) = b.get(*i) else { return Err("it ends inside a list".into()) };
        *i += 1;
        if t == 0x08 {
            return Ok(out);
        }
        let k = cstr(b, i)?;
        let n = match t {
            0x00 => Node::Map(map(b, i, depth + 1)?),
            0x01 => Node::Str(cstr(b, i)?),
            0x02 => {
                let v = b.get(*i..*i + 4).ok_or("a number runs off the end")?;
                *i += 4;
                Node::Int(i32::from_le_bytes([v[0], v[1], v[2], v[3]]))
            }
            0x03 | 0x04 | 0x06 => {
                let v = b.get(*i..*i + 4).ok_or("a value runs off the end")?.to_vec();
                *i += 4;
                Node::Raw(t, v)
            }
            0x07 => {
                let v = b.get(*i..*i + 8).ok_or("a value runs off the end")?.to_vec();
                *i += 8;
                Node::Raw(t, v)
            }
            _ => return Err(format!("a field of a kind it doesn't know ({t:#x})")),
        };
        out.push((k, n));
    }
}

/// shortcuts.vdf read: its root ({"shortcuts": {"0": {…}, "1": {…}}}); an empty file is an empty list.
pub fn read_binary(b: &[u8]) -> Result<Node, String> {
    if b.is_empty() {
        return Ok(Node::Map(vec![("shortcuts".into(), Node::Map(vec![]))]));
    }
    let mut i = 0;
    let root = map(b, &mut i, 0)?;
    Ok(Node::Map(root))
}

fn put(out: &mut Vec<u8>, k: &str, n: &Node) {
    match n {
        Node::Map(v) => {
            out.push(0x00);
            out.extend_from_slice(k.as_bytes());
            out.push(0);
            for (kk, nn) in v {
                put(out, kk, nn);
            }
            out.push(0x08);
        }
        Node::Str(s) => {
            out.push(0x01);
            out.extend_from_slice(k.as_bytes());
            out.push(0);
            out.extend_from_slice(s.replace('\0', "").as_bytes());
            out.push(0);
        }
        Node::Int(i) => {
            out.push(0x02);
            out.extend_from_slice(k.as_bytes());
            out.push(0);
            out.extend_from_slice(&i.to_le_bytes());
        }
        Node::Raw(t, b) => {
            out.push(*t);
            out.extend_from_slice(k.as_bytes());
            out.push(0);
            out.extend_from_slice(b);
        }
    }
}

pub fn write_binary(root: &Node) -> Vec<u8> {
    let mut out = Vec::new();
    for (k, n) in root.entries() {
        put(&mut out, k, n);
    }
    out.push(0x08);
    out
}

/// Is this a shortcuts.vdf Steam can read? (settings_file's check before the old file is kept as .bak.)
pub fn valid(b: &[u8]) -> bool {
    !b.is_empty() && read_binary(b).map(|r| r.get("shortcuts").is_some()).unwrap_or(false)
}

/// The text format: "key" "value" and "key" { … } (loginusers.vdf, libraryfolders.vdf).
pub fn read_text(t: &str) -> Node {
    let mut toks: Vec<String> = Vec::new();
    let mut it = t.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '"' => {
                let mut s = String::new();
                while let Some(c) = it.next() {
                    match c {
                        '\\' => {
                            if let Some(n) = it.next() {
                                s.push(n);
                            }
                        }
                        '"' => break,
                        _ => s.push(c),
                    }
                }
                toks.push(s);
            }
            '{' => toks.push("{".into()),
            '}' => toks.push("}".into()),
            '/' if it.peek() == Some(&'/') => {
                for c in it.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    fn block(t: &[String], i: &mut usize) -> Vec<(String, Node)> {
        let mut out = Vec::new();
        while *i < t.len() {
            if t[*i] == "}" {
                *i += 1;
                break;
            }
            let k = t[*i].clone();
            *i += 1;
            if *i < t.len() && t[*i] == "{" {
                *i += 1;
                out.push((k, Node::Map(block(t, i))));
            } else if *i < t.len() {
                out.push((k, Node::Str(t[*i].clone())));
                *i += 1;
            }
        }
        out
    }
    let mut i = 0;
    Node::Map(block(&toks, &mut i))
}

/// CRC-32 (the one Steam names a non-Steam game's id by).
pub fn crc32(data: &[u8]) -> u32 {
    let mut c: u32 = 0xffff_ffff;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0xedb8_8320 } else { c >> 1 };
        }
    }
    !c
}

/// A shortcut's id, as Steam and Steam ROM Manager work it out: CRC-32 of the program and the name, top bit set.
pub fn shortcut_id(exe: &str, name: &str) -> u32 {
    crc32(format!("{exe}{name}").as_bytes()) | 0x8000_0000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_reads_back_the_same() {
        let root = Node::Map(vec![(
            "shortcuts".into(),
            Node::Map(vec![(
                "0".into(),
                Node::Map(vec![
                    ("appid".into(), Node::Int(-123456)),
                    ("AppName".into(), Node::Str("Super Metroid".into())),
                    ("LastPlayTime".into(), Node::Int(0)),
                    ("weird".into(), Node::Raw(0x07, vec![1, 2, 3, 4, 5, 6, 7, 8])),
                    ("tags".into(), Node::Map(vec![("0".into(), Node::Str("Retro · SNES".into()))])),
                ]),
            )]),
        )]);
        let b = write_binary(&root);
        assert!(valid(&b));
        assert_eq!(read_binary(&b).unwrap(), root);
        assert!(!valid(b"\x00shortcuts\x00\x00"));
        assert_eq!(read_binary(b"").unwrap().get("shortcuts").unwrap().entries().len(), 0);
    }

    #[test]
    fn text_and_ids() {
        let n = read_text("\"users\"\n{\n\t\"76561198000000000\"\n\t{\n\t\t\"AccountName\"\t\t\"me\"\n\t\t\"PersonaName\"\t\t\"Dragon \\\"9\\\"\"\n\t}\n}\n");
        let u = n.get("users").unwrap().get("76561198000000000").unwrap();
        assert_eq!(u.str("PersonaName"), "Dragon \"9\"");
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        assert!(shortcut_id("\"/usr/bin/x\"", "Game") & 0x8000_0000 != 0);
    }
}
