//! Hearth Files (not built yet: a window that says so). Being written as its own pure-Rust app (hearth-ui).

use hearth_ui::app::{self, win};
use hearth_ui::ui::{self, t, ts, TitleMsgs, G};
use hearth_ui::Pal;
use iced::widget::{column, container};
use iced::{Element, Length, Subscription, Task};

#[derive(Debug, Clone)]
enum Msg {
    Theme,
    Drag,
    Maximize,
    Minimize,
    Close,
}

struct App {
    p: Pal,
}

fn new() -> (App, Task<Msg>) {
    (App { p: hearth_ui::theme::load() }, Task::none())
}

fn update(a: &mut App, m: Msg) -> Task<Msg> {
    match m {
        Msg::Theme => a.p = hearth_ui::theme::load(),
        Msg::Drag => return win::drag(),
        Msg::Maximize => return win::toggle_maximize(),
        Msg::Minimize => return win::minimize(),
        Msg::Close => return win::close(),
    }
    Task::none()
}

fn view(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let title = ui::title_bar(&p, G::Folder, "Hearth Files", None, TitleMsgs { drag: Msg::Drag, maximize: Msg::Maximize, minimize: Some(Msg::Minimize), close: Msg::Close }, p.bg2);
    let body = container(column![ts("Hearth Files", 22.0, p.text), t("Coming next.", 14.0, p.muted)].spacing(8.0)).center_x(Length::Fill).center_y(Length::Fill);
    container(column![title, body]).width(Length::Fill).height(Length::Fill).style(ui::boxed(p.bg, 0.0, None)).into()
}

fn main() -> iced::Result {
    if app::handle_cli("hearth-files-app", "Hearth Files") {
        return Ok(());
    }
    iced::application(new, update, view)
        .title("Hearth Files")
        .subscription(|_a: &App| Subscription::run(hearth_ui::theme::watch).map(|_| Msg::Theme))
        .theme(|a: &App| app::iced_theme(&a.p))
        .window(app::window("hearth-files", (1100.0, 720.0), (640.0, 420.0)))
        .default_font(app::INTER)
        .run()
}
