//! An idea: what's sent to Hearth's makers when Describe it is asked for something Hearth can't do yet
//! (hearth-ideas board 1). It's shown to the person exactly as it will be sent, and only sent when they press Send.
//! It holds what was asked, where, the versions, whether a sketch came with it, and the closest thing Hearth could
//! make. Nothing personal: no names, accounts, files or device details, and anything in the words that looks like an
//! email address, a web address with a login, or a home folder is blanked before it's shown.

use serde::{Deserialize, Serialize};

use crate::feature::Feature;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Idea {
    pub format: u32,
    /// the part it was asked in (feature::PARTS, or an app's id)
    pub part: String,
    /// the words, cleaned (`clean`)
    pub asked: String,
    pub needs: Vec<String>,
    /// the closest Hearth could make now, as a recipe (so it can be built from)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closest: Option<Feature>,
    pub sketch: bool,
    pub hearth: String,
    pub app: String,
}

/// Up to 1000 letters, with email addresses, logins in web addresses and home folders blanked.
pub fn clean(text: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    for w in text.split_whitespace() {
        let bare = w.trim_matches(|c: char| !c.is_alphanumeric() && c != '/' && c != '~' && c != '@');
        let personal = (bare.contains('@') && bare.contains('.') && !bare.starts_with('@'))
            || ((bare.starts_with("http://") || bare.starts_with("https://")) && bare.split('/').nth(2).map(|h| h.contains('@')).unwrap_or(false))
            || bare.starts_with("/home/")
            || bare.starts_with("/var/home/")
            || bare.starts_with("~/");
        out.push(if personal { "[left out]".into() } else { w.to_string() });
    }
    out.join(" ").chars().take(1000).collect()
}

impl Idea {
    pub fn new(part: &str, asked: &str, needs: Vec<String>, closest: Option<Feature>, sketch: bool, hearth: &str, app: &str) -> Idea {
        Idea { format: 1, part: part.into(), asked: clean(asked), needs, closest, sketch, hearth: hearth.into(), app: app.into() }
    }

    /// Exactly what will be sent, in words (the "What's sent" box).
    pub fn preview(&self) -> String {
        let mut t = format!("Asked in: {}\nAsked for: {}\n", self.part, self.asked);
        if !self.needs.is_empty() {
            t += &format!("Needs new work: {}\n", self.needs.join("; "));
        }
        if let Some(c) = &self.closest {
            t += &format!("Closest Hearth could make: {}\n", crate::feature::describe(c));
        }
        t += &format!("Sketch: {}\nHearth OS {} · {} {}\n", if self.sketch { "yes" } else { "no" }, self.hearth, self.part, self.app);
        t + "Nothing else is sent: no name, account, files or device details."
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn personal_bits_are_left_out() {
        let t = clean("put my files from /home/dragon/Games and ~/Music, mail me at someone@example.com, see https://me:pw@x.org/a or https://hearthos.fun");
        assert!(!t.contains("dragon") && !t.contains("Music") && !t.contains("someone") && !t.contains("pw@"));
        assert!(t.contains("https://hearthos.fun"));
        assert!(t.contains("[left out]"));
        assert_eq!(clean(&"a".repeat(5000)).len(), 1000);
    }

    #[test]
    fn the_preview_is_what_is_sent() {
        let i = Idea::new("calendar", "Show moon phases", vec!["showing \"moon-phase\"".into()], None, true, "1.17", "1.17.1");
        let p = i.preview();
        assert!(p.contains("Show moon phases") && p.contains("moon-phase") && p.contains("Sketch: yes") && p.contains("Hearth OS 1.17"));
        let back: Idea = serde_json::from_str(&i.to_json()).unwrap();
        assert_eq!(back, i);
    }
}
