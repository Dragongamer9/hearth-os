# Desktop layouts and Describe it (approved Nov 22, 2026)

The owner, Nov 22: "I want to add the ability to create any desktop layout and look … KDE look or a Gnome look",
"type what you want while editing your layout and it makes what you want", "Ok it can use ai", "add that Describe it
to all our apps as well", "it should be able to add features as well … features that are new should be sent to us",
"Same for Mantel", then "I like it let's do it", and "This go into both mantel versions": built in both Mantels, the QML
Mantel (hosts/linux/desktop/shell) and Mantel in Rust (hosts/mantel-rs), and in hearth-ui's apps. One shared core for
all of them (the layout files, the blocks' names, the Describe it helper that turns words, sketches and the AI's answer
into a layout or a feature recipe and checks it, sending ideas); each Mantel only draws it. A part is ticked when it's
in both Mantels.

Mockups: desktop-layouts/layouts-1-plasma-style.png, desktop-layouts/layouts-2-gnome-style.png, desktop-layouts/layouts-3-gnome-style-overview.png, desktop-layouts/layouts-4-windows-11-style.png, desktop-layouts/layouts-5-macos-style.png, desktop-layouts/layouts-6-layout-editor.png, desktop-layouts/layouts-7-describe-it.png, desktop-layouts/layouts-8-describe-it-new-feature.png, desktop-layouts/layouts-9-describe-it-send-idea.png, desktop-layouts/layouts-10-ideas-inbox.png, desktop-layouts/layouts-11-describe-it-in-files.png
Release: next

## 1. Plasma-style
- [ ] One full-width bottom bar (48 px, the look's glass, a line on top)
- [ ] The flame (app menu) at the start, lit while its menu is open, then a separator
- [ ] App icons only: pinned and open, the one in front with a long accent line, open ones a short line
- [ ] Tray arrow, Wi-Fi, volume, battery with %, notifications bell
- [ ] Clock on two lines (time, "Sun 22 Nov") at the end
- [ ] Game Mode button, then the Show desktop strip
- [ ] Start-style menu from the bottom left: search ring, Favourites / All apps / Games / Settings / Places, the grid of favourites with the current one ringed
- [ ] The menu's foot: you (name and picture), Game Mode, Settings, Power
- [ ] Windows with minimise, maximise and close on the right

## 2. GNOME-style
- [ ] Thin top bar (32 px, nearly black)
- [ ] Desktops pill at the start (the one you're on long), opens the overview
- [ ] Date and time in the middle ("Sun 22 Nov  1:52 PM")
- [ ] Game Mode pill and the status pill (Wi-Fi, volume, battery, power) at the end
- [ ] Windows with only a close button, the title in the middle

## 3. GNOME-style overview
- [ ] "Type to search" at the top
- [ ] The desktop shrunk, its windows side by side with their names under them
- [ ] The next and the previous desktops at the sides
- [ ] The dash: app icons, a line, the app grid button

## 4. Windows 11-style
- [ ] Centred taskbar: the flame, a Search box, Task view, app icons with their lines
- [ ] Game Mode at the start of the taskbar
- [ ] Tray arrow, Wi-Fi / volume / battery, time and date (11/22/2026), bell at the end
- [ ] Centred Start menu: search, Pinned with All apps, Recommended (files, notes, apps, Settings pages) with More
- [ ] Start menu foot: you, Game Mode, power

## 5. macOS-style
- [ ] Top menu bar: the flame, the app in front by name, its menus (File, Edit, View, Go, Window, Help)
- [ ] Menu bar end: Game Mode, battery, Wi-Fi, search, Quick Access, date and time
- [ ] Floating dock: the flame, apps with a dot when open, a line, Downloads, Trash
- [ ] Window buttons (close, minimise, maximise) on the left, the title in the middle

## 6. The layout editor (Settings › Appearance › Desktop layout)
- [ ] Desktop layout under Appearance (Looks, Desktop layout, Wallpaper, Bar and dock)
- [ ] Heading and line: "Where the bars, the dock, the app menu and the window buttons go. Your look's colours stay as they are."
- [ ] Share…, Open a layout file…, Back to (the layout), Save as my own…
- [ ] Ready-made layouts with pictures: Hearth, Steam-style, Plasma-style, GNOME-style, Windows 11-style, macOS-style, Start from empty; the one in use marked "in use"
- [ ] The screen with its panels: Add a panel at the top / on the left (and the other edges), the selected panel lit
- [ ] Blocks in a panel to drag along it, a lit line where a dragged block lands, a Space block
- [ ] Blocks: Search, Desktops, Window title, App menus (File, Edit…), Notifications, Quick Access, Show desktop, Keyboard, Widgets, Space (and the bar's others)
- [ ] This panel: Edge (Top, Bottom, Left, Right), Style (Full width, Floating, Centred), Height, Hide until the pointer touches the edge
- [ ] The whole desktop: App menu style, Dock (None, Floating, Hide), Window buttons (Left / Right, Close / All three)
- [ ] "Game Mode always stays in a panel, in every layout."

## 7. Describe it
- [ ] Editing your desktop: click any part to change it (a bar, the dock, the clock's calendar, Quick Access, the app menu), Done
- [ ] NOW and NEW side by side ("Try it, nothing is saved yet")
- [ ] What it changed, as a list
- [ ] "Made by your AI": Hearth AI on the Nvidia PC by default, or any AI picked in Settings
- [ ] "It only rearranges Mantel's own parts, so it can't break your desktop. Undo is always here."
- [ ] Keep this, Try again, Undo
- [ ] The box to type in, with an added sketch (removable), Add a sketch or a picture, Draw it here, examples to try, Make it

## 8. Describe it adds a feature (Mantel)
- [ ] The new item in its place, marked NEW and lit
- [ ] The feature as its parts: SHOWS, CLICK, RIGHT-CLICK, LOOKS, with where each comes from
- [ ] "Safe: only parts Hearth already has"
- [ ] How often it updates (only while it shows), and Settings › Your features to turn it off
- [ ] What you asked; Keep this, Change it…, Try again, Undo
- [ ] Share it: Send this feature to Hearth…

## 9. This needs something new
- [ ] "Needs something new" and the heading
- [ ] You asked, Why, The closest it can make now (with Add this instead)
- [ ] Exactly what goes, word for word: the part, the words asked, Hearth OS and the app's versions, the sketch or none, "No name, no files, no account, nothing else."
- [ ] Tell me if it's added (a notice in the Hearth Store)
- [ ] Not now, Send this idea to Hearth (nothing is sent without it)

## 10. Ideas inbox (Hearth Ideas)
- [ ] New, Later, Accepted, Said no, with counts; By app
- [ ] One row per idea with how many asked; the same idea from several people counted once
- [ ] Built by people already (ready to add as it is) / needs something new / has a sketch
- [ ] See it, Accept, Later, No; sort by Most asked
- [ ] "Accepting doesn't change anything for anyone by itself": the roadmap, then an update with the owner's number
- [ ] Ideas come through the Send logs service into a private list on GitHub

## 11. Describe it in Hearth Files
- [ ] EDITING in the title, the parts being changed outlined
- [ ] Sidebar icons only; the preview on the right and bigger; new buttons in Downloads marked NEW
- [ ] What it'll do as chips (layout ones and the new feature)
- [ ] Undo, Try again, Keep this
- [ ] "what its buttons do stays the same", Back to the original: View › Layout

## 12. Every Hearth app (the owner, Nov 22: "i want EVERY Hearth app i make to be able to be included in the modular stuff")
Each one gets Describe it as Hearth Files has it (part 11): its own parts rearranged, new features from Hearth's own
parts (hearth-layout feature.rs), what needs something new sent as an idea. One shared core, so a new app gets it by
taking the core, and a Hearth app isn't done without it. Ticked per app when it's in that app.
- [ ] Mantel (QML) and Mantel in Rust: the bars, the dock, the app menu, Quick Access, the calendar (parts 1-8)
- [ ] Hearth Settings (Mantel's and the Rust one)
- [ ] Hearth Files
- [ ] Hearth Store
- [ ] Task Manager, Resource Monitor, System Restore, Reliability history
- [ ] Capture, Hearth Notes, Hearth Family, Hearth Server, Logitech G15, the controller (hearth-pad-host)
- [ ] Hearth Ideas
- [ ] Hearth AI Command Center and Hearth AI (Ithil) (their own repos, the same core through hearth-layout)
- [ ] Beacon, Capture Card, Console Remote Play, Hearth TV, Hearth Buddy, Hearth OS CAD, Trains 2010 (their own repos)
- [ ] Every new Hearth app from now on
