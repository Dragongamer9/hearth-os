//! Mantel's keyboard shortcuts, read from the labwc rc.xml Desktop Mode already uses (hosts/linux/desktop/labwc/rc.xml,
//! or the copy hearth-system makes with your keyboard settings): the same keys work the same on Hearth's compositor
//! as on labwc, with one list to keep. Read only. Also the key repeat (<keyboard><repeatRate>, <repeatDelay>).
//!
//! What's understood: <keybind key="W-C-A-S-name" [onRelease="yes"]> with the actions Execute, Close, NextWindow,
//! PreviousWindow, SnapToEdge (left/right/up), Maximize/ToggleMaximize/UnMaximize, Iconify and ToggleFullscreen;
//! labwc's If blocks take their <else> action (Super+Up maximizes, Super+Down minimizes). Anything else is skipped and
//! named once in the log.

use smithay::input::keyboard::{xkb, Keysym, ModifiersState};

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Execute(String),
    Close,
    NextWindow,
    PreviousWindow,
    Snap(Edge),
    Maximize,
    ToggleMaximize,
    UnMaximize,
    Iconify,
    ToggleFullscreen,
    VtSwitch(i32),
    /// Ctrl+Alt+Backspace: leave Firehold for KWin at once (always there; never while the screen is locked).
    LeaveForKwin,
    /// labwc's GoToDesktop (to, wrap).
    GoToDesktop(DeskTarget, bool),
    /// labwc's SendToDesktop (to, wrap, follow).
    SendToDesktop(DeskTarget, bool, bool),
}

/// labwc's desktop "to": left, right or a number (1-based).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeskTarget {
    Left,
    Right,
    Number(usize),
}

fn desk_target(a: &roxmltree::Node<'_, '_>) -> Option<DeskTarget> {
    match a.attribute("to").unwrap_or("").trim() {
        "left" => Some(DeskTarget::Left),
        "right" => Some(DeskTarget::Right),
        n => n.parse::<usize>().ok().filter(|n| *n >= 1).map(DeskTarget::Number),
    }
}

fn yes(a: &roxmltree::Node<'_, '_>, name: &str, default: bool) -> bool {
    match a.attribute(name) {
        Some("yes") | Some("true") => true,
        Some("no") | Some("false") => false,
        _ => default,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
    Top,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Mods {
    pub logo: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl Mods {
    pub fn of(m: &ModifiersState) -> Mods {
        Mods {
            logo: m.logo,
            ctrl: m.ctrl,
            alt: m.alt,
            shift: m.shift,
        }
    }
    fn none(&self) -> bool {
        *self == Mods::default()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bind {
    pub mods: Mods,
    pub key: Keysym,
    pub on_release: bool,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, Default)]
pub struct Keymap {
    pub binds: Vec<Bind>,
    /// Keys per second and milliseconds before repeating (None: Hearth's defaults).
    pub repeat: Option<(i32, i32)>,
    pub skipped: Vec<String>,
    /// The virtual desktops' names (<desktops number="N"> with <names>, or <prefix> N).
    pub desktops: Vec<String>,
}

fn keysym(name: &str) -> Option<Keysym> {
    let k = xkb::keysym_from_name(name, xkb::KEYSYM_NO_FLAGS);
    let k = if k.raw() == 0 { xkb::keysym_from_name(name, xkb::KEYSYM_CASE_INSENSITIVE) } else { k };
    (k.raw() != 0).then_some(k)
}

/// "W-C-A-S-name" (labwc's spelling; Super, Mod4, Ctrl, Alt, Shift spelled out work too).
pub fn parse_key(spec: &str) -> Option<(Mods, Keysym)> {
    let mut mods = Mods::default();
    let parts: Vec<&str> = spec.split('-').collect();
    let (name, prefix) = match parts.split_last() {
        // "W--" would be the minus key; labwc writes it "minus".
        Some((last, rest)) if !last.is_empty() => (*last, rest),
        _ => return None,
    };
    for p in prefix {
        match *p {
            "W" | "Super" | "Mod4" | "Logo" => mods.logo = true,
            "C" | "Ctrl" | "Control" => mods.ctrl = true,
            "A" | "Alt" | "Mod1" => mods.alt = true,
            "S" | "Shift" => mods.shift = true,
            _ => return None,
        }
    }
    Some((mods, keysym(name)?))
}

fn actions_of(node: roxmltree::Node<'_, '_>, skipped: &mut Vec<String>) -> Vec<Action> {
    let mut out = Vec::new();
    for a in node.children().filter(|n| n.has_tag_name("action")) {
        let name = a.attribute("name").unwrap_or("");
        let act = match name.to_ascii_lowercase().as_str() {
            "execute" => a
                .attribute("command")
                .or_else(|| a.children().find(|n| n.has_tag_name("command")).and_then(|n| n.text()))
                .map(|c| Action::Execute(c.to_string())),
            "close" => Some(Action::Close),
            "nextwindow" => Some(Action::NextWindow),
            "previouswindow" => Some(Action::PreviousWindow),
            "snaptoedge" => match a.attribute("direction").unwrap_or("") {
                "left" => Some(Action::Snap(Edge::Left)),
                "right" => Some(Action::Snap(Edge::Right)),
                "up" => Some(Action::Snap(Edge::Top)),
                _ => None,
            },
            "maximize" => Some(Action::Maximize),
            "togglemaximize" => Some(Action::ToggleMaximize),
            "unmaximize" => Some(Action::UnMaximize),
            "iconify" | "minimize" => Some(Action::Iconify),
            "togglefullscreen" => Some(Action::ToggleFullscreen),
            "gotodesktop" => desk_target(&a).map(|t| Action::GoToDesktop(t, yes(&a, "wrap", true))),
            "sendtodesktop" => desk_target(&a).map(|t| Action::SendToDesktop(t, yes(&a, "wrap", true), yes(&a, "follow", true))),
            // labwc's conditions: Hearth's compositor takes the plain case (the <else>), e.g. Super+Up maximizes.
            "if" => {
                let inner = a
                    .children()
                    .find(|n| n.has_tag_name("else"))
                    .map(|e| actions_of(e, skipped))
                    .unwrap_or_default();
                out.extend(inner);
                None
            }
            _ => {
                skipped.push(name.to_string());
                None
            }
        };
        if let Some(act) = act {
            out.push(act);
        }
    }
    out
}

pub fn parse(xml: &str) -> Keymap {
    let mut km = Keymap::default();
    let doc = match roxmltree::Document::parse(xml) {
        Ok(d) => d,
        Err(e) => {
            km.skipped.push(format!("rc.xml: {e}"));
            return km;
        }
    };
    for kb in doc.descendants().filter(|n| n.has_tag_name("keybind")) {
        let Some(spec) = kb.attribute("key") else { continue };
        let Some((mods, key)) = parse_key(spec) else {
            km.skipped.push(format!("key {spec}"));
            continue;
        };
        let actions = actions_of(kb, &mut km.skipped);
        if actions.is_empty() {
            continue;
        }
        let on_release = matches!(kb.attribute("onRelease"), Some("yes") | Some("true"));
        // A later bind for the same keys replaces an earlier one (as labwc does with the user's rc.xml on top).
        km.binds.retain(|b| !(b.mods == mods && b.key == key && b.on_release == on_release));
        km.binds.push(Bind { mods, key, on_release, actions });
    }
    if let Some(d) = doc.descendants().find(|n| n.has_tag_name("desktops")) {
        let text = |tag: &str| d.children().find(|n| n.has_tag_name(tag)).and_then(|n| n.text()).map(|t| t.trim().to_string());
        let number = d
            .attribute("number")
            .map(|t| t.trim().to_string())
            .or_else(|| text("number"))
            .and_then(|t| t.parse::<usize>().ok())
            .unwrap_or(1)
            .clamp(1, 20);
        let names: Vec<String> = d
            .children()
            .find(|n| n.has_tag_name("names"))
            .map(|ns| ns.children().filter(|n| n.has_tag_name("name")).filter_map(|n| n.text()).map(|t| t.trim().to_string()).collect())
            .unwrap_or_default();
        let prefix = text("prefix").filter(|p| !p.is_empty()).unwrap_or_else(|| "Desktop".into());
        km.desktops = (0..number)
            .map(|i| names.get(i).filter(|n| !n.is_empty()).cloned().unwrap_or_else(|| format!("{prefix} {}", i + 1)))
            .collect();
    }
    if let Some(k) = doc.descendants().find(|n| n.has_tag_name("keyboard")) {
        let num = |tag: &str| {
            k.children()
                .find(|n| n.has_tag_name(tag))
                .and_then(|n| n.text())
                .and_then(|t| t.trim().parse::<i32>().ok())
        };
        if let (Some(rate), Some(delay)) = (num("repeatRate"), num("repeatDelay")) {
            if (1..=200).contains(&rate) && (50..=5000).contains(&delay) {
                km.repeat = Some((rate, delay));
            }
        }
    }
    km
}

/// The compositor's own keys, always there (they can't be taken away by a broken rc.xml): switching to another
/// console with Ctrl+Alt+F1..F12.
pub fn builtin(key: Keysym) -> Option<Action> {
    let raw = key.raw();
    (xkb::keysyms::KEY_XF86Switch_VT_1..=xkb::keysyms::KEY_XF86Switch_VT_12)
        .contains(&raw)
        .then(|| Action::VtSwitch((raw - xkb::keysyms::KEY_XF86Switch_VT_1 + 1) as i32))
}

/// The key that always gets out of the preview, whatever rc.xml says: Ctrl+Alt+Backspace (Terminate_Server is
/// the same keys with X's "terminate" option). Not in Mantel's rc.xml, so nothing else is lost.
pub fn leave_key(mods: Mods, key: Keysym) -> bool {
    (mods.ctrl && mods.alt && !mods.logo && key == Keysym::BackSpace) || key == Keysym::Terminate_Server
}

impl Keymap {
    /// The bind for a key press (on_release false) or release (true) with these modifiers held.
    pub fn find(&self, mods: Mods, key: Keysym, on_release: bool) -> Option<&Bind> {
        self.binds
            .iter()
            .find(|b| b.on_release == on_release && b.key == key && b.mods == mods)
    }

    /// Mantel's lock command (Super+L's Execute): the compositor asks Mantel to lock again with it when the lock
    /// screen went away while locked (lock.rs). Falls back to Mantel's ipc call.
    pub fn lock_command(&self) -> Option<String> {
        let l = keysym("l")?;
        let logo = Mods { logo: true, ..Mods::default() };
        let from_rc = self.find(logo, l, false).and_then(|b| {
            b.actions.iter().find_map(|a| match a {
                Action::Execute(c) if c.contains("lock") => Some(c.clone()),
                _ => None,
            })
        });
        from_rc.or_else(|| {
            let share = std::env::var("NIMBUS_SHARE").unwrap_or_else(|_| "/usr/local/share/nimbus".into());
            Some(format!("quickshell ipc -p '{share}/desktop/shell' call lock lock"))
        })
    }

    /// A lone modifier bind (Super_L onRelease: the launcher) wants the modifier itself to count as no modifier.
    pub fn find_lone(&self, key: Keysym) -> Option<&Bind> {
        self.binds
            .iter()
            .find(|b| b.on_release && b.key == key && b.mods.none())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RC: &str = r#"<?xml version="1.0"?>
<labwc_config>
  <keyboard>
    <repeatRate>30</repeatRate><repeatDelay>400</repeatDelay>
    <keybind key="Super_L" onRelease="yes"><action name="Execute" command="sh -c 'launcher'" /></keybind>
    <keybind key="W-q"><action name="Close" /></keybind>
    <keybind key="W-S-1"><action name="SendToDesktop" to="1" /></keybind>
    <keybind key="W-C-Left"><action name="GoToDesktop" to="left" wrap="no" /></keybind>
    <keybind key="W-C-S-Right"><action name="SendToDesktop" to="right" wrap="no" follow="no" /></keybind>
    <keybind key="W-Left"><action name="SnapToEdge" direction="left" /></keybind>
    <keybind key="W-Up"><action name="If"><query maximized="both" /><then><action name="UnMaximize" /></then>
      <else><action name="Maximize" /></else></action></keybind>
    <keybind key="W-l"><action name="Execute" command="sh -c 'quickshell ipc -p &quot;$X&quot; call lock lock'" /></keybind>
    <keybind key="XF86AudioRaiseVolume"><action name="Execute" command="up" /></keybind>
  </keyboard>
  <desktops number="3"><prefix>Desk</prefix><names><name>Work</name><name>Play</name></names></desktops>
</labwc_config>"#;

    #[test]
    fn reads_hearths_rc() {
        let km = parse(RC);
        assert_eq!(km.repeat, Some((30, 400)));
        let q = keysym("q").unwrap();
        let logo = Mods { logo: true, ..Mods::default() };
        assert_eq!(km.find(logo, q, false).unwrap().actions, vec![Action::Close]);
        let up = keysym("Up").unwrap();
        assert_eq!(km.find(logo, up, false).unwrap().actions, vec![Action::Maximize]);
        let l = keysym("l").unwrap();
        match &km.find(logo, l, false).unwrap().actions[0] {
            Action::Execute(c) => assert!(c.contains("\"$X\"")),
            a => panic!("{a:?}"),
        }
        assert!(km.find_lone(keysym("Super_L").unwrap()).is_some());
        assert!(km.lock_command().unwrap().contains("call lock lock"));
        assert!(km.find(Mods::default(), keysym("XF86AudioRaiseVolume").unwrap(), false).is_some());
        // Virtual desktops: the keys and the names.
        let one = keysym("1").unwrap();
        let ls = Mods { logo: true, shift: true, ..Mods::default() };
        assert_eq!(km.find(ls, one, false).unwrap().actions, vec![Action::SendToDesktop(DeskTarget::Number(1), true, true)]);
        let lc = Mods { logo: true, ctrl: true, ..Mods::default() };
        assert_eq!(km.find(lc, keysym("Left").unwrap(), false).unwrap().actions, vec![Action::GoToDesktop(DeskTarget::Left, false)]);
        let lcs = Mods { logo: true, ctrl: true, shift: true, ..Mods::default() };
        assert_eq!(
            km.find(lcs, keysym("Right").unwrap(), false).unwrap().actions,
            vec![Action::SendToDesktop(DeskTarget::Right, false, false)]
        );
        assert_eq!(km.desktops, vec!["Work".to_string(), "Play".into(), "Desk 3".into()]);
    }

    #[test]
    fn the_way_out_is_ctrl_alt_backspace() {
        let ca = Mods { ctrl: true, alt: true, ..Mods::default() };
        assert!(leave_key(ca, Keysym::BackSpace));
        assert!(!leave_key(Mods { ctrl: true, ..Mods::default() }, Keysym::BackSpace));
        assert!(!leave_key(ca, Keysym::Delete));
    }

    #[test]
    fn key_specs() {
        let (m, k) = parse_key("C-A-t").unwrap();
        assert!(m.ctrl && m.alt && !m.logo && !m.shift);
        assert_eq!(k, keysym("t").unwrap());
        assert!(parse_key("Q-x").is_none());
        assert!(parse_key("W-nosuchkeyname").is_none());
    }

    #[test]
    fn broken_rc_is_empty_not_fatal() {
        let km = parse("<labwc_config><keyboard>");
        assert!(km.binds.is_empty());
        assert!(!km.skipped.is_empty());
    }
}
