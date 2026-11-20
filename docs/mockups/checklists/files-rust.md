# Hearth Files and the Open / Save window in pure Rust (the approved Files and Hearth (C:) mockups, ported from Mantel)

Mockups: next-update/files-1-A-windows11-nav.png, next-update/files-3-A-home-page.png, next-update/files-4-A-narrow.png, next-update/files-5-this-pc.png, next-update/files-6-pictures-details-pane.png, next-update/files-7-right-click.png, next-update/files-8-view-menu.png, next-update/cdrive-1-save-window.png, next-update/cdrive-3-properties.png
Release: 1.16

The pure-Rust Hearth Files (hosts/hearth-ui/apps/files: hearth-files-app, and hearth-file-chooser for every app's Open /
Save window) for Hearth OS, Fedora edition, where Mantel's QML Files can't run. Same backend (desktop/shell/hearth-files),
same settings file (files.json), same mockups as hearth-files.md and c-drive.md. Parts are ticked only once they're
built and pictured in CI (hearth-apps-rust.yml, .github/scripts/hearth-apps-rust-shots.d/files.sh).

## files-1 / files-4 (the window, the left menu)
- [x] Tabs on top, like Windows 11 (files-6): pictured files-4-pictures-pane — hosts/hearth-ui/apps/files/src/view.rs: "fn tab_strip(a: &App)"
- [x] Back, forward, up, refresh; the address with its steps (This PC › Hearth (C:) › Users › chris); Search chris — hosts/hearth-ui/apps/files/src/view.rs: "nb(Ic::Refresh, true, Msg::Refresh)"
- [x] Command bar: New ▾, cut, copy, paste, rename, share, delete, Sort ▾, View ▾, Details, ··· — hosts/hearth-ui/apps/files/src/view.rs: "lb(Ic::Plus, \"New\", true, w, \"new\")"
- [x] Left menu: Home, your folders, This PC with Hearth (C:) open (Users, Program Files, Windows apps), the other drives, Network, Linux at the bottom. The mockup's Gallery and Home server entries: the owner took them off on Nov 19 ("there must be only one" of Home server and Network, keep Network; Gallery off the side menu, Pictures stays a folder): the home server's shared folders are listed first inside Network — hosts/hearth-ui/apps/files/src/app.rs: "label: \"Network\".into()"
- [x] Narrow left menu (158 px, the approved option A) without the pins and notes — hosts/hearth-ui/apps/files/src/view.rs: "let wide = a.nav_w >= 200.0;"
- [x] The wider left menu (dragged past 200 px) with the pins and the "hidden" note (pictured files-17-wide-nav, --show widenav) — hosts/hearth-ui/apps/files/src/view.rs: "if wide && q.pinned {"
- [x] Details: Name, Date modified, Type, Size — hosts/hearth-ui/apps/files/src/view.rs: "(\"date\", \"Date modified\", 160.0)"
- [x] Windows' type names (File folder, Python file) — hosts/hearth-ui/apps/files/src/model.rs: "\"py\" => \"Python file\""
- [x] Status bar: "N items" and "C: … free of …" — hosts/hearth-ui/apps/files/src/view.rs: "{letter}: {} free of {}"

## files-3 (Home)
- [x] QUICK ACCESS: your folders as tiles, "Pinned" (Saved Games: "Pinned · N games") — hosts/hearth-ui/apps/files/src/view.rs: "format!(\"Pinned · {} games\", a.saved_games)"
- [x] RECENT: name, Today / Yesterday and the time, the C:\ folder — hosts/hearth-ui/apps/files/src/view.rs: "model::fmt_date(e.mtime, true)"

## files-5 (This PC)
- [x] DRIVES as tiles with how full they are and "86 GB free of 144 GB" — hosts/hearth-ui/apps/files/src/view.rs: "fn pc_tile<'a>"
- [x] The bar turns amber when a drive is nearly full (pictured files-18-thispc-full: a stand-in drive 88 % full) — hosts/hearth-ui/apps/files/src/view.rs: "} else if used > 0.8 { p.warn }"
- [x] NETWORK LOCATIONS: the home server's drive with its letter and bar, Hearth Link devices (pictured files-18-thispc-full with CI's stand-in for hearth-files, files-fixture.py) — hosts/hearth-ui/apps/files/src/view.rs: "(\"net\", \"Network locations\")"

## files-6 (Pictures, large icons, details pane)
- [x] Large icons with the pictures' thumbnails; the picked one outlined — hosts/hearth-ui/apps/files/src/view.rs: "View::XLarge | View::Large =>"
- [x] Details pane: preview, name, "PNG image · 6 KB", Size (960 × 540), Taken, Where (C:\Users\…\Pictures), Previous versions — hosts/hearth-ui/apps/files/src/view.rs: "rows.push((\"Previous versions\".into()"
- [x] Details pane: Set as wallpaper · Open with ▾ — hosts/hearth-ui/apps/files/src/view.rs: "link(\"Set as wallpaper\", Msg::Wallpaper)"
- [x] Status bar: "10 items · 1 selected, 6 KB" — hosts/hearth-ui/apps/files/src/view.rs: "{s} · {} selected{}"

## files-7 (right-click)
- [x] The icons on top: cut, copy, rename, copy as path, delete — hosts/hearth-ui/apps/files/src/menus.rs: "MenuItem::new(\"copywin\", \"Copy as path\").icon(Ic::Share).quick()"
- [x] Open (Enter), Open in new tab, Pin to the left menu, Compress to ZIP, Lock as a Briefcase, Open in Terminal, Restore previous versions, Copy as path (C:\…); Open with ▸ for files — hosts/hearth-ui/apps/files/src/menus.rs: "MenuItem::new(\"lockbc\", \"Lock as a Briefcase\")"
- [x] Properties (Alt+Enter), Show more options (the whole menu) — hosts/hearth-ui/apps/files/src/menus.rs: "MenuItem::new(\"more\", \"Show more options\")"

## files-8 (View)
- [x] LAYOUT: Extra large icons, Large icons, List, Details (ticked), Compact view (and Medium, Small icons, Tiles, as Mantel has) — hosts/hearth-ui/apps/files/src/menus.rs: "MenuItem::new(\"compactview\", \"Compact view\")"
- [x] SHOW: Navigation pane, Details pane, File name extensions, Hidden items, Linux folders (/, /home, /usr…), Linux paths in the address bar — hosts/hearth-ui/apps/files/src/menus.rs: "\"Linux paths in the address bar\""

## cdrive-1 (every app's Save window)
- [x] "Save As – Firefox" with the C:\ address on the same row — hosts/hearth-ui/apps/files/src/view.rs: "fn pick_head(a: &App)"
- [x] Left menu: Home, your folders, This PC with the drives, Network; no Linux folders (Gallery taken off by the owner, Nov 19) — hosts/hearth-ui/apps/files/src/app.rs: "if sys && c_open && !self.picking()"
- [x] Name, Date modified, Type, Size with Windows' type names (Disc image, PDF document, Python file) — hosts/hearth-ui/apps/files/src/model.rs: "\"iso\" => \"Disc image\""
- [x] File name box, Cancel, Save — hosts/hearth-ui/apps/files/src/view.rs: "r = r.push(t1(\"File name\", 15.0, p.text));"
- [x] hearth-portal opens it where there's no Quickshell (pictured chooser-4-portal: hearth-portal's own run_picker started it), and Plasma's kde-portals.conf sends apps' Open / Save here, KDE's dialog the fallback (unit-tested: test_hearth_portal.py; still to try on a device) — .github/scripts/hearth-apps-rust-shots.d/files.sh: "hp.run_picker(req, env=dict(os.environ), game=False)"

## cdrive-3 (Properties)
- [x] Type, Location (C:\Users\…\Desktop), Size, Created — hosts/hearth-ui/apps/files/src/view.rs: "body = body.push(prow(&p, \"Location\", ptext(&p, loc)));"
- [x] Linux path with "only with Show system folders" — hosts/hearth-ui/apps/files/src/view.rs: "\"only with Show system folders\""
- [x] Copy path → C:\Users\…\Desktop\Budget.ods — hosts/hearth-ui/apps/files/src/view.rs: "format!(\"Copy path → {}\""

## Mantel's Files beyond the mockups (FileManager.qml, Files*.qml)
- [x] Terminal panel under the files (F4, FilesTerminal.qml): runs what you type, cd moves Files, Up / Down history, Ctrl+C, Esc, Open in Terminal (pictured files-19-terminal) — hosts/hearth-ui/apps/files/src/terminal.rs: "\"Still running the last one (Ctrl+C stops it)\""
- [x] Split view (F9, FilesSplitPane.qml): up, open, Copy here / Move here (n), middle-click opens on the left, drops (pictured files-20-split) — hosts/hearth-ui/apps/files/src/split.rs: "format!(\"Copy here{n}\")"
- [x] Tags and git badges on icons (FilesMarks.qml, FilesExtras.qml), Tags › and the Tags in the left menu, the branch in the status bar, Act as administrator (red bar, pkexec hearth-files-admin), sharing a folder (Properties › Sharing) (pictured files-21 … files-25) — hosts/hearth-ui/apps/files/src/extras.rs: "\"Acting as administrator in system folders\""
- [x] Briefcase statuses and dialogs (Briefcase.qml, BriefcaseDialog.qml): Status column and badges, Update All / Unlock / Lock / Customize buttons, the menus, Update All, Unlock, Lock with a password, Change password, Customize, choosing a Briefcase, jobs with Stop; Lock as a Briefcase makes it, adds the folder and sets up its lock (pictured files-26 … files-33) — hosts/hearth-ui/apps/files/src/briefcase.rs: "\"Lock “{}” with a password\""
- [x] Drag and drop inside the window: onto a folder, a tab, an address step, a place, the split view, a Briefcase or the Trash (moved; copied onto another drive or with Ctrl; Ctrl+Shift links), and files dropped from other windows are copied in (pictured files-35-drag) — hosts/hearth-ui/apps/files/src/dnd.rs: "\"Drop it on a folder\""
- [ ] Dragging files out of Hearth Files into another app's window: iced 0.14 (winit) can't start a drag that leaves the window; needs the owner's decision (a toolkit change, or leave it to Copy and Paste, which works between apps)
- [x] Your devices' shared folders (net-open, sign in with a sharing password, Remember in the keyring or Just this time, Wake it, which shared folder, Connect automatically) (pictured files-34-netsign) — hosts/hearth-ui/apps/files/src/extras.rs: "\"Just this time\".into()"
- [x] The controller and Hearth's on-screen keyboard inside the Open / Save window (Game Mode, HEARTH_PICKER_PAD=1: its own hearth-pad, the shared keyboard engine) (pictured chooser-3-keyboard; still to try with a controller on a device) — hosts/hearth-ui/apps/files/src/pad.rs: "send(json!({ \"cmd\": \"keys\", \"on\": true }));"

## Where it stands (feature/files-rust)
Pictured in CI (hearth-apps-rust.yml run 44448907206, files-*.png and chooser-*.png) and opened next to each mockup.
First frame 30–51 ms, 0.00 % of a core at rest, about 23 MB. Opening Files writes nothing (files.sh checks files.json).

## Where it stands (feature/files-rust-finish, Nov 20)
Mantel's Files beyond the mockups ported (terminal.rs, split.rs, extras.rs, briefcase.rs, dnd.rs, pad.rs and
osk_view.rs, the keyboard engine shared with hearth-pad-host by path) and pictured in CI (files-17 … files-35,
chooser-3, chooser-4; files-fixture.py stands in for hearth-files where CI has no hardware or network). Files at rest
0.00 % of a core, 26 MB. Open: dragging out to other apps (iced can't start one); to try on a device: the controller
in Game Mode's Open / Save window, hearth-portal through D-Bus, a real locked Briefcase (CI has no gocryptfs).
