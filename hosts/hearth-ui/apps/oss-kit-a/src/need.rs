//! The open-source project an app is built on: is it on this device (its own command, or its Flathub app), and
//! installing it the Hearth way when it isn't: from Flathub, for this person only (`flatpak install --user`), so
//! nothing system-wide changes and nothing about starting the device is touched (red flag 2). The app says so and
//! waits for a yes; it never installs anything by itself.

use hearth_ui::run;
use std::future::Future;

/// One project: its name, its own command, its Flathub app (and the command inside it), its licence.
#[derive(Debug, Clone, Copy)]
pub struct Need {
    pub name: &'static str,
    pub cmd: &'static str,
    pub flatpak: &'static str,
    /// the program inside the Flathub app, when it isn't the app's own (Bottles' bottles-cli)
    pub flatpak_cmd: &'static str,
    pub licence: &'static str,
}

/// Where it was found.
#[derive(Debug, Clone, PartialEq)]
pub enum Have {
    /// still looking
    Looking,
    /// its own command, at this path
    Native(String),
    /// its Flathub app
    Flatpak,
    Missing,
}

impl Have {
    pub fn ok(&self) -> bool {
        matches!(self, Have::Native(_) | Have::Flatpak)
    }
}

/// The shell that finds it (the command first, then the Flathub app, the person's or the system's).
fn probe_script(n: &Need) -> String {
    let mut s = String::new();
    if !n.cmd.is_empty() {
        s.push_str(&format!("p=$(command -v {} 2>/dev/null) && {{ echo \"native $p\"; exit 0; }}; ", n.cmd));
    }
    if !n.flatpak.is_empty() {
        s.push_str(&format!("command -v flatpak >/dev/null 2>&1 && flatpak info {} >/dev/null 2>&1 && {{ echo flatpak; exit 0; }}; ", n.flatpak));
    }
    s.push_str("echo missing; exit 1");
    s
}

/// Looks for it, off the window's thread.
pub fn probe(n: &Need) -> impl Future<Output = Have> + Send + 'static {
    let cmd = vec!["sh".to_string(), "-c".to_string(), probe_script(n)];
    async move {
        let o = run::spawn(cmd).await;
        parse(&o.out)
    }
}

pub fn parse(out: &str) -> Have {
    let l = out.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    if let Some(p) = l.strip_prefix("native ") {
        Have::Native(p.to_string())
    } else if l == "flatpak" {
        Have::Flatpak
    } else {
        Have::Missing
    }
}

/// The program to run it with: its own command, or `flatpak run` of its Flathub app (with the command inside it).
pub fn argv(n: &Need, have: &Have, args: &[&str]) -> Vec<String> {
    let mut v: Vec<String> = match have {
        Have::Flatpak => {
            let mut f = vec!["flatpak".to_string(), "run".into()];
            if !n.flatpak_cmd.is_empty() {
                f.push(format!("--command={}", n.flatpak_cmd));
            }
            f.push(n.flatpak.to_string());
            f
        }
        Have::Native(p) => vec![p.clone()],
        _ => vec![if n.cmd.is_empty() { n.flatpak_cmd.to_string() } else { n.cmd.to_string() }],
    };
    v.extend(args.iter().map(|s| s.to_string()));
    v
}

/// Installing it from Flathub, for this person only: Flathub is added for them first when it isn't there yet.
pub fn install_argv(n: &Need) -> Vec<String> {
    let s = format!(
        "flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo && flatpak install --user -y --noninteractive flathub {}",
        n.flatpak
    );
    vec!["sh".into(), "-c".into(), s]
}

/// What the app says when it isn't here.
pub fn missing_words(n: &Need) -> String {
    format!("{} isn't on this device yet. Hearth can get it from Flathub, just for you ({}, free and open source). Nothing else changes.", n.name, n.licence)
}

#[cfg(test)]
mod tests {
    use super::*;
    const L: Need = Need { name: "Lutris", cmd: "lutris", flatpak: "net.lutris.Lutris", flatpak_cmd: "", licence: "GPL-3" };
    const B: Need = Need { name: "Bottles", cmd: "bottles-cli", flatpak: "com.usebottles.bottles", flatpak_cmd: "bottles-cli", licence: "GPL-3" };

    #[test]
    fn finds_and_runs() {
        assert_eq!(parse("native /usr/bin/lutris\n"), Have::Native("/usr/bin/lutris".into()));
        assert_eq!(parse("flatpak\n"), Have::Flatpak);
        assert_eq!(parse("missing\n"), Have::Missing);
        assert_eq!(argv(&L, &Have::Flatpak, &["-l", "-j"]), ["flatpak", "run", "net.lutris.Lutris", "-l", "-j"]);
        assert_eq!(argv(&B, &Have::Flatpak, &["list"]), ["flatpak", "run", "--command=bottles-cli", "com.usebottles.bottles", "list"]);
        assert_eq!(argv(&L, &Have::Native("/x/lutris".into()), &[]), ["/x/lutris"]);
        assert!(install_argv(&L)[2].contains("install --user -y --noninteractive flathub net.lutris.Lutris"));
        assert!(probe_script(&L).contains("command -v lutris"));
    }
}
