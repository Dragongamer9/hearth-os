# Quick updates in the Hearth Store (approved Nov 21, 2026: "looks good"; source: hearth-workspace hearth/mockups/1.17-quick-updates)

Mockups: 1.17-quick-updates/quick-updates-1-updates.png, 1.17-quick-updates/quick-updates-2-app-page.png
Release: 1.17

The Store's side is hosts/hearth-ui/apps/store/src/parts.rs, main.rs (PartUpdate, PartRemove, PartRetry) and view.rs
(updates, part_page); the work is /usr/libexec/hearth/hearth-part (os-bases/anatase/system_files), which every app's
start and Mantel's autostart also ask. The names in "Hearth's own apps and desktops" come from the Store's list.

## 1. Updates
- [ ] Update all (n) counts quick updates too — view.rs: "let n = ready.len() + offers.len()"
- [ ] The sidebar's Updates count includes them — view.rs: "a.ready().len() + a.part_offers().len()"
- [ ] A quick update's card first, the first one open with what's new — view.rs: "Quick updates for Hearth's own apps and Mantel"
- [ ] Its line: from → to · size · "quick update: no Hearth OS update, no restart" (Mantel) / "quick update · opens with it next time" — view.rs: "quick update · opens with it next time"
- [ ] Hide what's new ▴ / What's new ▾ and Update on each — view.rs: "Msg::PartUpdate(e.id.clone())"
- [ ] What's new in <version> with its lines, and Mantel's restart note — view.rs: "Your desktop restarts in a moment to use it"
- [ ] A closed card for the next one (smaller icon, one line) — view.rs: "let big = shown"
- [ ] Apps that update themselves as before (Command Center) — view.rs: "this app updates itself"
- [ ] Section "Hearth's own apps and desktops" with the up-to-date row and its chip — view.rs: "Hearth's own apps and desktops"
- [ ] A row for one that went back to the built-in one, with Try it again and Remove the update — view.rs: "went back to the built-in one"
- [ ] Test versions as before — view.rs: "Test versions are off on this PC"
- [ ] Hints A Update, B Back, X Update all, Y What's new, LB RB as before

## 2. An app's page
- [ ] "‹ Built into Hearth", the icon, name, "Built into Hearth OS · by Dragongamer9", Open — view.rs: "Built into Hearth OS · by"
- [ ] The Quick updates card: its text and its three points — view.rs: "Fixes for Hearth's own apps and desktops come here"
- [ ] Running (version · quick update, date), Built into Hearth OS, Installs as (no password, no restart), Checked with — view.rs: "The signed list from hearthos.fun"
- [ ] Use the built-in one and What's new — view.rs: "Use the built-in one"
