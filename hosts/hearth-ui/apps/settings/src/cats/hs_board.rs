//! Settings › Home server, the Server section's home (board 11, docs/mockups/server-expansion/11-server-home.png;
//! checklist docs/mockups/checklists/server-server-home.md): how the server is doing (Fix it, Open the dashboard,
//! Server mode), one tile for each of the server's parts (each with "Updates in the Store" and its state there), your
//! devices, and on the right Updates for the server and the Home server pages that are also here. Drawn from
//! hearth_server_parts (the same pieces as the Home server helper's Server home), each tile its own block.

use super::sections::{Msg, State};
use crate::App;
use hearth_server_parts::home::{self as h, HAct};
use hearth_server_parts::parts;
use hearth_ui::run;
use hearth_ui::ui;
use iced::widget::{container, row, Column};
use iced::{Element, Length, Task};
use serde_json::{Map, Value};

pub fn load() -> Task<Msg> {
    Task::perform(h::load(), |d| Msg::Srv(Box::new(d)))
}

fn m(x: HAct) -> Msg {
    Msg::SrvAct(x)
}

/// What a button asks for, done the Settings way (the helper's window, the Store, Settings' own pages).
pub fn act(sh: &mut crate::shared::Shared, st: &mut State, x: HAct) -> Task<Msg> {
    match x {
        HAct::Open(id) => run::detached(&["hearth-home-server-app", "--page", id]),
        HAct::Dashboard => run::detached(&["hearth-home-server-app", "--page", "dash"]),
        HAct::Fix => {
            let first = hearth_server_parts::arr(st.srv.overall(), "needs").first().map(|n| {
                let f = hearth_server_parts::s(n, "fix");
                if f.is_empty() { hearth_server_parts::s(n, "id") } else { f }
            });
            if let Some(id) = first {
                return Task::perform(hearth_server_parts::engine::spawn("dash", vec!["fix".into(), id], String::new()), |o| Msg::SrvDone(o));
            }
        }
        HAct::ServerMode => sh.go("hs-server"),
        HAct::Update(id) | HAct::Test(id) if hearth_server_parts::engine::fixtures().is_some() => st.msg = format!("{id}: the Hearth Store installs it"),
        HAct::Update(id) => {
            let u = st.srv.up(&id).cloned().unwrap_or_default();
            let cmd = hearth_server_parts::store::install_cmd(&u, false);
            if u.system {
                return Task::perform(run::spawn(cmd), Msg::SrvDone);
            }
            let v: Vec<&str> = cmd.iter().map(|s| s.as_str()).collect();
            run::detached(&v);
        }
        HAct::Test(id) => {
            let u = st.srv.up(&id).cloned().unwrap_or_default();
            let cmd = hearth_server_parts::store::install_cmd(&u, true);
            return Task::perform(run::spawn(cmd), Msg::SrvDone);
        }
        HAct::Notes(id) => st.srv_notes = if st.srv_notes == id { String::new() } else { id },
        HAct::Auto(on) => return Task::perform(hearth_server_parts::engine::spawn("dash", vec!["autoupdate".into(), if on { "on" } else { "off" }.into()], String::new()), Msg::SrvDone),
        HAct::Idle(on) => return Task::perform(hearth_server_parts::engine::spawn("dash", vec!["set".into(), "onlyIdle".into(), if on { "on" } else { "off" }.into()], String::new()), Msg::SrvDone),
        HAct::Also(go) => sh.go(go),
        HAct::EditLayout => {}
    }
    Task::none()
}

/// "Also here": the older Home server pages with their values from hearth-settings-values (as the section page reads
/// them), each opening its page here in Settings.
fn also(v: &Map<String, Value>) -> Vec<(&'static str, String, &'static str)> {
    let g = |k: &str| super::sections::s(v, k);
    vec![
        ("Shared folders", g("shares"), "hs-shares"),
        ("Files & links", g("hs_files"), "hs-files"),
        ("People", g("family"), "hs-people"),
        ("Drives & undo", g("drive_health"), "hs-drives"),
        ("Safety", g("hs_safety"), "hs-safety"),
        ("Alerts & power", g("hs_alerts"), "hs-alerts"),
        ("The other services", "Cloud, Home, Shield…".to_string(), "hs-services"),
    ]
}

pub fn view<'a>(a: &'a App) -> Element<'a, Msg> {
    let p = a.sh.p;
    let st = &a.sections;
    let d = &st.srv;
    let mut items: Vec<(String, Element<'a, Msg>)> = vec![("How the server is doing".into(), h::state_card(&p, d, m))];
    let order = parts::sorted();
    for (i, pt) in order.iter().copied().enumerate() {
        items.push((pt.name.to_string(), h::tile(&p, d, pt, i == 0, m)));
    }
    items.push(("Your devices".into(), h::devices(&p, d)));
    items.push(("Each tile is a block".into(), h::foot(&p, m)));
    items.push(("Updates for the server".into(), h::updates(&p, d, &st.srv_notes, m)));
    items.push(("Also here".into(), h::also(&p, also(&st.vals), m)));
    let n = order.len();
    let msg = st.msg.clone();
    crate::secs::parts_dyn(a, items, move |els| {
        let mut it = els.into_iter();
        let state = it.next();
        let tiles: Vec<Element<'a, Msg>> = it.by_ref().take(n).collect();
        let devices = it.next();
        let foot = it.next();
        let updates = it.next();
        let also = it.next();
        let mut main = Column::new().spacing(14.0);
        if let Some(s_) = state {
            main = main.push(s_);
        }
        main = main.push(h::tiles_grid(tiles, 4, 293.0));
        if let Some(x) = devices {
            main = main.push(x);
        }
        if let Some(x) = foot {
            main = main.push(x);
        }
        if !msg.is_empty() {
            main = main.push(crate::widgets::note(&p, msg, true));
        }
        let mut side = Column::new();
        if let Some(x) = updates {
            side = side.push(x);
        }
        if let Some(x) = also {
            side = side.push(x);
        }
        row![container(main).width(Length::Fill), ui::vline(p.line), container(side).padding(iced::Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 20.0 }).width(Length::Fixed(360.0))].spacing(6.0).into()
    })
}
