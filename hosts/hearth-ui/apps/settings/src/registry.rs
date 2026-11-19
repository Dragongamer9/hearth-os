//! Every Settings page (pages_list.rs, written from Mantel's settings-pages.js) and the 13 categories. A page is
//! built when one of the category modules (cats/*.rs) lists it in its PAGES; the others show on their category as
//! "Coming next" and open nothing.

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
    cat_index(page(id).map(|p| p.cat).unwrap_or("home"))
}

pub fn built(id: &str) -> bool {
    crate::cats::built(id)
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
    PAGES.iter().find(|p| p.words.split(' ').any(|w| w == id)).map(|p| (p.id, part))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_category_and_page_is_there() {
        assert_eq!(CATEGORIES.len(), 13);
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
}
