//! Updates & recovery: hearth-admin (through pkexec: Hearth's polkit rule lets the administrator at the PC do it
//! without a password) for the status, checking, updating and going back; System Restore and Reliability history.

use crate::data;
use crate::shared::Shared;
use crate::widgets::*;
use crate::App;
use hearth_ui::run;
use hearth_ui::ui::{self, btn, glyph, t, ts, Kind, G};
use hearth_ui::Pal;
use iced::widget::{column, container, row, Column};
use iced::{Element, Length, Subscription, Task};

pub const PAGES: &[&str] = &["updates"];

#[derive(Default)]
pub struct State {
    pub u: data::Updates,
    pub busy: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Got(run::Output),
    Did(&'static str, run::Output),
    Check,
    Update,
    Restart,
    GoBack,
    OpenApp(&'static str),
    Go(&'static str),
}

fn read() -> Task<Msg> {
    Task::perform(run::spawn(vec!["pkexec".into(), data::ADMIN.into(), "update-status".into()]), Msg::Got)
}
fn act(cmd: Vec<String>, what: &'static str) -> Task<Msg> {
    Task::perform(run::spawn(cmd), move |o| Msg::Did(what, o))
}

pub fn open(_sh: &mut Shared, _st: &mut State, _page: &'static str) -> Task<Msg> {
    read()
}

pub fn update(sh: &mut Shared, st: &mut State, m: Msg) -> Task<Msg> {
    match m {
        Msg::Got(o) => match o.json() {
            Some(v) if o.ok => st.u = data::updates(&v),
            _ => {
                st.u.read = true;
                st.u.error = if o.code == 126 || o.code == 127 { "Hearth couldn't ask for permission to read the updates".into() } else { o.why() };
            }
        },
        Msg::Did(what, o) => {
            let was = st.busy.take();
            sh.result(what, &o);
            if o.ok {
                sh.note = match was {
                    Some("update") => "The update is ready. Restart to start it.".into(),
                    Some("back") => "The version before starts after a restart.".into(),
                    _ => String::new(),
                };
            }
            return read();
        }
        Msg::Check => {
            st.busy = Some("check");
            return act(vec!["pkexec".into(), data::ADMIN.into(), "update-check".into()], "Couldn't look for updates");
        }
        Msg::Update => {
            st.busy = Some("update");
            let to = data::shown_version(&st.u.available);
            let rp = run::shell_helper("hearth_restore.py");
            let script = format!("python3 {} snapshot --kind update --to {} >/dev/null 2>&1; exec pkexec {} update", data::q(&rp), data::q(&to), data::ADMIN);
            return act(data::sh(&script), "The update didn't finish");
        }
        Msg::Restart => run::detached(&["systemctl", "reboot"]),
        Msg::GoBack => {
            st.busy = Some("back");
            return act(vec!["pkexec".into(), data::ADMIN.into(), "rollback".into()], "Hearth couldn't go back");
        }
        Msg::OpenApp(bin) => run::detached(&[bin]),
        Msg::Go(id) => sh.go(id),
    }
    Task::none()
}

pub fn view<'a>(a: &'a App, page: &'static str) -> Option<Element<'a, Msg>> {
    if page != "updates" {
        return None;
    }
    let p = a.sh.p;
    let st = &a.updates;
    let u = &st.u;
    let busy = st.busy;
    let (title, sub) = if busy == Some("update") {
        ("Updating…".to_string(), "Downloading and putting the update in place. Keep using the PC; don't turn it off.".to_string())
    } else if busy == Some("check") {
        ("Looking for updates…".to_string(), String::new())
    } else if !u.staged.is_empty() {
        (format!("{} is ready", data::shown_version(&u.staged)), "Restart to start it. The version you have now stays in the boot menu.".to_string())
    } else if !u.available.is_empty() {
        (format!("{} is available", data::shown_version(&u.available)), "A restore point is made first, and the version you have now stays in the boot menu.".to_string())
    } else if !u.read {
        ("Reading…".to_string(), String::new())
    } else if !u.error.is_empty() {
        ("Couldn't read the updates".to_string(), u.error.clone())
    } else {
        ("You're up to date".to_string(), format!("Hearth OS {}{}", data::shown_version(&u.booted), if u.booted_image.is_empty() { String::new() } else { format!(" · {}", u.booted_image) }))
    };
    let action: Element<Msg> = if !u.staged.is_empty() {
        btn(&p, "Restart now", Kind::Primary, false, Some(Msg::Restart))
    } else if !u.available.is_empty() {
        btn(&p, "Update now", Kind::Primary, false, if busy.is_none() { Some(Msg::Update) } else { None })
    } else {
        btn(&p, "Check for updates", Kind::Primary, false, if busy.is_none() { Some(Msg::Check) } else { None })
    };
    let top = card(&p, row![
        container(glyph(G::Update, p.accent, 28.0, 2.2)).center_x(Length::Fixed(56.0)).center_y(Length::Fixed(56.0)).style(ui::boxed(Pal::mix(p.panel, p.accent, 0.12), 28.0, None)),
        column![ts(title, sz(20.0), p.text), t(sub, sz(13.0), p.muted)].spacing(4.0).width(Length::Fill),
        action
    ].spacing(18.0).align_y(iced::Center));
    let mut c = Column::new().spacing(10.0).push(top);
    c = c.push(head(&p, "Recovery"));
    let back_sub = if u.rollback.is_empty() { "There's no earlier version kept on this PC".to_string() } else { format!("{} starts after a restart; this one stays", data::shown_version(&u.rollback)) };
    c = c.push(srow(&p, G::Restore, "Go back a version", back_sub, btn(&p, "Go back", Kind::Normal, false, if u.rollback.is_empty() || busy.is_some() || !u.staged.is_empty() { None } else { Some(Msg::GoBack) })));
    c = c.push(link_row(&p, G::Restore, "System Restore", "Hearth and its settings back to an earlier restore point", Msg::OpenApp("hearth-system-restore")));
    c = c.push(link_row(&p, G::Reliability, "Reliability history", "How Hearth has been running, day by day", Msg::OpenApp("hearth-reliability-history")));
    Some(c.into())
}

pub fn subscription(_st: &State) -> Subscription<Msg> {
    Subscription::none()
}

pub fn dialog<'a>(_a: &'a App) -> Option<(Element<'a, Msg>, Option<Msg>)> {
    None
}

pub fn back(_st: &mut State) -> bool {
    false
}
