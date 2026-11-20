//! Display › Identify outside KDE Plasma (Mantel's SetDisplay.qml: each screen shows its number for 3 s, in a rounded
//! box in its top-left corner). On Plasma, KDE's own screen numbers (kscreen's OSD) show instead.
//!
//! `hearth-settings-app --identify DP-1=1,HDMI-A-1=2` is this program started again for those 3 s: one layer-shell
//! overlay a screen (iced_layershell, as hearth-pad-host draws its overlays), on top, never taking the keyboard or the
//! mouse, then it ends by itself. Settings itself never waits for it. Without layer-shell (or Wayland) it says so and
//! ends with 4.

use hearth_ui::app;
use hearth_ui::ui::tb;
use hearth_ui::Pal;
use iced::widget::container;
use iced::{window, Element, Length, Task};
use iced_layershell::build_pattern::daemon;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption};
use iced_layershell::settings::{LayerShellSettings, Settings, StartMode};
use iced_layershell::to_layer_message;
use std::collections::HashMap;
use std::sync::OnceLock;

/// How long the numbers show (SetDisplay.qml's identifyOff).
pub const SHOW_MS: u64 = 3000;

#[to_layer_message(multi)]
#[derive(Debug, Clone)]
pub enum Msg {
    Done,
}

struct Ident {
    p: Pal,
    nums: HashMap<window::Id, String>,
}

/// What to show: each screen's name and number, and for how long.
static WANT: OnceLock<(Vec<(String, String)>, u64)> = OnceLock::new();

/// "DP-1=1,HDMI-A-1=2" → [("DP-1", "1"), ("HDMI-A-1", "2")] (screen names are letters, digits and dashes).
pub fn parse(spec: &str) -> Vec<(String, String)> {
    spec.split(',')
        .filter_map(|x| x.split_once('='))
        .map(|(n, k)| (n.trim().to_string(), k.trim().to_string()))
        .filter(|(n, k)| !n.is_empty() && n.len() < 64 && n.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c)) && !k.is_empty() && k.len() < 4 && k.chars().all(|c| c.is_ascii_digit()))
        .take(16)
        .collect()
}

/// The argument for these screens (in Settings' order: 1, 2…).
pub fn spec(names: &[String]) -> String {
    names.iter().enumerate().map(|(i, n)| format!("{n}={}", i + 1)).collect::<Vec<_>>().join(",")
}

/// The command that shows the numbers: this program again, with --identify.
pub fn command(names: &[String]) -> Vec<String> {
    let me = std::env::current_exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|_| "hearth-settings-app".into());
    vec![me, "--identify".into(), spec(names)]
}

fn new() -> (Ident, Task<Msg>) {
    let (want, ms) = WANT.get().cloned().unwrap_or_default();
    let mut a = Ident { p: hearth_ui::theme::load(), nums: HashMap::new() };
    let mut tasks = Vec::new();
    for (name, n) in want {
        let id = window::Id::unique();
        a.nums.insert(id, n);
        let settings = NewLayerShellSettings {
            layer: Layer::Overlay,
            anchor: Anchor::Top | Anchor::Left,
            size: Some((180, 180)),
            margin: Some((40, 0, 0, 40)),
            exclusive_zone: Some(-1),
            keyboard_interactivity: KeyboardInteractivity::None,
            events_transparent: true,
            output_option: OutputOption::OutputName(name),
            namespace: Some("hearth-identify".into()),
        };
        tasks.push(Task::done(Msg::NewLayerShell { settings, id }));
    }
    tasks.push(Task::perform(hearth_ui::run::after(ms.max(500)), |_| Msg::Done));
    (a, Task::batch(tasks))
}

fn update(_a: &mut Ident, m: Msg) -> Task<Msg> {
    if let Msg::Done = m {
        std::process::exit(0);
    }
    Task::none()
}

/// A screen's number in its rounded box: the panel colour with the accent edge, the number big (SetDisplay.qml).
fn view(a: &Ident, id: window::Id) -> Element<'_, Msg> {
    let p = a.p;
    let n = a.nums.get(&id).cloned().unwrap_or_default();
    container(tb(n, 110.0, p.text))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(hearth_ui::ui::boxed(p.panel, 24.0, Some((p.accent, 3.0))))
        .into()
}

/// Does this desktop have layer-shell (KWin, sway, labwc…)? Asked before anything is drawn (as hearth-pad-host).
fn has_layer_shell() -> bool {
    use wayland_client::protocol::wl_registry;
    use wayland_client::{Connection, Dispatch, QueueHandle};
    struct S(bool);
    impl Dispatch<wl_registry::WlRegistry, ()> for S {
        fn event(st: &mut S, _: &wl_registry::WlRegistry, ev: wl_registry::Event, _: &(), _: &Connection, _: &QueueHandle<S>) {
            if let wl_registry::Event::Global { interface, .. } = ev {
                if interface == "zwlr_layer_shell_v1" {
                    st.0 = true;
                }
            }
        }
    }
    let Ok(conn) = Connection::connect_to_env() else { return false };
    let mut q = conn.new_event_queue();
    let qh = q.handle();
    let _reg = conn.display().get_registry(&qh, ());
    let mut s = S(false);
    q.roundtrip(&mut s).is_ok() && s.0
}

/// `--identify SPEC [--for MS]`: shows the numbers, then ends (the process's exit code).
pub fn run(spec: &str) -> i32 {
    let want = parse(spec);
    if want.is_empty() {
        eprintln!("hearth-settings-app --identify: no screens given (like DP-1=1,HDMI-A-1=2)");
        return 2;
    }
    if !has_layer_shell() {
        eprintln!("hearth-settings-app --identify: this desktop has no layer-shell (or no Wayland): the numbers can't be shown here");
        return 4;
    }
    let ms = app::arg("--for").and_then(|v| v.parse::<u64>().ok()).unwrap_or(SHOW_MS).min(30_000);
    let _ = WANT.set((want, ms));
    iced_layershell::disable_clipboard();
    // Rounded boxes on whatever is behind them: buffers with alpha (hearth-ui's softbuffer, vendor/README.md), drawn
    // with tiny-skia like hearth-pad-host's overlays. Only this short-lived process.
    std::env::set_var("HEARTH_SOFTBUFFER_ALPHA", "1");
    std::env::set_var("ICED_BACKEND", "tiny-skia");
    app::renderer("hearth-identify");
    let r = daemon(new, || "hearth-identify".to_string(), update, view)
        .theme(|a: &Ident, _id: window::Id| app::iced_theme(&a.p))
        .style(|a: &Ident, _t: &iced::Theme| iced::theme::Style { background_color: iced::Color::TRANSPARENT, text_color: a.p.text })
        .settings(Settings {
            layer_settings: LayerShellSettings { start_mode: StartMode::Background, ..LayerShellSettings::default() },
            default_font: app::INTER,
            ..Settings::default()
        })
        .run();
    match r {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("hearth-settings-app --identify: {e}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_screens_and_their_numbers() {
        assert_eq!(parse("DP-1=1,HDMI-A-1=2"), vec![("DP-1".to_string(), "1".to_string()), ("HDMI-A-1".to_string(), "2".to_string())]);
        assert!(parse("; rm -rf=1,x=y,=3").is_empty());
        assert_eq!(spec(&["eDP-1".to_string(), "DP-2".to_string()]), "eDP-1=1,DP-2=2");
        let c = command(&["HEADLESS-1".to_string()]);
        assert_eq!(&c[1..], &["--identify".to_string(), "HEADLESS-1=1".to_string()]);
    }
}
