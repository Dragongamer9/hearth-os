//! Typing any character through Hearth's input method for KWin (hosts/linux/desktop/kwin-helpers/hearth-kwin-im.c,
//! KWin's input method on the Fedora edition too: os-bases/anatase hearth-kwin-im-launch). It commits text whole
//! into the text field that has the keyboard, whatever the character (accents, other scripts, emoji) and whatever the
//! keyboard layout, which hearth-pad's uinput keyboard can't (US keys only). The same socket and lines hearth-type
//! uses on Debian: "T <text>" (%-escaped) → "ok", or "no" when no text field has the keyboard.
//! Not running (KDE's own keyboard is KWin's input method, or none): None, and the keyboard types through hearth-pad
//! as before. One connection, kept; every wait is short (a stuck input method never holds the keyboard up).

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

static CONN: Mutex<Option<BufReader<UnixStream>>> = Mutex::new(None);

fn path() -> PathBuf {
    let run = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(format!("/run/user/{}", unsafe { libc::getuid() })));
    run.join("hearth").join("im.sock")
}

/// hearth-kwin-im's escaping: % and line ends.
pub fn pct(t: &str) -> String {
    t.replace('%', "%25").replace('\n', "%0A").replace('\r', "%0D")
}

fn ask(line: &str) -> Option<String> {
    let mut g = CONN.lock().ok()?;
    if g.is_none() {
        let p = path();
        if !p.exists() {
            return None;
        }
        let s = UnixStream::connect(&p).ok()?;
        let _ = s.set_read_timeout(Some(Duration::from_millis(300)));
        let _ = s.set_write_timeout(Some(Duration::from_millis(300)));
        *g = Some(BufReader::new(s));
    }
    let r = g.as_mut()?;
    let ok = r.get_mut().write_all(format!("{line}\n").as_bytes()).is_ok();
    let mut ans = String::new();
    if !ok || r.read_line(&mut ans).unwrap_or(0) == 0 {
        // Gone (KWin started it again, or it stopped): the next try connects anew.
        *g = None;
        return None;
    }
    Some(ans.trim().to_string())
}

/// Some(true): the text field took it; Some(false): no text field has the keyboard; None: no Hearth input method.
pub fn commit(text: &str) -> Option<bool> {
    ask(&format!("T {}", pct(text))).map(|a| a == "ok")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_like_hearth_type() {
        assert_eq!(pct("è%\nx"), "è%25%0Ax");
    }
}
