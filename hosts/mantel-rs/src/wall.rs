//! The wallpaper, behind everything (the background layer): the picture you chose in Settings › Appearance (desktop
//! .json "wallpaper"), else the look's own. This test draws the animated ones as their still pictures (Tide, Lava,
//! the Hearth look's embers) and the scenes without one (Neon, Zen, Glass, Synthwave) as the look's colours: nothing
//! moves, so it costs nothing while you work. Follows a new pick at once.

use iced::widget::{container, image};
use iced::{ContentFit, Element, Gradient, Length, Radians, Subscription, Task};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings, StartMode};
use iced_layershell::to_layer_message;
use serde_json::Value;
use std::path::PathBuf;

use crate::look;

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    Look,
}

struct Wall {
    pic: Option<PathBuf>,
}

/// What desktop.json's "wallpaper" (or the look's) means as a file, if there's a picture for it.
pub fn resolve(w: &str, share: &std::path::Path) -> Option<PathBuf> {
    let shell = share.join("desktop/shell/images");
    let f = if let Some(b) = w.strip_prefix("builtin:") {
        shell.join("wallpapers").join(format!("{b}.jpg"))
    } else if let Some(a) = w.strip_prefix("anim:") {
        match a {
            "tide" | "lava" | "aurora" | "ridges" | "motes" => shell.join(format!("wall-{a}.jpg")),
            _ => return None,
        }
    } else if w.is_empty() {
        shell.join("wall-motes.jpg")
    } else {
        PathBuf::from(w.strip_prefix("file://").unwrap_or(w))
    };
    f.exists().then_some(f)
}

/// The picture for this screen now.
pub fn picture() -> Option<PathBuf> {
    let d: Value = std::fs::read(hearth_ui::theme::desktop_json()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
    let mut w = d.get("wallpaper").and_then(Value::as_str).unwrap_or("").to_string();
    if w.is_empty() {
        let map = hearth_ui::theme::palette_map();
        w = map.get(&look::look_name()).and_then(|l| l.get("wallpaper")).and_then(Value::as_str).unwrap_or("").to_string();
    }
    resolve(&w, &look::share())
}

fn update(s: &mut Wall, m: Message) -> Task<Message> {
    if let Message::Look = m {
        s.pic = picture();
    }
    Task::none()
}

fn view(s: &Wall) -> Element<'_, Message> {
    let p = look::pal();
    match &s.pic {
        Some(f) => image(image::Handle::from_path(f)).content_fit(ContentFit::Cover).width(Length::Fill).height(Length::Fill).into(),
        None => {
            let g = Gradient::Linear(iced::gradient::Linear::new(Radians(2.4)).add_stop(0.0, p.bg2).add_stop(0.6, p.bg).add_stop(1.0, hearth_ui::theme::Pal::mix(p.bg, p.accent, 0.3)));
            container("").width(Length::Fill).height(Length::Fill).style(move |_| container::Style { background: Some(g.into()), ..container::Style::default() }).into()
        }
    }
}

fn subscription(_s: &Wall) -> Subscription<Message> {
    Subscription::run(look::changes).map(|_| Message::Look)
}

pub fn run() -> i32 {
    iced_layershell::disable_clipboard();
    let layer_settings = LayerShellSettings {
        anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
        layer: Layer::Background,
        exclusive_zone: -1,
        size: Some((0, 0)),
        keyboard_interactivity: KeyboardInteractivity::None,
        start_mode: StartMode::Active,
        ..Default::default()
    };
    let r = iced_layershell::application(|| Wall { pic: picture() }, || "hearth-wallpaper".to_string(), update, view)
        .style(|_s: &Wall, _t: &iced::Theme| iced::theme::Style { background_color: look::pal().bg, text_color: look::pal().text })
        .subscription(subscription)
        .settings(Settings { layer_settings, default_font: look::INTER, ..Default::default() })
        .run();
    crate::ended("wallpaper", r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wallpaper_names_become_files() {
        let d = tempfile::tempdir().unwrap();
        let img = d.path().join("desktop/shell/images");
        std::fs::create_dir_all(img.join("wallpapers")).unwrap();
        std::fs::write(img.join("wallpapers/slate-dusk.jpg"), b"x").unwrap();
        std::fs::write(img.join("wall-tide.jpg"), b"x").unwrap();
        assert_eq!(resolve("builtin:slate-dusk", d.path()), Some(img.join("wallpapers/slate-dusk.jpg")));
        assert_eq!(resolve("anim:tide", d.path()), Some(img.join("wall-tide.jpg")));
        assert_eq!(resolve("anim:neon", d.path()), None);
        assert_eq!(resolve("/nope.png", d.path()), None);
    }
}
