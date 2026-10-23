//! Text in files: which encoding a file is in (UTF-8, UTF-16 with or without a byte-order mark, or Latin-1 when
//! it's neither), its line endings (LF, CRLF or old Mac CR), and whether it's really text at all. The page always
//! gets the text with plain "\n" line breaks; saving puts the file's own line endings and encoding back.

use serde::Serialize;

/// How a file's bytes were turned into text.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Decoded {
    pub text: String,
    /// "utf-8", "utf-16le", "utf-16be" or "latin-1".
    pub encoding: String,
    /// The file starts with a byte-order mark (kept when it's saved).
    pub bom: bool,
    /// "lf", "crlf" or "cr": the line ending most lines use.
    pub eol: String,
    /// Lines end in more than one way (saving makes them all `eol`).
    pub mixed_eol: bool,
    /// Not text (NUL bytes and not UTF-16): shown read-only, with the odd bytes as dots.
    pub binary: bool,
    /// Bytes that weren't valid in the encoding (forced UTF-8 on a Latin-1 file): replaced, so saving would lose them.
    pub lossy: bool,
}

pub const ENCODINGS: [&str; 4] = ["utf-8", "utf-16le", "utf-16be", "latin-1"];

/// A NUL byte in the first 8 KB means "not text" (unless it's UTF-16): the same rule Hearth Files uses.
pub fn looks_binary(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(8192)];
    head.contains(&0) && utf16_guess(head).is_none()
}

/// UTF-16 without a byte-order mark: plain-ASCII text in UTF-16 has a NUL in every other byte.
fn utf16_guess(head: &[u8]) -> Option<&'static str> {
    let n = head.len() / 2;
    if n < 8 { return None; }
    let (mut even, mut odd) = (0usize, 0usize);
    for (i, b) in head[..n * 2].iter().enumerate() {
        if *b == 0 { if i % 2 == 0 { even += 1 } else { odd += 1 } }
    }
    if odd * 10 >= n * 3 && even * 20 <= n { return Some("utf-16le"); }
    if even * 10 >= n * 3 && odd * 20 <= n { return Some("utf-16be"); }
    None
}

/// The encoding the bytes are most likely in, and whether they start with a byte-order mark.
pub fn detect(bytes: &[u8]) -> (&'static str, bool) { detect_with(bytes, false) }

/// `partial`: the bytes are only the start of a bigger file, so a character cut off at the very end is fine.
pub fn detect_with(bytes: &[u8], partial: bool) -> (&'static str, bool) {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) { return ("utf-8", true); }
    if bytes.starts_with(&[0xFF, 0xFE]) { return ("utf-16le", true); }
    if bytes.starts_with(&[0xFE, 0xFF]) { return ("utf-16be", true); }
    let head = &bytes[..bytes.len().min(8192)];
    if let Some(e) = utf16_guess(head) { return (e, false); }
    match std::str::from_utf8(bytes) {
        Ok(_) => ("utf-8", false),
        // Only cut off at the very end of a big file shown in part: still UTF-8.
        Err(e) if partial && e.error_len().is_none() => ("utf-8", false),
        Err(_) => ("latin-1", false),
    }
}

fn bom_len(enc: &str, bytes: &[u8]) -> usize {
    match enc {
        "utf-8" if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) => 3,
        "utf-16le" if bytes.starts_with(&[0xFF, 0xFE]) => 2,
        "utf-16be" if bytes.starts_with(&[0xFE, 0xFF]) => 2,
        _ => 0,
    }
}

/// Turns the file's bytes into text, in `force` if it's given (Reopen with encoding), else the detected one.
pub fn decode(bytes: &[u8], force: Option<&str>) -> Decoded { decode_with(bytes, force, false) }

/// decode() for bytes that may be only part of the file (`partial`, see detect_with).
pub fn decode_with(bytes: &[u8], force: Option<&str>, partial: bool) -> Decoded {
    let (det, _) = detect_with(bytes, partial);
    let binary = force.is_none() && looks_binary(bytes);
    let enc = if binary { "latin-1" } else { force.filter(|f| ENCODINGS.contains(f)).unwrap_or(det) };
    let skip = bom_len(enc, bytes);
    let body = &bytes[skip..];
    let mut lossy = false;
    let raw: String = match enc {
        "utf-8" => match std::str::from_utf8(body) {
            Ok(s) => s.to_string(),
            Err(e) if partial && e.error_len().is_none() => String::from_utf8_lossy(&body[..e.valid_up_to()]).into_owned(),
            Err(_) => { lossy = true; String::from_utf8_lossy(body).into_owned() }
        },
        "utf-16le" | "utf-16be" => {
            let units: Vec<u16> = body.chunks_exact(2)
                .map(|c| if enc == "utf-16le" { u16::from_le_bytes([c[0], c[1]]) } else { u16::from_be_bytes([c[0], c[1]]) })
                .collect();
            let s: String = char::decode_utf16(units.iter().copied()).map(|r| r.unwrap_or_else(|_| { lossy = true; '\u{FFFD}' })).collect();
            s
        }
        _ if binary => body.iter().map(|&b| match b {
            b'\t' | b'\n' | b'\r' => b as char,
            0 => '\u{2400}',
            b if b < 0x20 || b == 0x7f => '·',
            b => b as char,
        }).collect(),
        _ => body.iter().map(|&b| b as char).collect(),
    };
    let (text, eol, mixed_eol) = normalize_eol(&raw);
    Decoded { text, encoding: enc.to_string(), bom: skip > 0, eol, mixed_eol, binary, lossy }
}

/// "\r\n" and lone "\r" become "\n"; says which ending most lines had and whether there was more than one kind.
pub fn normalize_eol(s: &str) -> (String, String, bool) {
    let (mut lf, mut crlf, mut cr) = (0usize, 0usize, 0usize);
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '\r' => {
                if it.peek() == Some(&'\n') { it.next(); crlf += 1; } else { cr += 1; }
                out.push('\n');
            }
            '\n' => { lf += 1; out.push('\n'); }
            c => out.push(c),
        }
    }
    let eol = if crlf > lf && crlf >= cr { "crlf" } else if cr > lf && cr > crlf { "cr" } else if lf == 0 && crlf == 0 && cr == 0 { default_eol() } else { "lf" };
    let kinds = [lf, crlf, cr].iter().filter(|n| **n > 0).count();
    (out, eol.to_string(), kinds > 1)
}

/// A new file's line endings: the system's own.
pub fn default_eol() -> &'static str { if cfg!(windows) { "crlf" } else { "lf" } }

/// The bytes to save: the text with `eol` line endings, in `encoding`, with a byte-order mark if `bom`.
/// Latin-1 can only hold the first 256 characters: anything else is refused with the first one it can't hold.
pub fn encode(text: &str, encoding: &str, bom: bool, eol: &str) -> Result<Vec<u8>, String> {
    let nl = match eol { "crlf" => "\r\n", "cr" => "\r", _ => "\n" };
    let unified = if text.contains('\r') { normalize_eol(text).0 } else { text.to_string() };
    let body = if nl == "\n" { unified } else { unified.replace('\n', nl) };
    let mut out = Vec::with_capacity(body.len() + 3);
    match encoding {
        "utf-8" => { if bom { out.extend_from_slice(&[0xEF, 0xBB, 0xBF]); } out.extend_from_slice(body.as_bytes()); }
        "utf-16le" | "utf-16be" => {
            let le = encoding == "utf-16le";
            if bom { out.extend_from_slice(if le { &[0xFF, 0xFE] } else { &[0xFE, 0xFF] }); }
            for u in body.encode_utf16() { out.extend_from_slice(&if le { u.to_le_bytes() } else { u.to_be_bytes() }); }
        }
        "latin-1" => {
            for (i, c) in body.chars().enumerate() {
                let n = c as u32;
                if n > 0xFF {
                    let line = body.chars().take(i).filter(|c| *c == '\n' || *c == '\r').count() + 1;
                    return Err(format!("“{c}” (line {line}) can't be saved in Latin-1. Choose UTF-8 under Encoding, then save."));
                }
                out.push(n as u8);
            }
        }
        other => return Err(format!("Unknown encoding {other}")),
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_and_line_endings() {
        let d = decode(b"one\r\ntwo\r\nthree", None);
        assert_eq!((d.text.as_str(), d.encoding.as_str(), d.eol.as_str(), d.bom, d.mixed_eol), ("one\ntwo\nthree", "utf-8", "crlf", false, false));
        let d = decode("héllo\nwörld\n".as_bytes(), None);
        assert_eq!((d.encoding.as_str(), d.eol.as_str()), ("utf-8", "lf"));
        let d = decode(b"a\r\nb\nc\n", None);
        assert!(d.mixed_eol);
        assert_eq!(d.eol, "lf");
        assert_eq!(decode(b"a\rb\rc", None).eol, "cr");
    }

    #[test]
    fn boms_and_utf16() {
        let d = decode(&[0xEF, 0xBB, 0xBF, b'h', b'i'], None);
        assert_eq!((d.text.as_str(), d.encoding.as_str(), d.bom), ("hi", "utf-8", true));
        let le: Vec<u8> = [0xFF, 0xFE].into_iter().chain("hé\r\n".encode_utf16().flat_map(|u| u.to_le_bytes())).collect();
        let d = decode(&le, None);
        assert_eq!((d.text.as_str(), d.encoding.as_str(), d.bom, d.eol.as_str()), ("hé\n", "utf-16le", true, "crlf"));
        let be: Vec<u8> = "plain ascii text, long enough".encode_utf16().flat_map(|u| u.to_be_bytes()).collect();
        let d = decode(&be, None);
        assert_eq!((d.text.as_str(), d.encoding.as_str(), d.bom, d.binary), ("plain ascii text, long enough", "utf-16be", false, false));
    }

    #[test]
    fn latin1_when_not_utf8() {
        let d = decode(&[b'c', b'a', b'f', 0xE9], None);
        assert_eq!((d.text.as_str(), d.encoding.as_str()), ("café", "latin-1"));
        assert_eq!(encode("café", "latin-1", false, "lf").unwrap(), vec![b'c', b'a', b'f', 0xE9]);
        assert!(encode("€", "latin-1", false, "lf").unwrap_err().contains("Latin-1"));
    }

    #[test]
    fn binary_files() {
        let d = decode(&[0x7f, b'E', b'L', b'F', 0, 0, 1, 0, 0, 0, 0, 0, 3, 0, 0, 0], None);
        assert!(d.binary);
        assert!(d.text.contains('\u{2400}'));
        assert!(looks_binary(&[1, 2, 0, 4]));
        assert!(!looks_binary(b"just text"));
    }

    #[test]
    fn cut_off_utf8_is_still_utf8() {
        let mut b = "aé".as_bytes().to_vec();
        b.pop();
        let d = decode_with(&b, None, true);
        assert_eq!((d.encoding.as_str(), d.text.as_str(), d.lossy), ("utf-8", "a", false));
        // The whole file ending that way isn't UTF-8.
        assert_eq!(decode(&b, None).encoding, "latin-1");
    }

    #[test]
    fn round_trips() {
        for enc in ENCODINGS {
            for eol in ["lf", "crlf", "cr"] {
                for bom in [false, true] {
                    if enc == "latin-1" && bom { continue; }
                    let bytes = encode("first line\nsecond ü\n", enc, bom, eol).unwrap();
                    let d = decode(&bytes, None);
                    assert_eq!(d.text, "first line\nsecond ü\n", "{enc} {eol} {bom}");
                    assert_eq!(d.eol, eol, "{enc} {eol} {bom}");
                    assert_eq!(d.bom, bom, "{enc} {eol} {bom}");
                    if !(enc == "latin-1") { assert_eq!(d.encoding, enc); }
                }
            }
        }
    }

    #[test]
    fn forced_encoding() {
        let d = decode("é".as_bytes(), Some("latin-1"));
        assert_eq!(d.text, "Ã©");
        let d = decode(&[0xE9], Some("utf-8"));
        assert!(d.lossy);
    }
}
