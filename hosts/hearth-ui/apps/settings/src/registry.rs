//! Every Settings page (pages_list.rs, written from Mantel's settings-pages.js) and the 13 categories. A page is
//! built when one of the category modules (cats/*.rs, and quick.rs for the Quick tab) lists it in its PAGES: every
//! page is now (the test every_page_of_mantels_settings_is_built); one added to settings-pages.js later would show on
//! its category as "Coming next" and open nothing until it's built. The controller look (layout B) groups the
//! categories in 8 tabs (TABS, as settings-pages.js), each listing its pages on the left.

pub use crate::pages_list::{CATEGORIES, PAGES};

#[derive(Debug)]
pub struct Page {
    pub id: &'static str,
    pub cat: &'static str,
    pub label: &'static str,
    pub desc: &'static str,
    pub words: &'static str,
    pub parent: &'static str,
    pub hidden: bool,
    pub landing: bool,
    pub live: bool,
}

pub fn page(id: &str) -> Option<&'static Page> {
    PAGES.iter().find(|p| p.id == id)
}

pub fn cat_index(cat: &str) -> usize {
    CATEGORIES.iter().position(|c| c.0 == cat).unwrap_or(0)
}

pub fn cat_of(id: &str) -> usize {
    // A section's front page (cats/sections.rs: "sec-system") is in its section.
    if let Some(cat) = id.strip_prefix("sec-") {
        return cat_index(cat);
    }
    cat_index(page(id).map(|p| p.cat).unwrap_or("home"))
}

pub fn built(id: &str) -> bool {
    crate::cats::built(id) || crate::quick::PAGES.contains(&id)
}

/// The controller look's tabs (settings-B1/B2: Quick, System, Network, Devices, Gaming, Look, Accounts, Server) and the
/// categories in each (settings-pages.js TABS).
pub const TABS: &[(&str, &str, &[&str])] = &[
    ("quick", "Quick", &[]),
    ("system", "System", &["system", "home", "apps", "updates", "a11y"]),
    ("network", "Network", &["net"]),
    ("devices", "Devices", &["bt", "link"]),
    ("gaming", "Gaming", &["gaming"]),
    ("look", "Look", &["pers"]),
    ("accounts", "Accounts", &["accounts", "privacy"]),
    ("server", "Server", &["hs"]),
];

/// The tab a page is in (settings-pages.js tabOf).
pub fn tab_of(id: &str) -> usize {
    if id == "quick" {
        return 0;
    }
    let cat = page(id).map(|p| p.cat).unwrap_or("");
    TABS.iter().position(|t| t.2.contains(&cat)).unwrap_or(0)
}

/// A tab's pages, for its list on the left (settings-pages.js tabPages): each category's own page first when it's a
/// page of its own (Home, Wi-Fi; not a category's list of pages), then its pages. Only the ones built: the others are
/// on their category's page as "Coming next" in the other look.
pub fn tab_pages(tab: usize) -> Vec<&'static Page> {
    let mut out: Vec<&'static Page> = Vec::new();
    let Some(t) = TABS.get(tab) else { return out };
    for cat in t.2.iter() {
        let landing = CATEGORIES.iter().find(|c| c.0 == *cat).and_then(|c| page(c.2));
        if let Some(l) = landing {
            if built(l.id) && !out.iter().any(|x| x.id == l.id) {
                out.push(l);
            }
        }
        for p in PAGES.iter().filter(|p| p.cat == *cat && !p.landing && !p.hidden && p.parent.is_empty() && !p.live && built(p.id)) {
            if !out.iter().any(|x| x.id == p.id) {
                out.push(p);
            }
        }
    }
    out
}

/// The page a page is listed under on the left (itself, or the page above it).
pub fn top_of(id: &str) -> &'static str {
    chain(id).first().map(|p| p.id).unwrap_or("home")
}

/// The page and the pages above it (its parent, then the parent's parent), for the title's path.
pub fn chain(id: &str) -> Vec<&'static Page> {
    let mut v = Vec::new();
    let mut cur = page(id);
    while let Some(p) = cur {
        if v.iter().any(|x: &&Page| x.id == p.id) {
            break;
        }
        v.push(p);
        cur = if p.parent.is_empty() { None } else { page(p.parent) };
    }
    v.reverse();
    v
}

/// What `hearth-settings PAGE` asks for: a page id, "page:part" (the part is the page's to use), or a word.
pub fn resolve(arg: &str) -> Option<(&'static str, String)> {
    let mut it = arg.splitn(2, ':');
    let id = it.next().unwrap_or("");
    let part = it.next().unwrap_or("").to_string();
    if let Some(p) = page(id) {
        return Some((p.id, part));
    }
    // A section's front page ("sec-system", cats/sections.rs; "sec-ai", cats/ai.rs).
    if let Some(sec) = crate::cats::sections::PAGES.iter().chain(crate::cats::ai::PAGES.iter()).find(|x| **x == id) {
        return Some((sec, part));
    }
    PAGES.iter().find(|p| p.words.split(' ').any(|w| w == id)).map(|p| (p.id, part))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_category_and_page_is_there() {
        assert_eq!(CATEGORIES.len(), 14);
        for c in CATEGORIES {
            assert!(page(c.2).is_some(), "{} has no landing page {}", c.0, c.2);
        }
        for p in PAGES {
            assert!(CATEGORIES.iter().any(|c| c.0 == p.cat), "{} is in no category", p.id);
            assert!(p.parent.is_empty() || page(p.parent).is_some(), "{}'s parent {} isn't a page", p.id, p.parent);
        }
        assert_eq!(resolve("keyboard:osk").map(|r| r.0), Some("keyboard"));
        assert_eq!(chain("sound-apps").iter().map(|p| p.id).collect::<Vec<_>>(), vec!["sound", "sound-apps"]);
    }

    #[test]
    fn every_page_of_mantels_settings_is_built() {
        // Nothing is left as "Coming next": every page of Mantel's Settings (settings-pages.js, pages_list.rs) and of
        // KDE's System Settings (kde-settings.json) is drawn here; the categories' own pages are their lists.
        let missing: Vec<&str> = PAGES.iter().filter(|p| !p.landing && !built(p.id)).map(|p| p.id).collect();
        assert!(missing.is_empty(), "pages not built yet: {missing:?}");
        assert!(PAGES.len() > 150, "{}", PAGES.len());
    }

    #[test]
    fn the_controller_looks_tabs() {
        assert_eq!(TABS.len(), 8);
        // Every category is in one tab.
        for c in CATEGORIES {
            assert_eq!(TABS.iter().filter(|t| t.2.contains(&c.0)).count(), 1, "{}", c.0);
        }
        assert_eq!(tab_of("quick"), 0);
        assert_eq!(TABS[tab_of("display")].0, "system");
        assert_eq!(TABS[tab_of("wifi")].0, "network");
        let sys: Vec<&str> = tab_pages(1).iter().map(|p| p.id).collect();
        assert!(sys.contains(&"display") && sys.contains(&"home"), "{sys:?}");
        assert!(sys.iter().position(|x| *x == "display") < sys.iter().position(|x| *x == "home"));
        assert!(!sys.contains(&"quick"));
        assert_eq!(top_of("sound-apps"), "sound");
    }
}
