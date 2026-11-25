//! Apps › Claude (Mantel's SetClaude.qml): Anthropic's Claude Desktop and Claude Code, installed for you in your
//! home folder by the hearth-claude helper. Claude Desktop comes from Anthropic's own repository, checked against
//! Anthropic's signing key, and Hearth checks for a newer one every day; on the Fedora edition it's Anthropic's
//! Debian build run from your home folder (the page says it's unofficial here). Claude Code is Anthropic's
//! installer and updates itself. Sign-in for both is with a claude.ai account, in the browser.

use super::kit::{self, *};
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::{t, G};
use iced::Task;
use serde_json::Value;

#[derive(Default)]
pub struct St {
    pub st: Value,
    pub busy: String,
    pub job: Busy,
    pub open_after: bool,
    pub error: String,
}

#[derive(Debug, Clone)]
pub enum M {
    Got(bool, Res),
    Act(&'static str, bool),
    Ev(Ev),
    Check,
    OpenClaude,
    Opened(Res),
    Terminal,
}

fn status(check: bool) -> Task<M> {
    let a: &[&str] = if check { &["status", "--check"] } else { &["status"] };
    read(py("hearth-claude", a, 60), move |r| M::Got(check, r))
}

pub fn open(_sh: &mut Shared, st: &mut St) -> Task<M> {
    st.error.clear();
    status(false)
}

pub fn update(_sh: &mut Shared, st: &mut St, m: M) -> Task<M> {
    match m {
        M::Got(check, r) => {
            if r.ok {
                st.st = r.v;
            } else if check {
                st.error = r.err;
            }
        }
        M::Act(cmd, then_open) => {
            if !st.busy.is_empty() {
                return Task::none();
            }
            st.busy = cmd.to_string();
            st.error.clear();
            st.open_after = then_open;
            st.job.start();
            return job(py("hearth-claude", &[cmd], 1800), None, M::Ev);
        }
        M::Ev(e) => match e {
            Ev::Line(f, l) => st.job.step(f, l),
            Ev::Done(r) => {
                st.job.stop();
                st.busy.clear();
                if r.ok {
                    st.st = r.v;
                    if st.open_after && gb(&st.st, "installed") {
                        return read(py("hearth-claude", &["open"], 30), M::Opened);
                    }
                } else {
                    st.error = r.err;
                    return status(false);
                }
            }
        },
        M::Check => return status(true),
        M::OpenClaude => return read(py("hearth-claude", &["open"], 30), M::Opened),
        M::Opened(r) => {
            if !r.ok {
                st.error = r.err;
            }
        }
        M::Terminal => kit::open(&["sh", "-c", "c=\"$HOME/.local/bin/claude\"; command -v hearth-terminal >/dev/null && exec hearth-terminal -e \"$c\"; command -v konsole >/dev/null && exec konsole -e \"$c\"; exec xdg-terminal-exec \"$c\""]),
    }
    Task::none()
}

fn label(b: &str) -> &'static str {
    match b {
        "install" => "Getting Claude",
        "update" => "Updating Claude",
        "remove" => "Removing Claude",
        "install-code" => "Getting Claude Code",
        "remove-code" => "Removing Claude Code",
        _ => "Working",
    }
}

pub fn view<'a>(sh: &'a Shared, st: &'a St) -> E<'a, M> {
    let p = sh.p;
    let s = &st.st;
    let idle = st.busy.is_empty();
    let mut v = crate::secs::SecVec::new();
    v.push(head(&p, "Claude Desktop"));
    let sub = if s.is_null() {
        "Checking…".to_string()
    } else if !gb(s, "supported") {
        "Anthropic makes Claude Desktop for x86-64 and ARM64 computers only".into()
    } else if gb(s, "apt") {
        let av = gs(s, "aptVersion");
        format!("Installed through apt{}. apt keeps it up to date.", if !av.is_empty() && av != "?" { format!(" · version {av}") } else { String::new() })
    } else if !gb(s, "ours") {
        "Anthropic's app for chatting with Claude, Cowork and Claude Code. Downloaded from Anthropic (about 170 MB).".into()
    } else {
        let state = if gb(s, "pending") {
            format!(" · {} is ready: close Claude to finish", gs(s, "pending"))
        } else if gb(s, "updateAvailable") {
            format!(" · {} is available", gs(s, "latest"))
        } else {
            " · up to date".into()
        };
        format!("Version {}{state} · Hearth checks for updates every day", gs(s, "version"))
    };
    v.push(r(G::Info, "Claude Desktop", sub).view(&p));
    if !s.is_null() && !gb(s, "official") && gb(s, "supported") {
        v.push(r(G::Info, "Unofficial on this edition", "Anthropic supports its Linux app on Debian and Ubuntu. Here it's Anthropic's Debian build, run by Hearth from your home folder: it should work, but Anthropic doesn't support it yet.").view(&p));
    }
    if gb(s, "supported") && s.get("sandbox").is_some() && !gb(s, "sandbox") {
        v.push(r(G::Info, "This system blocks the app's sandbox", "Claude Desktop needs user namespaces, which every Hearth OS edition allows. It won't start until they're allowed again.").view(&p));
    }
    if !idle {
        opt(&mut v, busy_view(&p, &st.job, &format!("{}…", label(&st.busy))));
    }
    if idle && !s.is_null() {
        if gb(s, "supported") && !gb(s, "installed") {
            v.push(r(G::Update, "Get Claude", "Installs Claude Desktop for you, then opens it. Sign in with your Claude account in the browser.").chev().on(M::Act("install", true)).view(&p));
        }
        if gb(s, "installed") {
            v.push(r(G::Arrow, "Open Claude", "").on(M::OpenClaude).view(&p));
        }
        if gb(s, "ours") {
            let up = gb(s, "updateAvailable") || gb(s, "pending");
            v.push(r(G::Update, if up { "Update now" } else { "Check for an update" }, if gb(s, "running") { "Close Claude first" } else { "" }).on(if up { M::Act("update", false) } else { M::Check }).view(&p));
            v.push(r(G::Close, "Remove Claude Desktop", "Your chats are kept in your Claude account; settings stay in your home folder").danger().on(M::Act("remove", false)).view(&p));
        }
    }
    v.push(head(&p, "Claude Code"));
    let code = go(s, "code");
    let csub = if s.is_null() {
        "Checking…".to_string()
    } else if gb(code, "installed") {
        format!("Installed{} · type claude in the Terminal. It keeps itself up to date.", if gb(code, "version") { format!(" · version {}", gs(code, "version")) } else { String::new() })
    } else {
        "Claude in the Terminal, for coding. Installed in your home folder with Anthropic's installer.".into()
    };
    v.push(r(G::Info, "Claude Code", csub).view(&p));
    if idle && !s.is_null() {
        if !gb(code, "installed") {
            v.push(r(G::Update, "Get Claude Code", "Runs Anthropic's installer (claude.ai/install.sh) for you").chev().on(M::Act("install-code", false)).view(&p));
        } else {
            v.push(r(G::Arrow, "Open in the Terminal", "").on(M::Terminal).view(&p));
            v.push(r(G::Close, "Remove Claude Code", "Your settings in ~/.claude stay").danger().on(M::Act("remove-code", false)).view(&p));
        }
    }
    if !st.error.is_empty() {
        v.push(t(st.error.clone(), sz(13.0), p.danger).into());
    }
    page(v)
}
