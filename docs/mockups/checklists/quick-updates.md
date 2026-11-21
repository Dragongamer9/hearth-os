# Quick updates in the Hearth Store (approved Nov 21, 2026: "looks good"; source: hearth-workspace hearth/mockups/1.17-quick-updates)

Mockups: 1.17-quick-updates/quick-updates-1-updates.png, 1.17-quick-updates/quick-updates-2-app-page.png
Release: 1.17

The Store's side is hosts/hearth-ui/apps/store/src/parts.rs, main.rs (PartUpdate, PartRemove, PartRetry) and view.rs
(updates, part_page); the work is /usr/libexec/hearth/hearth-part (os-bases/anatase/system_files), which every app's
start and Mantel's autostart also ask. Pictured in CI (hearth-apps-rust 44896710335): store-14-quick-updates and
store-15-quick-app-page, opened next to the mockups on Nov 21. The names in "Hearth's own apps and desktops" come from the Store's list.

## 1. Updates
- [x] Update all (n) counts quick updates too — hosts/hearth-ui/apps/store/src/view.rs: "let n = ready.len() + offers.len()"
- [x] The sidebar's Updates count includes them — hosts/hearth-ui/apps/store/src/view.rs: "a.ready().len() + a.part_offers().len()"
- [x] A quick update's card first, the first one open with what's new — hosts/hearth-ui/apps/store/src/view.rs: "Quick updates for Hearth's own apps and Mantel"
- [x] Its line: from → to · size · "quick update: no Hearth OS update, no restart" (Mantel) / "quick update · opens with it next time" — hosts/hearth-ui/apps/store/src/view.rs: "quick update · opens with it next time"
- [x] Hide what's new ▴ / What's new ▾ and Update on each — hosts/hearth-ui/apps/store/src/view.rs: "Msg::PartUpdate(e.id.clone())"
- [x] What's new in <version> with its lines, and Mantel's restart note — hosts/hearth-ui/apps/store/src/view.rs: "Your desktop restarts in a moment to use it"
- [x] A closed card for the next one (smaller icon, one line) — hosts/hearth-ui/apps/store/src/view.rs: "let big = shown"
- [x] Apps that update themselves as before (Command Center) — hosts/hearth-ui/apps/store/src/view.rs: "this app updates itself"
- [x] Section "Hearth's own apps and desktops" with the up-to-date row and its chip — hosts/hearth-ui/apps/store/src/view.rs: "Hearth's own apps and desktops"
- [x] A row for one that went back to the built-in one, with Try it again and Remove the update — hosts/hearth-ui/apps/store/src/view.rs: "went back to the built-in one"
- [x] Test versions as before — hosts/hearth-ui/apps/store/src/view.rs: "Test versions are off on this PC"
- [x] Hints A Update, B Back, X Update all, Y What's new, LB RB as before — hosts/hearth-ui/apps/store/src/view.rs: "hint(&p, \"Y\", \"What's new\")"

## 2. An app's page
- [x] "‹ Built into Hearth", the icon, name, "Built into Hearth OS · by Dragongamer9", Open — hosts/hearth-ui/apps/store/src/view.rs: "Built into Hearth OS · by"
- [x] The Quick updates card: its text and its three points — hosts/hearth-ui/apps/store/src/view.rs: "Fixes for Hearth's own apps and desktops come here"
- [x] Running (version · quick update, date), Built into Hearth OS, Installs as (no password, no restart), Checked with — hosts/hearth-ui/apps/store/src/view.rs: "The signed list from hearthos.fun"
- [x] Use the built-in one and What's new — hosts/hearth-ui/apps/store/src/view.rs: "Use the built-in one"
