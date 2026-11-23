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
    /// they kept what Describe it made instead (Hearth Ideas counts them: "kept by 15 of them")
    #[serde(default)]
    pub kept: bool,
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
        Idea { format: 1, part: part.into(), asked: clean(asked), needs, closest, sketch, hearth: hearth.into(), app: app.into(), kept: false }
    }

    /// Exactly what will be sent, in words (board 9's "Send it as an idea · exactly what goes"). The rest of the
    /// idea (what it needs, the closest recipe) is Hearth's own words about it, nothing of yours.
    pub fn preview(&self) -> String {
        format!(
            "Idea for: {}\nAsked: \"{}\"\nHearth OS {}{}\nSketch: {}\nNo name, no files, no account, nothing else.",
            crate::feature::part_label(&self.part),
            self.asked,
            if self.hearth.is_empty() { "?" } else { self.hearth.as_str() },
            if self.app.is_empty() { String::new() } else { format!(" · {}", self.app) },
            if self.sketch { "yes" } else { "none" }
        )
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
        let i = Idea::new("quickaccess", "Add a button that records the last 30 seconds", vec!["keeping the last moments".into()], None, false, "1.17", "Mantel in Rust 1.17.1");
        assert_eq!(i.preview(), "Idea for: Quick Access (Mantel)\nAsked: \"Add a button that records the last 30 seconds\"\nHearth OS 1.17 · Mantel in Rust 1.17.1\nSketch: none\nNo name, no files, no account, nothing else.");
        let back: Idea = serde_json::from_str(&i.to_json()).unwrap();
        assert_eq!(back, i);
    }
}
