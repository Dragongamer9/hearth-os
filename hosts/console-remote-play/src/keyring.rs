//! The one thing kept between starts: the sign-in (refresh token, key, gamertag), as one item in the system keyring
//! (the Secret Service: KDE Wallet on Plasma, GNOME Keyring elsewhere), through `secret-tool` with the secret on its
//! stdin, never on a command line (the same way as Hearth's Online accounts, hearth_accounts.py). With no keyring the
//! sign-in isn't kept and the app says so: it's never written to a file instead. Sign out deletes it.
//! HEARTH_SECRET_TOOL: another secret-tool (CI's stand-in).

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const ATTRS: [&str; 4] = ["application", "hearth-console-remote-play", "account", "xbox"];

fn tool() -> String {
    std::env::var("HEARTH_SECRET_TOOL").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| "secret-tool".into())
}

/// Runs secret-tool with `input` on stdin, at most 30 s (a keyring that asks to be unlocked waits for the person).
fn run(args: &[&str], input: Option<&str>) -> Result<String, String> {
    let mut c = Command::new(tool());
    c.args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
    let mut child = c.spawn().map_err(|_| "This device has no keyring (the Secret Service), so the sign-in can't be kept safely.".to_string())?;
    if let Some(mut si) = child.stdin.take() {
        if let Some(t) = input {
            let _ = si.write_all(t.as_bytes());
        }
    }
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(st)) => {
                let mut out = String::new();
                if let Some(mut so) = child.stdout.take() {
                    use std::io::Read;
                    let _ = so.read_to_string(&mut out);
                }
                return if st.success() { Ok(out) } else { Err("The keyring said no (is it locked?).".into()) };
            }
            Ok(None) if start.elapsed() < Duration::from_secs(30) => std::thread::sleep(Duration::from_millis(30)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("The keyring didn't answer.".into());
            }
        }
    }
}

/// Keeps the sign-in (replacing what was there).
pub fn save(secret: &str) -> Result<(), String> {
    let mut args = vec!["store", "--label=Hearth Console Remote Play sign-in"];
    args.extend_from_slice(&ATTRS);
    run(&args, Some(secret)).map(|_| ())
}

/// The kept sign-in, if there is one.
pub fn load() -> Option<String> {
    let mut args = vec!["lookup"];
    args.extend_from_slice(&ATTRS);
    run(&args, None).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// Sign out: the item is deleted.
pub fn clear() -> Result<(), String> {
    let mut args = vec!["clear"];
    args.extend_from_slice(&ATTRS);
    run(&args, None).map(|_| ())
}
