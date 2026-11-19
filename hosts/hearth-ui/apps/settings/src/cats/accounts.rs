//! (Not built yet: its pages show on their category as "Coming next".) See cats/mod.rs for the shape.

use crate::shared::Shared;
use crate::App;
use iced::{Element, Subscription, Task};

pub const PAGES: &[&str] = &[];

#[derive(Default)]
pub struct State {}

#[derive(Debug, Clone)]
pub enum Msg {
    Go(&'static str),
}

pub fn open(_sh: &mut Shared, _st: &mut State, _page: &'static str) -> Task<Msg> {
    Task::none()
}

pub fn update(sh: &mut Shared, _st: &mut State, m: Msg) -> Task<Msg> {
    match m {
        Msg::Go(id) => sh.go(id),
    }
    Task::none()
}

pub fn view<'a>(_a: &'a App, _page: &'static str) -> Option<Element<'a, Msg>> {
    None
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
