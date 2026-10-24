# Mantel parity: Hearth OS vs Windows (Ember)

Mantel on Hearth OS is Quickshell QML (`hosts/linux/desktop/shell/`) on labwc (`hosts/linux/desktop/labwc/`).
Mantel on Windows is HTML in Ember's WebView2 windows (`ui/mantel.html`, `ui/mantel-settings.html`,
`ui/mantel-tasks.html`, `ui/mantel-files.html`, `ui/mantel-apps.html`) hosted by `hosts/windows/Nimbus.Host/Mantel/`.
The goal is that both look and behave the same; this table is the audit (October 2026) and what's left. The second
round (after Hearth OS's "Mantel overhaul", f3e0556) added the taskbar's pins and menus, virtual desktops, desktop
icons, the clock's panel, clipboard history, Apps and six Settings pages; `tools/mantel-check.js` (a release check)
drives every piece through a stand-in for Ember's host.

Status: **same** = matches the Linux piece; **done** = was different, now ported; **differs** = still
different, with the reason.

## Look (Theme.qml)

| Item | Hearth OS | Windows before | Now |
|---|---|---|---|
| Colors (bg, panel, panel2/3, line, text, muted, danger, ember, flame) | Theme.qml | Same hex values | **same** |
| Accent | Follows the profile (library.json) | Follows Ember's profile | **same** (Windows also has an accent picker under Settings › Appearance › Windows) |
| Accent ink (text on accent) | Luminance > 0.45 → dark | Same formula | **same** |
| Glass panels | panel at 92 % | panel at 97 % (WebView can't see through the window) | **differs**: Windows windows are opaque; 97 % keeps the same tone on the dim backdrop |
| Corner radii | 14 bar, 20 panels, 12 rows, 10 buttons | Same (window regions + CSS) | **same** |
| Motion | quick 140 ms, calm 260 ms | Same variables, but Quick Access didn't slide and the Hearth menu didn't pop | **done**: Quick Access slides in from the right (calm, OutCubic), the Hearth menu pops (scale .85→1, OutBack) |
| Font | `sans-serif` (Bazzite's default sans) | Segoe UI Variable | **differs**: each uses the system's sans; no font is bundled (same sizes and weights) |
| Icons | Hearth's own SVG set (`shell/icons`) | Hand-drawn look-alikes | **done**: `ui/mantel-icons.js` is generated from the Linux icons (all 56 shapes, including the overhaul's bell, calendar, clipboard, desktops, pin/unpin, folder, file, trash, printer, store…); every Mantel page uses it |
| Hearth mark | `images/hearth-logo.png` | `ui/hearth-logo.svg` (the same art, vector) | **same** |
| Window decorations | labwc theme (10 px radius, Hearth colors) for every app | Windows 11 title bars; Hearth Settings gets Hearth caption colors | **differs**: Windows doesn't let a shell re-theme other apps' title bars (Hearth Settings, Task Manager, Files and Apps get Hearth's caption colors) |
| Right-click menus (PopMenu.qml) | Glass card 268 wide, radius 14, title, 40 px rows (icon, label, ✓, ›), separators, submenus; mouse, keys, controller | — | **done**: `mantel.html?part=menu`, its own window shaped like the card; the page that opens it keeps what each item does (`mantel.menuPick`). Same keys: Up/Down, Enter (A), Right into a submenu, Left / Esc (B) back, then closed; a click elsewhere closes it |
| Small dialogs (DeskDialog.qml) | Rename / new folder box, details card | — | **done**: `part=dialog` over a dimmed screen; Enter/Esc/Tab, "Controller: Y opens the keyboard" (Y types into the dialog) |

## Taskbar (Bar.qml)

| Item | Hearth OS | Windows before | Now |
|---|---|---|---|
| Shape and size | 60 px strip, floating bar 8/4/6 px in, radius 14, down the left side on upright screens | Same | **same** |
| Launcher button | Hearth mark, 32 px, "Apps (Super)" | Same, "Apps (Windows key)" | **same** (the key's Windows name) |
| Apps on the taskbar | Pinned apps first (in pin order), then open apps; one button per app (its windows share it); marks: 16 px accent in front, 10 px for several windows, 5 px for one | Pinned apps (matched to windows by a guess at the name), every window its own button | **done**: windows are matched to their app by Windows' app id (AppUserModelID, set on the window or the packaged app's) or the program behind a Start menu entry, so each app has one button and a pin lights up when its app runs; same three marks; pins keep their icon when the app isn't running |
| Click / middle-click | Click: open (no windows), focus (in front: minimize), several windows: a list of them to pick from (Super+1-9 steps through them); middle-click opens another window; pinned apps can be dragged into a new order | Click focused one window (and the taskbar took the front, so "in front: minimize" never happened) | **same** (October 2026 "normal desktop" round): same click, list, middle-click and dragging; Windows+1-9 in full Mantel |
| Right-click an app (X on the controller, or press and hold) | New window / Open, Pin to / Unpin from taskbar, Move to desktop ›, Maximize / Restore size, Close window / Close all N windows | Nothing (the browser's menu was turned off) | **done**, same items and words. Move to desktop: see Virtual desktops |
| Pins | `hearth-desk pin/unpin` (desk.json), up to 40 | `mantel.json`, from the launcher's hover button only | **done**: the same three ways to pin as Hearth OS (taskbar menu, launcher menu), up to 40; a pin is a Start menu app id, a program's path (from a running window) or one of Hearth's apps |
| Virtual desktops (DeskSwitcher.qml) | Numbered pills (24 px, accent on the current), click switches, scroll steps; hidden with one desktop | — | **done** over Explorer: Windows' own desktops (list and names read from Explorer's registry values, the current one too). Switching types Win+Ctrl+Left/Right (Windows has no public switch API). **Differs**: moving a window uses IVirtualDesktopManager, which Windows only allows for the caller's own windows, so for other apps Mantel says to drag it in Task View (Win+Tab); desktops are made in Task View. Full Mantel has no Explorer and so no virtual desktops: the switcher hides |
| Notifications bell | Bell (crossed out with Do Not Disturb), accent dot for unseen ones; click opens the clock's panel, right-click toggles Do Not Disturb | — | **done** |
| Own apps' icons | Hearth Settings / Task Manager / Files show the Hearth mark | Ember's exe icon | **done**: Hearth Settings, Task Manager, Files and Apps show the Hearth mark and share a button with their pin |
| Tray icons | SNI items, 30 px wide; click activates (or its menu), right-click its menu, middle-click the second action, scroll, tooltip with description | Full Mantel: hosted tray (click, right-click, double-click); on top of Explorer: a `^` button that shows Windows' tray for 10 s | **done** in Full Mantel: middle-click added; **differs**: Windows tray icons have no scroll action and one-line tooltips. The `^` is Windows-only (Explorer owns the tray) |
| Volume | Icon (high > 66, medium > 33, low, muted) + %, click opens Quick Access, middle-click mutes, right-click: sound output, Mute, Sound settings; scroll ±5 % | Showed "muted" at 0 % | **same**, except the right-click menu offers Windows' Sound settings and Volume mixer instead of a list of outputs |
| Network | Wired → wired icon, Wi-Fi connected → Wi-Fi, else offline; click opens Quick Access | Wi-Fi on but not connected showed Wi-Fi; wired never detected | **done**: `sys.status` reports `wired`; same icon rules and tooltip |
| Battery | > 60 good, > 20 low, else caution; charging icon | Threshold 20 only | **done** |
| Quick Access, Keyboard buttons | open-menu and keyboard icons | Keyboard opened Windows' touch keyboard / osk.exe | **done**: opens Hearth's on-screen keyboard (see below) |
| Clock | h:mm AP (HH:mm with Settings › Date & time › 24-hour) bold 14 / ddd, MMM d 11 muted; h:mm only when upright; click opens the clock's panel | Same, 12-hour only, no click | **done** |
| Game Mode button | Accent, 36 px, "Game Mode" / pad icon when upright | Same | **same** |
| Hover labels | Custom popup above the bar (panel2, radius 8) | Browser `title` tooltips | **differs**: the bar is its own 60 px window; a label above it would need another window per hover. Native tooltips show the same text |

## Launcher (Launcher.qml)

| Item | Hearth OS | Windows before | Now |
|---|---|---|---|
| Box | 880×640 max, radius 20, margins 22, dim #b0080b10 | Same | **same** |
| Search | 48 px, bg2, accent border when focused, "Search apps" | Same | **same** |
| Matching | Name, generic name, comment, keywords; names starting with the query first | Name only | **differs**: Windows' Start apps list only has names |
| Grid | 120 px cells, 112 tall, 48 px icons, 2-line names, highlight panel3 + accent border | Same | **same** |
| Keys | Arrows, Enter, Esc; Right/Left at the end of the text move the grid | Same (Esc clears the query first) | **same** |
| Hearth's apps | "Files", "Task Manager", "Hearth Settings" with the Hearth mark | "Hearth Files", "Hearth Task Manager" with their own icons | **done**: same names and icon; Hearth Settings added. Windows' own Task Manager also appears (from Start) with its own icon |
| Actions row | Game Mode (accent), Settings, Sleep, Restart, Shut down, Log out | Same | **same** |
| Pin to taskbar | Right-click, press and hold, or the Menu key (X on the controller): Open / Pin to taskbar; a small muted pin on pinned apps | A pin button that only showed while the mouse was over a cell | **done**: the same menu (right-click, hold, Menu key, Shift+F10, X on the controller) and the same pin mark; the hover button is gone |
| Apps and Clipboard history | Apps opens by IPC; clipboard history on Super+V | — | **differs, on purpose**: both are in the launcher as Hearth apps, because Windows+V is Windows' own picker (and stays so) |
| "Finding your apps…" | — (apps are known at once) | Spinner while Windows lists apps | Windows-only, kept |

## Quick Access (QuickAccess.qml)

| Item | Hearth OS | Windows before | Now |
|---|---|---|---|
| Panel | 400 wide, 12 from the right/top, 72 from the bottom, radius 20, slides in | Same place, appeared at once | **done**: slides in |
| Header | Time 26 semibold, battery icon + % | Charging icon when charging | **done**: same icon as Linux |
| Volume / Brightness sliders | 64 px, ember→flame bar, icon click mutes | Same | **same** (Windows can't go past 100 %: no over-amplification) |
| Night light toggle | wlsunset, "Sunset to sunrise" / "Not installed" | Opened Windows' Night light settings, "In Windows Settings" | **done**: Hearth's own night light (gamma ramp, 3700 K, 19:30–07:00, 30 min fades) while Mantel runs; "Not available" if the display refuses gamma changes |
| Controller mouse toggle | On/Off | Same | **same** |
| Wi-Fi toggle | Detail On/Off | Detail was the network name | **done** |
| Performance choices | Battery saver / Balanced / Performance | Same (Windows' power mode slider) | **same** |
| Actions | Settings, Keyboard, Game Mode (accent), Sleep (sleep icon) | Sleep used the moon icon | **done** |
| Controller | D-pad rows/columns, Left/Right change sliders, A, B | Same | **same** |

## Power menu (PowerMenu.qml)

| Item | Hearth OS | Windows before | Now |
|---|---|---|---|
| Card, items, keys | 360 wide, Lock / Sleep / Restart / Shut down / Game Mode / Cancel, hover selects | Same | **same** |
| Lock | Mantel's own lock screen | Windows' lock screen | see Lock screen |

## Hearth menu (RadialMenu.qml)

| Item | Hearth OS | Windows before | Now |
|---|---|---|---|
| Ring, 8 items, centre mark and label | 62 % of the short side, glass, ember glow | Same | **same** |
| Hint | "Stick to choose · A select · B close" | "D-pad to choose…" | **done** |
| Choosing | Either stick points at any of the 8 items (hearth-pad), D-pad, LB/RB | D-pad: only 4 items reachable | **done**: two directions together (stick or D-pad diagonal) pick the item between them, so all 8 are reachable; LB/RB step |
| Opening animation | Pop | None | **done** |

## Controls card and messages (Help.qml)

| Item | Hearth OS | Windows before | Now |
|---|---|---|---|
| Header | Hearth mark + "Controller controls" | Title only | **done** |
| List | 13 rows (X / LT right-click, Guide, "press it in: precise") | Different list (LT only, "Xbox button", View + Menu row) | **done**: word for word; the controller now does the same (below). View + Menu still toggles the controller mouse on Windows (not listed, like on Hearth OS) |
| Closing | Click anywhere, or after 12 s | Esc/Enter only | **done** |
| Messages ("Precise pointer on", "Controller mouse off") | Glass pill at the top, 44 px, 1.8 s | Blue-grey Ember box at the bottom | **done** while Mantel runs (`Osd.HearthStyle`); Game Mode keeps Ember's look |

## On-screen keyboard (Keyboard.qml + hearth-pad)

Since the on-screen keyboard overhaul, Hearth OS's Desktop Mode uses Hearth's shared keyboard (ui/osk, docs/OSK.md:
suggestions, accents, emoji, layouts, the Ember Wheel; typing through wtype). Ember's Mantel still has the older
US-layout keyboard below; moving it to ui/osk (KeyTyper already types any character) is still to do.

| Item | Hearth OS | Windows before | Now |
|---|---|---|---|
| Keyboard | Hearth's own: full US layout (5 rows), wide keys, glass, docked at the bottom, reserves its space | Windows' touch keyboard, or osk.exe | **done**: `mantel.html?part=keyboard` in its own no-focus window, docked above the taskbar as an app bar (full Mantel: by hand + work area), same size rule (≤ 40 % height, ≥ 260 or 22 % width) |
| Typing | uinput; Shift/Ctrl/Alt one-shot, Caps | — | **done**: `KeyTyper` (SendInput: Unicode for characters, keys for Ctrl/Alt shortcuts and specials) into the window you were in |
| Controller | D-pad/left stick pick, A type, X ⌫, LB shift, ☰ Enter, B/Y close; RT still clicks | — | **done** (`ControllerBridge.KeyboardOpen` / `KeyboardButton`) |
| Opening | Y, taskbar button, left-edge swipe, Quick Access, Hearth menu, Settings › Controller | Y / buttons opened Windows' keyboard | **done** (except the edge swipe, below). Outside Mantel, Y still opens Windows' keyboard |
| Lock-screen keyboard | Its own keyboard on the lock screen | — | **differs**: Windows' lock screen has its own touch keyboard |

## Lock screen (Lock.qml)

| Item | Hearth OS | Windows before | Now |
|---|---|---|---|
| Lock screen | Mantel's own (clock, password, its own keyboard, PAM) | Windows' lock screen (`LockWorkStation`) | **differs, on purpose**: Windows' lock screen is the secure one (Windows Hello, PIN, secure desktop); an app can't replace it and a look-alike would be weaker |
| Lock when waking | Settings › Sign-in switch | — | **done**: the same switch sets Windows' "require sign-in on wake" (powercfg CONSOLELOCK) |

## Notifications (Notifications.qml)

| Item | Hearth OS | Windows before | Now |
|---|---|---|---|
| Popups | Mantel's own cards, top right | Windows' own toasts | **differs**: reading other apps' notifications needs `UserNotificationListener`, which only packaged (MSIX) apps may use; Ember is a plain exe. Windows' toasts show (with Explorer). What does reach Ember (its own messages; in Full Mantel the tray icons' balloon notifications, which Ember receives as the tray host) pops up as Hearth's glass pill at the top, not as cards |

## The clock's panel (ClockPanel.qml)

| Item | Hearth OS | Windows before | Now |
|---|---|---|---|
| Panel | 380 wide above the clock (beside a side bar), radius 20, glass, slides up; click outside closes | — | **same** (`part=clock`, sized to what's in it) |
| Calendar | Today's date in accent, month and year, ‹ ›, 7 columns from the locale's first weekday, today in an accent circle; scroll / Page Up/Down change the month | — | **same** |
| Notification history | This session's notifications (newest first, 100), × dismisses, Clear all; "No notifications" | — | **differs**: the history holds what reaches Ember (see Notifications: its own messages, and tray balloons in Full Mantel), for the same reason |
| Do Not Disturb | Switch; urgent ones still pop up | — | **differs**: it stops Mantel's own popups (the history still fills). Windows' own Do Not Disturb (Focus) has no API for apps to switch it, so it isn't changed; Windows Settings › Notifications has it |
| Keys / controller | Up/Down, Enter (A), Left/Right months, Esc (B) | — | **same** |

## Volume bar (VolumeOsd.qml)

| Item | Hearth OS | Windows before | Now |
|---|---|---|---|
| Bar | Glass pill at the top, 360×64, ember→flame, 1.5 s | Full Mantel: the same pill (native `VolumeOsd`) | **same** in Full Mantel; on top of Explorer, Windows shows its own volume flyout (Explorer handles the keys) |
| Over 100 % | Over-amplification to 150 % | — | **differs**: Windows has no system-wide over-amplification |

## Normal-desktop round (October 2026)

| Item | Hearth OS | Windows (Ember) |
|---|---|---|
| Show desktop | Strip at the end of the taskbar, Super+D / Super+M (the shell minimizes everything, then puts it back) | Same strip; Windows+D over Explorer, Ember's own in full Mantel |
| Desktop icons | Double-click opens, Ctrl/Shift+click, selection box, Cut/Copy/Paste (text/uri-list, so file managers share it), drag out to apps and in from them, Open with, New text file and Templates, Sort by, Keep icons lined up, Shift+Delete asks | Same except dragging files out of / into the desktop (WebView2 can't start a Windows file drag) and Keep icons lined up; the clipboard is Windows' (File Explorer pastes it) |
| Windows-key shortcuts | Super+I/A/N/S/R/1-9/Tab, Ctrl+Alt+T, Print / Shift+Print / Super+Shift+S screenshots (clipboard + notification) | Windows handles them over Explorer; full Mantel handles D, M, E, I, A, N, S, R, Shift+S (Snipping Tool) and 1-9 |
| Notifications | Mute an app from its pop-up, Settings › Notifications | not yet |
| Mouse & touchpad, screen off / sleep timers, several screens | Hearth Settings | Windows Settings |
| Autostart apps | XDG autostart entries that are on (not Steam) | Windows' own |

## Desktop and wallpaper (Wallpaper.qml)

| Item | Hearth OS | Windows before | Now |
|---|---|---|---|
| Moving wallpapers | Fireside (default), Lava, Aurora | Same animations (Full Mantel) | **same**; on top of Explorer they can't move (Windows draws that desktop) |
| Pictures | 13 Hearth pictures (A–Z), your Pictures › Wallpapers, any file | 8 Hearth pictures | **done**: all 13 (`ui/wallpapers`, cat/kiln/mark/ridge/fireside added) with the same names; "Choose a picture…" copies any picture into `%LOCALAPPDATA%\Nimbus\wallpapers` (served as `https://hearth-walls.local/`) |
| Reduce motion | Still wallpaper | — | **done** |
| "Set as Hearth wallpaper" in Files | Right-click a picture | — | **differs**: not in `ui/mantel-files.html` yet (left alone as asked) |
| Your Windows wallpaper | — | "" = Windows' own | Windows-only tile, kept |
| Desktop icons (Desktop.qml) | ~/Desktop: 104×112 grid, folders first then by name, places remembered (desk.json); click selects, click again / double-click opens, drag to move (swaps with an icon already there), right-click / hold: Open, Show in Files, Rename, Move to Trash, Properties; arrows, Enter, F2, Delete, Menu key, Ctrl+Shift+N | Full Mantel: none | **done** in Full Mantel: your Desktop folder plus the Public desktop, shortcuts (.lnk, .url) with their target's icon and pictures as thumbnails (Windows' shell), places in mantel.json, the same clicks, keys and menu ("Move to the Recycle Bin"); Properties is Hearth's card. Over Explorer the layer is off: Windows draws its own desktop |
| Desktop's right-click menu | New folder, Change wallpaper, Display settings, Open Files, Line up icons by name, Show desktop icons ✓ | — | **done** (Full Mantel; over Explorer, right-clicking the desktop is Windows') |

## Hearth Settings (SettingsApp.qml + Set*.qml)

| Item | Hearth OS | Windows before | Now |
|---|---|---|---|
| Window and page list | 1100×720, 270 list (220 narrow), mark + "Settings", "Ⓐ open  Ⓑ back" | Same | **same** |
| Pages and order | Wi-Fi, Bluetooth, Sound, Display, Appearance, Power, Controller, Keyboard, Printers, Date & time, Default apps, Clipboard, Users, Sign-in, Web browser, Updates, About (+ Install Hearth on the live USB) | No Keyboard, Printers, Date & time, Default apps, Clipboard or Users | **done** (Install Hearth is live-USB only) |
| Page layout | Left-aligned at x 24, up to 820, title 26 | Centered | **done** |
| Rows | SetRow (chevron on rows that do something), SetSwitch, SetSlider 76, SetChoice, SetField 52, SetHeader | Similar, no chevrons, 44 px fields, every row focusable | **done**: same controls, chevrons, sizes; rows that do nothing aren't in the focus order; Enter on a choice takes the next option |
| Keys / controller | Up/Down pick a page, Right/A in, Up/Down controls, Left/B back, B on the list closes | Same, plus Left/Right moved across wallpaper tiles | **done**: tiles move with Up/Down like on Hearth OS |
| Results | Shown as a muted line under the first row | Toasts | **done** (toasts only for unexpected errors) |
| **Wi-Fi** | Switch ("Connected to …"), "Networks · looking…", rows (Connected/Connecting…/Saved/Secured/Open, bars + 🔒), password field + Connect, select connected = disconnect, "Look again", "Turn on Wi-Fi to see networks.", refresh 15 s | Different wording, Disconnect/Forget buttons | **done**. Forget isn't on Hearth OS's page, so it's gone here too (Windows Settings has it) |
| **Bluetooth** | Switch (connected names), Your devices (select = connect/disconnect), New devices + Look for devices, Manage › Forget, refresh 8 s | "Add a device", Remove as the value | **done**, except: selecting a paired device opens Windows' Bluetooth settings — **differs**: Windows has no API to connect or disconnect a paired device |
| **Sound** | Volume, Mute, Over-amplification, Play sound through, Microphone (input volume, devices) | Volume, Mute, Output | **done** (microphone added: `audio.input`); Over-amplification hidden (not possible on Windows) |
| **Display** | Keep-or-go-back banner (15 s), Brightness, Scale, Refresh rate, Resolution rows (native first, "In use") | Resolution as a choice, no countdown | **done**. Scale offers what Windows allows for the screen |
| **Appearance** | Wallpaper grid (3 columns, 130 tall, name strip, ✓), Choose a picture…, Comfort: Night light, Reduce motion | Desktop style, 4-column grid, accent | **done**; Desktop style and accent moved under a "Windows" heading at the end |
| **Power** | Battery (…left / charging / full), Power mode, TDP slider, Power button Press/Hold, Turn off or switch | Battery, Power mode, link to Windows | **done** (battery time from Windows; Press = Windows' power button action). **Differs**: no TDP on Windows (no standard API; vendors' tools do it), no "Hold" (hardware) and no "Power menu" choice (Windows doesn't hand the button to apps) |
| **Controller** | Controller mouse switch, Pointer speed, Button mapping (InputPlumber), Help: Controller buttons, On-screen keyboard | Mouse mode choice, Game Bar switch, text list | **done**: same rows; pointer speed works; Button mapping shows Hearth OS's own "works on Hearth, the OS" line (no InputPlumber on Windows). Mouse mode and the Game Bar switch are under "Windows" |
| **Sign-in** | Start in, Sign in automatically, Ask for my password after sleep, Password, Device name | — | **done**: Start in (Ember's start mode), Ask for password (powercfg). Password and Rename open Windows Settings — **differs**: changing them needs Windows' own UI/admin; automatic sign-in is hidden (Windows keeps that behind netplwiz/admin) |
| **Web browser** | Aloha Browser: status, Install (downloads it), Make default, Look for an update, Remove | — | **done** with Windows' limits: Install opens alohabrowser.com (its installer), Make default opens Default apps (Windows makes you pick), Remove opens Apps; no "Look for an update" (Aloha updates itself on Windows) |
| **Updates** | "Hearth x", staged banner, Restart now, progress, Download, Look for updates, Update anyway | Different rows | **done** with Ember's updater ("Ember x", ready banner, progress, Download, Look for updates); "Check automatically" under "Windows"; no "Update anyway" |
| **Date & time** | Now (time · date, zone, automatic or by hand), 12/24-hour clock (the taskbar follows), Set the time automatically, Set the clock (date + time), time zone search or region › city | — | **done**: the same rows. Time zones are Windows' (grouped by region from their IANA names; cities are Windows' names like "Pacific Time (US & Canada)"). The time zone changes without administrator rights (tzutil, like Windows Settings); automatic time and setting the clock ask for Windows' administrator OK through Ember's elevated helper (`--admin ntp` / `time-set`). No "restart the desktop" row: the taskbar clock follows at once |
| **Keyboard** | Layouts (first one starts, Move up/down, Remove, up to 4), Add a layout (search), Switch layouts with (None, Alt+Shift, Ctrl+Shift, Alt+Space, Caps Lock), key repeat delay and speed, Try typing here, Keyboard shortcuts | — | **done**: layouts are Windows' language list (Set-WinUserLanguageList, the supported way), the catalogue from Windows' keyboard layouts. **Differs**: Windows only offers None / Alt+Shift / Ctrl+Shift for switching (Windows+Space always works); repeat delay has Windows' 4 steps (250–1000 ms); the shortcut list is Windows' and Mantel's |
| **Default apps** | Each kind (browser, files, text, pictures, video, music, PDF & e-books, documents, spreadsheets, presentations, terminal, email once an app can, calendar, contacts, maps, archives), Hearth's own app for it and the installed apps to choose from; Reset to Hearth's defaults (the same page is in Game Mode's Settings › System) | — | **differs**: Windows 11 doesn't let apps set defaults; the page shows each kind's current app and opens Settings › Default apps |
| **Users** | You (picture, name, kind), change picture, change password, every account (remove: keep or delete files), add an account (standard or administrator) | — | **done** with Windows' limits: the accounts list and removing (keep or delete the files) are Ember's, removal through the elevated helper (`--admin user-remove`, net user + the profile); picture, password and adding an account open Windows Settings (Windows wants its own flows for Microsoft accounts and passwords) |
| **Printers** | Your printers (state, kind), test page, make default, remove; find printers and add, or by address | — | **done**: printers, test page, default and remove through Windows' printer management (WMI); "Add a printer or scanner" opens Windows' page, which finds network and USB printers (no "by address" row) |
| **Clipboard** | Keep clipboard history, Open clipboard history, Clear… | — | **done**: Windows' own clipboard history switch; clearing keeps pinned items (Windows does) |
| **About** | Mark + "Hearth" + version, facts: Device, Name, Signed in as, Processor (threads), Graphics, Memory, Storage (free of), System, Version, Kernel, Desktop | Ember row + facts | **done**, same facts in the same order ("Ember", "Mantel · Ember x") |

## Controller in Desktop Mode (hearth-pad / Pad.qml)

| Item | Hearth OS | Windows before | Now |
|---|---|---|---|
| Pointer, scroll, A/RT click, B Esc, D-pad arrows, LB/RB switch windows, Menu apps, Guide Hearth menu, View tap controls, View + D-pad snap, View + B close | hearth-pad | ControllerBridge | **same** |
| X right-click | Yes | LT only | **done** |
| L3 middle-click, R3 precise pointer (toast) | Yes | — | **done** |
| Pointer speed | Settings › Controller | — | **done** (`PointerSpeed`, saved in library.json) |
| Y | Hearth's keyboard | Windows' keyboard | **done** |
| Keys in Hearth's own windows (menus, Settings, Task Manager, Files) | A Enter, B Esc, X Menu key | A Enter, B Esc, X/Y F23/F24 for Task Manager/Files | **same** in effect |
| Pause while a game is fullscreen | Yes | Games get the controller (by game detection) | **same** in effect |

## Other

| Item | Hearth OS | Windows before | Now |
|---|---|---|---|
| Edge swipes (EdgeSwipes.qml): right = Quick Access, left = keyboard, bottom = launcher | Yes | — | **differs**: Windows 11 keeps edge swipes for itself (widgets, notifications); a shell can't take them without turning them off by policy |
| Key bindings: Super, Super+Space, Ctrl+Shift+Esc, Super+L, volume keys | labwc | Full Mantel: Windows key (launcher), Ctrl+Shift+Esc (Task Manager), volume keys | **same** for those; Super+L is Windows' lock; Super+Q/E/G/Return aren't taken (Windows uses some of them) |
| Live USB welcome (LiveWelcome.qml), Install Hearth page | Live USB only | — | not applicable on Windows |
| Night light while Mantel isn't running | Only in Desktop Mode | — | same: Mantel's night light runs only with Mantel |
| Task Manager, Files (TaskManager.qml, FileManager.qml) | | `ui/mantel-tasks.html`, `ui/mantel-files.html` | written to match earlier; not changed here |
| Snapping (labwc: halves, quarters, regions; Super+arrows; View + D-pad) | labwc | Windows' own over Explorer | **same** in effect over Explorer (Windows' snap, Snap Layouts, Win+arrows; View + D-pad types Win+arrows). Full Mantel: Explorer, which handles Win+arrows, isn't there, so Ember does Win+Left/Right (halves) and Up/Down (maximize, restore/minimize); no quarters or Snap Layouts there |
| Alt+Tab | Hearth's switcher in Hearth's colours | Windows' | **differs**: Windows' own switcher (Ember doesn't block it; LB/RB on the controller drive it) |
| Title-bar menu | Hearth's (labwc menu.xml) | Windows' | **differs**: Windows draws every app's title bar and its menu (Alt+Space) |

## Clipboard history (Clipboard.qml)

| Item | Hearth OS | Windows before | Now |
|---|---|---|---|
| Picker | Super+V: 640×620 box, text (2 lines) and pictures (with size), × forgets, Clear all (press twice); Ⓐ paste Ⓧ forget Ⓑ close | — | **done**: `part=clip`, the same box, rows, keys and words, reading Windows' own history (Windows.ApplicationModel.DataTransfer.Clipboard, open to desktop apps); pasting makes the item the clipboard's content and types Ctrl+V into the window you were in |
| Opening it | Super+V, Settings › Clipboard | — | **differs**: Windows+V stays Windows' own picker; Hearth's opens from the launcher ("Clipboard history"), Settings › Clipboard, and with the controller like any Mantel panel (D-pad, A, X, B) |
| Keeping the history | cliphist, skipping copies marked secret | — | **differs**: Windows keeps it (its switch is Settings › Clipboard); Windows skips what password managers mark as secret too |

## Apps (AppStore.qml)

| Item | Hearth OS | Windows before | Now |
|---|---|---|---|
| Window and pages | "Apps", 1100×720, page list: Discover, Search, Installed, Updates (n), Games, Music & video, Graphics & photos, Internet, Office, Programming, Learning, Science, System, Tools | — | **same** (`ui/mantel-apps.html`, Hearth's settings look, same keys and controller) |
| Where apps come from | Flathub (flatpak) | — | **differs**: winget, Windows' package manager (App Installer); search, show, install, upgrade (one or all), list and uninstall. winget has no machine-readable output for these, so its tables are read by column; a shortened id is looked up again on its own |
| Discover | Flathub's popular apps | — | **differs**: winget has no popularity feed, so Discover is a fixed list of well-known apps; categories search winget's tags |
| App page | Icon, name, developer, summary, Install / Open / Update / Remove…, screenshots, About, Details, Website | — | **done**, except: winget's catalogue has no icons or screenshots (installed apps show their Start menu icon, others their letter); Remove has one choice (each app's own uninstaller decides about its settings) instead of keep / delete data; Open starts the Start menu entry with the same name |
| Progress | Download and install progress | — | **done** from winget's progress lines (MB downloaded, %) |

## Windows-only features kept

The `^` tray peek on top of Explorer; the Public desktop's icons in Full Mantel; "Your Windows wallpaper"; Settings › Appearance › Windows (Desktop style, Accent color); Settings › Power › Windows (More power
settings); Settings › Controller › Windows (controller mouse outside Desktop Mode, Xbox button / Game Bar);
Settings › Updates › Windows (Check automatically); View + Menu toggles the controller mouse.
