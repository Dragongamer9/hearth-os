//! Key codes (linux/input-event-codes.h), the names shown and saved for them, and the US layout for typing text
//! (the same table as hearth-pad's: ASCII; what it can't type is skipped).

pub const EV_SYN: u16 = 0;
pub const EV_KEY: u16 = 1;
pub const KEY_MACRO1: u16 = 0x290;
pub const KEY_MACRO_RECORD_START: u16 = 0x2b0;
pub const KEY_MACRO_PRESET1: u16 = 0x2b3;
pub const KEY_KBD_LCD_MENU1: u16 = 0x2b8;

pub const KEY_LEFTCTRL: u16 = 29;
pub const KEY_LEFTSHIFT: u16 = 42;
pub const KEY_LEFTALT: u16 = 56;
pub const KEY_LEFTMETA: u16 = 125;

/// What a key on the G-key part is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GKey {
    /// G1 … G18 (1-based).
    G(u8),
    /// M1 … M3 (1-based): the three key sets.
    M(u8),
    /// MR: record a macro.
    Mr,
    /// The keys under the screen, left to right: 1 is the round one.
    Lcd(u8),
}

/// An event code from "Logitech Gaming Keyboard Gaming Keys" as the key it is.
pub fn gkey(code: u16) -> Option<GKey> {
    match code {
        c if (KEY_MACRO1..KEY_MACRO1 + 18).contains(&c) => Some(GKey::G((c - KEY_MACRO1 + 1) as u8)),
        c if (KEY_MACRO_PRESET1..KEY_MACRO_PRESET1 + 3).contains(&c) => Some(GKey::M((c - KEY_MACRO_PRESET1 + 1) as u8)),
        KEY_MACRO_RECORD_START => Some(GKey::Mr),
        c if (KEY_KBD_LCD_MENU1..KEY_KBD_LCD_MENU1 + 5).contains(&c) => Some(GKey::Lcd((c - KEY_KBD_LCD_MENU1 + 1) as u8)),
        _ => None,
    }
}

/// Names for the keys a combination or a recorded macro uses (saved in g15.json, shown in the app).
pub const NAMES: &[(&str, u16)] = &[
    ("Esc", 1), ("1", 2), ("2", 3), ("3", 4), ("4", 5), ("5", 6), ("6", 7), ("7", 8), ("8", 9), ("9", 10), ("0", 11),
    ("-", 12), ("=", 13), ("Backspace", 14), ("Tab", 15), ("Q", 16), ("W", 17), ("E", 18), ("R", 19), ("T", 20),
    ("Y", 21), ("U", 22), ("I", 23), ("O", 24), ("P", 25), ("[", 26), ("]", 27), ("Enter", 28), ("Ctrl", 29),
    ("A", 30), ("S", 31), ("D", 32), ("F", 33), ("G", 34), ("H", 35), ("J", 36), ("K", 37), ("L", 38), (";", 39),
    ("'", 40), ("`", 41), ("Shift", 42), ("\\", 43), ("Z", 44), ("X", 45), ("C", 46), ("V", 47), ("B", 48), ("N", 49),
    ("M", 50), (",", 51), (".", 52), ("/", 53), ("RightShift", 54), ("Num*", 55), ("Alt", 56), ("Space", 57),
    ("CapsLock", 58), ("F1", 59), ("F2", 60), ("F3", 61), ("F4", 62), ("F5", 63), ("F6", 64), ("F7", 65), ("F8", 66),
    ("F9", 67), ("F10", 68), ("NumLock", 69), ("ScrollLock", 70), ("Num7", 71), ("Num8", 72), ("Num9", 73),
    ("Num-", 74), ("Num4", 75), ("Num5", 76), ("Num6", 77), ("Num+", 78), ("Num1", 79), ("Num2", 80), ("Num3", 81),
    ("Num0", 82), ("Num.", 83), ("F11", 87), ("F12", 88), ("NumEnter", 96), ("RightCtrl", 97), ("Num/", 98),
    ("Print", 99), ("AltGr", 100), ("Home", 102), ("Up", 103), ("PageUp", 104), ("Left", 105), ("Right", 106),
    ("End", 107), ("Down", 108), ("PageDown", 109), ("Insert", 110), ("Delete", 111), ("Mute", 113),
    ("VolumeDown", 114), ("VolumeUp", 115), ("Pause", 119), ("Super", 125), ("RightSuper", 126), ("Menu", 127),
    ("Next", 163), ("PlayPause", 164), ("Previous", 165), ("Stop", 166), ("F13", 183), ("F14", 184), ("F15", 185),
    ("F16", 186), ("F17", 187), ("F18", 188), ("F19", 189), ("F20", 190), ("F21", 191), ("F22", 192), ("F23", 193),
    ("F24", 194),
];

/// Other spellings people (and Mantel's mouse-button page) use for the same keys.
const ALIASES: &[(&str, &str)] = &[
    ("control", "Ctrl"), ("ctl", "Ctrl"), ("lctrl", "Ctrl"), ("rctrl", "RightCtrl"), ("meta", "Super"),
    ("win", "Super"), ("windows", "Super"), ("cmd", "Super"), ("escape", "Esc"), ("return", "Enter"), ("del", "Delete"),
    ("ins", "Insert"), ("pgup", "PageUp"), ("pgdown", "PageDown"), ("pgdn", "PageDown"), ("prtsc", "Print"),
    ("printscreen", "Print"), ("volume up", "VolumeUp"), ("volume down", "VolumeDown"), ("volume mute", "Mute"),
    ("media play", "PlayPause"), ("media next", "Next"), ("media previous", "Previous"), ("bksp", "Backspace"),
    ("plus", "="), ("minus", "-"),
];

pub fn code_of(name: &str) -> Option<u16> {
    let n = name.trim();
    if n.is_empty() {
        return None;
    }
    if let Some(&(_, c)) = NAMES.iter().find(|(k, _)| k.eq_ignore_ascii_case(n)) {
        return Some(c);
    }
    let l = n.to_ascii_lowercase();
    let real = ALIASES.iter().find(|(a, _)| *a == l)?.1;
    NAMES.iter().find(|(k, _)| *k == real).map(|&(_, c)| c)
}

pub fn name_of(code: u16) -> String {
    NAMES.iter().find(|(_, c)| *c == code).map(|(n, _)| n.to_string()).unwrap_or_else(|| format!("Key {code}"))
}

pub fn is_modifier(code: u16) -> bool {
    matches!(code, 29 | 42 | 54 | 56 | 97 | 100 | 125 | 126)
}

/// "Ctrl+Shift+T" → [29, 42, 20]. None when a part isn't a key.
pub fn combo(text: &str) -> Option<Vec<u16>> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    // "+" on its own, or at the end ("Ctrl++"), is the = key with Shift: keep it simple and take "+" as "=".
    let mut parts: Vec<String> = Vec::new();
    let mut cur = String::new();
    for ch in t.chars() {
        if ch == '+' && !cur.trim().is_empty() {
            parts.push(std::mem::take(&mut cur));
        } else {
            cur.push(ch);
        }
    }
    if !cur.is_empty() {
        parts.push(cur);
    }
    let v: Option<Vec<u16>> = parts.iter().map(|p| if p == "+" { Some(13) } else { code_of(p) }).collect();
    v.filter(|v| !v.is_empty())
}

/// [29, 42, 20] → "Ctrl+Shift+T".
pub fn combo_text(codes: &[u16]) -> String {
    codes.iter().map(|&c| name_of(c)).collect::<Vec<_>>().join("+")
}

/// A character on a US keyboard: (key, with Shift). Newline is Enter, tab is Tab.
pub fn char_key(ch: char) -> Option<(u16, bool)> {
    const SHIFTED: &[(char, char)] = &[
        ('`', '~'), ('1', '!'), ('2', '@'), ('3', '#'), ('4', '$'), ('5', '%'), ('6', '^'), ('7', '&'), ('8', '*'),
        ('9', '('), ('0', ')'), ('-', '_'), ('=', '+'), ('[', '{'), (']', '}'), ('\\', '|'), (';', ':'), ('\'', '"'),
        (',', '<'), ('.', '>'), ('/', '?'),
    ];
    match ch {
        '\n' => return Some((28, false)),
        '\t' => return Some((15, false)),
        ' ' => return Some((57, false)),
        'a'..='z' => return code_of(&ch.to_ascii_uppercase().to_string()).map(|c| (c, false)),
        'A'..='Z' => return code_of(&ch.to_string()).map(|c| (c, true)),
        _ => {}
    }
    if let Some(&(plain, _)) = SHIFTED.iter().find(|(_, s)| *s == ch) {
        return code_of(&plain.to_string()).map(|c| (c, true));
    }
    code_of(&ch.to_string()).filter(|&c| !is_modifier(c)).map(|c| (c, false))
}

/// The codes the virtual keyboard must be able to send: every key above.
pub fn all_codes() -> Vec<u16> {
    let mut v: Vec<u16> = NAMES.iter().map(|&(_, c)| c).collect();
    v.sort_unstable();
    v.dedup();
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maps_the_g15s_keys() {
        assert_eq!(gkey(0x290), Some(GKey::G(1)));
        assert_eq!(gkey(0x2a1), Some(GKey::G(18)));
        assert_eq!(gkey(0x2a2), None, "only 18 G-keys");
        assert_eq!(gkey(0x2b3), Some(GKey::M(1)));
        assert_eq!(gkey(0x2b5), Some(GKey::M(3)));
        assert_eq!(gkey(0x2b0), Some(GKey::Mr));
        assert_eq!(gkey(0x2b8), Some(GKey::Lcd(1)));
        assert_eq!(gkey(0x2bc), Some(GKey::Lcd(5)));
        assert_eq!(gkey(30), None);
    }
    #[test]
    fn reads_combinations() {
        assert_eq!(combo("Ctrl+Shift+T"), Some(vec![29, 42, 20]));
        assert_eq!(combo("ctrl+c"), Some(vec![29, 46]));
        assert_eq!(combo("Meta+W"), Some(vec![125, 17]));
        assert_eq!(combo("Volume Up"), Some(vec![115]));
        assert_eq!(combo("Ctrl+Nonsense"), None);
        assert_eq!(combo_text(&[29, 56, 111]), "Ctrl+Alt+Delete");
    }
    #[test]
    fn types_us_characters() {
        assert_eq!(char_key('a'), Some((30, false)));
        assert_eq!(char_key('A'), Some((30, true)));
        assert_eq!(char_key('@'), Some((3, true)));
        assert_eq!(char_key('.'), Some((52, false)));
        assert_eq!(char_key('\n'), Some((28, false)));
        assert_eq!(char_key('é'), None);
        for c in all_codes() {
            assert!(c < 0x2ff);
        }
    }
}
