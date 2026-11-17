//! How the compositor was started (main.rs parses the command line once; everything else reads it with `get()`).

use std::{path::PathBuf, sync::OnceLock};

pub const USAGE: &str = "usage: firehold-comp [--winit | --tty-udev | --headless | --auto] [--socket NAME] [--config FILE] \
[--labwc-rc FILE] [--size WxH] [--no-xwayland] [-- COMMAND ARGS...]";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Winit,
    Udev,
    /// Drawn into memory, no screen or window (tests, CI: headless.rs).
    Headless,
    Auto,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub backend: Backend,
    /// The Wayland socket's name in $XDG_RUNTIME_DIR (None: the first free wayland-N).
    pub socket: Option<String>,
    /// The compositor's own settings (read only). None: $FIREHOLD_CONFIG or ~/.config/hearth/compositor.json.
    pub config: Option<PathBuf>,
    /// The labwc rc.xml Desktop Mode uses (keybinds, key repeat), read only.
    pub labwc_rc: Option<PathBuf>,
    /// The nested window's size (winit).
    pub size: (i32, i32),
    pub xwayland: bool,
    /// Started once the compositor is up: Desktop Mode's autostart.
    pub command: Vec<String>,
    pub help: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            backend: Backend::Auto,
            socket: None,
            config: None,
            labwc_rc: None,
            size: (1280, 800),
            xwayland: true,
            command: Vec::new(),
            help: false,
        }
    }
}

fn parse_size(s: &str) -> Option<(i32, i32)> {
    let (w, h) = s.split_once('x')?;
    let (w, h) = (w.trim().parse().ok()?, h.trim().parse().ok()?);
    (w >= 64 && h >= 64 && w <= 16384 && h <= 16384).then_some((w, h))
}

impl Options {
    pub fn parse<I: Iterator<Item = String>>(mut args: I) -> Result<Options, String> {
        let mut o = Options::default();
        if let Some(s) = std::env::var("FIREHOLD_SIZE").ok().and_then(|s| parse_size(&s)) {
            o.size = s;
        }
        while let Some(a) = args.next() {
            match a.as_str() {
                "--winit" => o.backend = Backend::Winit,
                "--tty-udev" | "--udev" => o.backend = Backend::Udev,
                "--headless" => o.backend = Backend::Headless,
                "--auto" => o.backend = Backend::Auto,
                "--socket" => o.socket = Some(args.next().ok_or("--socket needs a name")?),
                "--config" => o.config = Some(args.next().ok_or("--config needs a file")?.into()),
                "--labwc-rc" => o.labwc_rc = Some(args.next().ok_or("--labwc-rc needs a file")?.into()),
                "--size" => {
                    let s = args.next().ok_or("--size needs WxH")?;
                    o.size = parse_size(&s).ok_or_else(|| format!("bad size {s}"))?;
                }
                "--no-xwayland" => o.xwayland = false,
                "-h" | "--help" => o.help = true,
                "--" => {
                    o.command = args.by_ref().collect();
                    break;
                }
                other if other.starts_with('-') => return Err(format!("unknown option {other}")),
                // Like KWin: the first word that isn't an option starts the command.
                other => {
                    o.command.push(other.to_string());
                    o.command.extend(args.by_ref());
                    break;
                }
            }
        }
        if let Some(s) = &o.socket {
            if s.is_empty() || s.contains('/') {
                return Err(format!("bad socket name {s}"));
            }
        }
        Ok(o)
    }

    /// The backend to run: --auto is nested inside another desktop, on the screens otherwise.
    pub fn backend(&self) -> Backend {
        match self.backend {
            Backend::Auto => {
                let set = |k: &str| std::env::var_os(k).is_some_and(|v| !v.is_empty());
                if set("WAYLAND_DISPLAY") || set("DISPLAY") {
                    Backend::Winit
                } else {
                    Backend::Udev
                }
            }
            b => b,
        }
    }
}

static OPTIONS: OnceLock<Options> = OnceLock::new();

pub fn set(o: Options) {
    let _ = OPTIONS.set(o);
}

pub fn get() -> &'static Options {
    OPTIONS.get_or_init(Options::default)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(a: &[&str]) -> Options {
        Options::parse(a.iter().map(|s| s.to_string())).unwrap()
    }

    #[test]
    fn command_after_dashes_and_bare_words() {
        let o = p(&["--winit", "--socket", "wayland-hearth", "--", "sh", "-c", "x"]);
        assert_eq!(o.backend, Backend::Winit);
        assert_eq!(o.socket.as_deref(), Some("wayland-hearth"));
        assert_eq!(o.command, vec!["sh", "-c", "x"]);
        let o = p(&["--tty-udev", "/usr/share/autostart", "--not-ours"]);
        assert_eq!(o.backend, Backend::Udev);
        assert_eq!(o.command, vec!["/usr/share/autostart", "--not-ours"]);
    }

    #[test]
    fn sizes_and_bad_options() {
        assert_eq!(p(&["--size", "1920x1080"]).size, (1920, 1080));
        assert!(Options::parse(["--size", "1x1"].iter().map(|s| s.to_string())).is_err());
        assert!(Options::parse(["--bogus"].iter().map(|s| s.to_string())).is_err());
        assert_eq!(p(&["--headless"]).backend(), Backend::Headless);
        assert!(Options::parse(["--socket", "a/b"].iter().map(|s| s.to_string())).is_err());
    }
}
