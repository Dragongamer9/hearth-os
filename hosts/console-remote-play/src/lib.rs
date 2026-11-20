//! Hearth Console Remote Play's workings, apart from its window (so `cargo test` checks them without a screen):
//!
//! * [`http`]: HTTPS to Microsoft (rustls), with time limits; CI's stand-in answers (HEARTH_CRP_FIXTURE);
//! * [`xal`]: signing in the way Greenlight's xal-node does (device token, SISU, Microsoft's page, XSTS, streaming
//!   tokens), all in memory; [`keyring`] keeps the one thing that's kept (the refresh token and the key);
//! * [`xhome`]: the console list, Wake, and the stream's session (play, state, SDP and ICE, keepalive, end);
//! * [`input`]: the controller packets the Xbox reads, byte for byte as xbox-xcloud-player sends them;
//! * [`chan`]: the control and message channels' JSON;
//! * [`prefs`] (the settings, red flag 9) and [`data`] (the monthly data counted on every network).
//!
//! Nothing here writes a token, a code or a password to a file or a log.

pub mod chan;
pub mod data;
pub mod http;
pub mod input;
pub mod keyring;
pub mod prefs;
pub mod spad;
pub mod stream;
pub mod xal;
pub mod xhome;

/// base64url without padding (JWK, PKCE).
pub fn b64url(b: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b)
}

pub fn b64(b: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(b)
}

pub fn unb64(s: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(s.trim()).ok()
}

/// Random bytes from the kernel.
pub fn random(n: usize) -> Vec<u8> {
    let mut v = vec![0u8; n];
    let _ = getrandom::getrandom(&mut v);
    v
}

/// A random (version 4) UUID.
pub fn uuid() -> String {
    let mut b = random(16);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

/// The home folder.
pub fn home() -> std::path::PathBuf {
    std::env::var_os("HOME").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
}

/// CI's stand-in for Microsoft's services (a folder of recorded answers), or None (the real ones).
pub fn fixture() -> Option<std::path::PathBuf> {
    std::env::var_os("HEARTH_CRP_FIXTURE").filter(|s| !s.is_empty()).map(std::path::PathBuf::from)
}

/// Seconds since 1970.
pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Does this text look like it holds a token (for the log check)? Long runs of base64 / JWT-ish characters.
pub fn looks_secret(s: &str) -> bool {
    let mut run = 0usize;
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() || "-_.+/=".contains(ch) {
            run += 1;
            if run >= 60 {
                return true;
            }
        } else {
            run = 0;
        }
    }
    false
}

/// A line for the log (stderr): never one that looks like it holds a token.
pub fn log(s: &str) {
    if looks_secret(s) {
        eprintln!("hearth-console-remote-play: (a line was left out: it looked like it held a sign-in token)");
    } else {
        eprintln!("hearth-console-remote-play: {s}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuids_and_base64() {
        let u = uuid();
        assert_eq!(u.len(), 36);
        assert_eq!(&u[14..15], "4");
        assert_eq!(b64url(&[0xfb, 0xff]), "-_8");
        assert_eq!(unb64(&b64(b"hello")).unwrap(), b"hello");
    }

    #[test]
    fn the_log_never_takes_a_token() {
        assert!(!looks_secret("Living room is Sleeping (Xbox Series X)"));
        assert!(looks_secret("token eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6Ik"));
    }
}
