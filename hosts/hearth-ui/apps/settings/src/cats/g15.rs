//! Bluetooth & devices › Logitech G15 (Mantel's SetG15.qml): one row that opens the Logitech G15 app
//! (hearth-g15-app, hosts/hearth-ui/apps/g15), with whether the keyboard is plugged in (`hearth-g15 status`, read once
//! when the page opens). Everything else is in the app. Its own module so it merges apart from cats/bt.rs.
//! See cats/mod.rs for the shape. Opening it writes nothing.

use crate::shared::Shared;
use crate::widgets::*;
use crate::App;
use hearth_ui::run;
use hearth_ui::ui::G;
use iced::widget::Column;
use iced::{Element, Subscription, Task};
use serde_json::Value;

pub const PAGES: &[&str] = &["g15"];

#[derive(Default)]
pub struct State {
    pub status: Option<Value>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Status(run::Output),
    Open,
}

pub fn open(_sh: &mut Shared, st: &mut State, _page: &'static str) -> Task<Msg> {
    st.status = None;
    Task::perform(run::spawn(run::args(&["timeout", "5", "hearth-g15", "status"])), Msg::Status)
}

pub fn update(_sh: &mut Shared, st: &mut State, m: Msg) -> Task<Msg> {
    match m {
        Msg::Status(o) => st.status = Some(o.json().unwrap_or(Value::Null)),
        Msg::Open => run::detached(&["hearth-g15-app"]),
    }
    Task::none()
}

pub fn view<'a>(a: &'a App, page: &'static str) -> Option<Element<'a, Msg>> {
    if page != "g15" {
        return None;
    }
    let p = a.sh.p;
    let st = &a.g15.status;
    let plugged = st.as_ref().and_then(|v| v.get("plugged")).and_then(|v| v.as_bool()).unwrap_or(false);
    let model = st.as_ref().and_then(|v| v.get("model")).and_then(|v| v.as_str()).unwrap_or("G15").to_string();
    let sub = match st {
        None => "Looking for the keyboard…".to_string(),
        Some(_) if plugged => format!("{model} plugged in · G-keys and macros, its screen, its lights"),
        Some(_) => "Not plugged in · G-keys and macros, its screen, its lights".to_string(),
    };
    let c = Column::new().spacing(10.0).push(link_row(&p, G::Grid, "Logitech G15", sub, Msg::Open));
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
