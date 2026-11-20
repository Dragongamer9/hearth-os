# Every KDE System Settings page in Hearth Settings (1.16)

Release: 1.16
Mockups: next-update/settings-A2-display-mouse.png, next-update/settings-B2-display-controller.png

The owner (Nov 19, 2026): "Build them all into Hearth Settings": every KDE System Settings module (KCM) gets a real page
in Hearth Settings (the Rust Settings app, hosts/hearth-ui/apps/settings), in Hearth's look (the approved Settings
mockups above, both looks), so KDE's own System Settings is never needed on the Fedora edition.

**The list** is every module in Fedora 45's Plasma 6.8.3 (kcmshell6 --list in the Fedora image, CI job
kde-kcm-inventory.yml, Nov 19, 2026; KDE's website can't be read from the owner's sessions), plus the modules that are a
part of another (window frame themes, KRunner, Trash). 108 modules: 64 already had a Hearth page, 12 have a new page
(hosts/linux/desktop/shell/kde-settings.json, drawn by cats/kde.rs, saved by hearth-kde-settings), 32 are in the next
batches. A new page is ticked once its CI pictures were opened and checked.

**How the new pages save**: only the keys a page lists, the way the KCM does (the same file, group and key as the KCM's
kcfg in Plasma 6.8.3), through hearth_kconfig (atomic, .bak kept, every other line left alone, a file that can't be read
never written over, [$i] locks kept), then KDE's "config changed" signal and the program that reads it asked to read it
again. Where the KCM applies a choice with KDE's own tool (plasma-apply-colorscheme, -desktoptheme, -lookandfeel,
KDE's GTK settings service, logind for the firmware setup), the page runs the same tool. Opening a page writes nothing
(CI checks). On labwc the pages still read and save KDE's files and say which settings only KDE Plasma uses.

Not part of System Settings (each app's own settings window; nothing to build here): Dolphin's own pages
(kcm_dolphingeneral, kcm_dolphinviewmodes; Hearth Files has its own), KRunner plugins' own small settings windows
(kcm_krunner_charrunner, _dictionary, _kill, _spellcheck: on the KRunner page in batch 2), KWin effects' own settings
windows (kcm_kwin4_genericscripted and each effect's: Window effects).

## Every module
- [ ] systemsettings (KDE's System Settings app itself) → Hearth Settings › All KDE settings (new, batch 1: built, waiting for its CI pictures)
- [x] kcm_about-distro (About this System: Information About This System) → Hearth Settings › About (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"about\""
- [x] kcm_access (Accessibility: Configure Accessibility Options) → Hearth Settings › Accessibility (already there) — hosts/hearth-ui/apps/settings/src/cats/a11y.rs: "\"accessibility\""
- [ ] kcm_activities (Activities: Activities help you to focus on a specific task) → new page kde-activities (batch 2): Activities: make, rename, switch, forget their history
- [x] kcm_animations (Animations: Configure animation speed and style) → Hearth Settings › Window effects (already there) — hosts/hearth-ui/apps/settings/src/cats/pers.rs: "\"kwin-effects\""
- [x] kcm_app-permissions (Application Permissions: Configure Application Permissions) → Hearth Settings › App permissions (already there) — hosts/hearth-ui/apps/settings/src/cats/apps.rs: "\"app-permissions\""
- [ ] kcm_audio_information (Audio Device Information: Audio Device Information) → new page kde-info-audio (batch 3): Info Center: sound devices
- [ ] kcm_audiocd (Audio CDs: Audiocd IO Worker Configuration) → new page kde-audiocd (batch 3): Audio CDs: rip formats and quality
- [ ] kcm_auroraedecoration (Aurorae window frames) → Hearth Settings › Title bars (new, batch 1: built, waiting for its CI pictures)
- [x] kcm_autostart (Autostart: Automatically Started Applications) → Hearth Settings › Autostart (already there) — hosts/hearth-ui/apps/settings/src/cats/apps.rs: "\"autostart\""
- [x] kcm_baloofile (File Search: Configure File Search) → Hearth Settings › Search (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"search\""
- [ ] kcm_block_devices (Block Devices: Block Devices) → new page kde-info-block (batch 3): Info Center: drives and partitions
- [x] kcm_bluetooth (Bluetooth: Manage Bluetooth devices) → Hearth Settings › Bluetooth (already there) — hosts/hearth-ui/apps/settings/src/cats/bt.rs: "\"bluetooth\""
- [x] kcm_bolt (Thunderbolt: Thunderbolt Device Management) → Hearth Settings › Thunderbolt (already there) — hosts/hearth-ui/apps/settings/src/cats/bt.rs: "\"thunderbolt\""
- [ ] kcm_breezedecoration (Breeze Window Decoration) → Hearth Settings › Title bars (new, batch 1: built, waiting for its CI pictures)
- [ ] kcm_cddb (CDDB Retrieval: Configure the CDDB Retrieval) → new page kde-cddb (batch 3): CDDB: looking up CDs' track names
- [x] kcm_cellular_network (Cellular Network: Management of cellular networks) → Hearth Settings › Connection details (already there) — hosts/hearth-ui/apps/settings/src/cats/net.rs: "\"net-details\""
- [x] kcm_clock (Date & Time: Date and Time) → Hearth Settings › Date & time (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"datetime\""
- [ ] kcm_colord (Color Management: Manage color profile of devices) → new page kde-colord (batch 3): Colour management: each screen's and printer's colour profile
- [ ] kcm_colors (Colors: Choose color scheme) → Hearth Settings › Colours (new, batch 1: built, waiting for its CI pictures)
- [x] kcm_componentchooser (Default Applications: Choose default Applications) → Hearth Settings › Default apps (already there) — hosts/hearth-ui/apps/settings/src/cats/apps.rs: "\"defaults\""
- [x] kcm_cpu (Advanced CPU Information: Advanced CPU Information) → Hearth Settings › Info Center (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"info-cpu\""
- [x] kcm_cursortheme (Pointers: Choose mouse pointer theme) → Hearth Settings › Fonts, cursor & icons (already there) — hosts/hearth-ui/apps/settings/src/cats/pers.rs: "\"fonts\""
- [ ] kcm_desktoppaths (Locations: Location for Personal Files) → new page kde-locations (batch 2): Where Desktop, Documents, Downloads, Music, Pictures and Videos are
- [ ] kcm_desktoptheme (Plasma Style: Choose Plasma style) → Hearth Settings › Plasma style (new, batch 1: built, waiting for its CI pictures)
- [x] kcm_device_automounter (Configure automatic mounting of disks and volumes: Configure automatic mounting of disks and volumes) → Hearth Settings › USB devices & drives (already there) — hosts/hearth-ui/apps/settings/src/cats/bt.rs: "\"usb-devices\""
- [x] kcm_disks (SMART Status: Storage device health monitoring) → Hearth Settings › Drives & undo (already there) — hosts/hearth-ui/apps/settings/src/cats/hs.rs: "\"hs-drives\""
- [ ] kcm_edid (Extended Display Identification Data: Extended Display Identification Data) → new page kde-info-edid (batch 3): Info Center: what each screen tells about itself (EDID)
- [x] kcm_egl (EGL information: EGL information) → Hearth Settings › Info Center · Vulkan & OpenGL (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"info-graphics\""
- [x] kcm_energyinfo (Energy Consumption Statistics: Energy Consumption Statistics) → Hearth Settings › Info Center · Energy (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"info-energy\""
- [ ] kcm_feedback (User Feedback: Configure user feedback settings) → Hearth Settings › KDE user feedback (new, batch 1: built, waiting for its CI pictures)
- [x] kcm_filetypes (File Associations: Configure file associations) → Hearth Settings › File types (already there) — hosts/hearth-ui/apps/settings/src/cats/apps.rs: "\"file-types\""
- [x] kcm_firmware_security (Firmware Security Information: Firmware Security Information) → Hearth Settings › Firmware & device security (already there) — hosts/hearth-ui/apps/settings/src/cats/privacy.rs: "\"firmware\""
- [ ] kcm_fontinst (Font Management: Install, manage and preview fonts) → new page kde-fontinst (batch 3): Font management: install, enable, disable, remove fonts
- [x] kcm_fonts (Fonts: Configure user interface fonts) → Hearth Settings › Fonts, cursor & icons (already there) — hosts/hearth-ui/apps/settings/src/cats/pers.rs: "\"fonts\""
- [ ] kcm_gamecontroller (Game Controller: Test and Configure Game Controllers) → new page kde-gamecontroller (batch 3): Game controller: test buttons and sticks, KDE's controller emulation
- [x] kcm_glx (GLX information: GLX information) → Hearth Settings › Info Center · Vulkan & OpenGL (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"info-graphics\""
- [x] kcm_icons (Icons: Choose icon theme) → Hearth Settings › Fonts, cursor & icons (already there) — hosts/hearth-ui/apps/settings/src/cats/pers.rs: "\"fonts\""
- [x] kcm_interrupts (Interrupt information: Interrupt information) → Hearth Settings › Info Center · Interrupts (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"info-interrupts\""
- [x] kcm_kaccounts (Online Accounts: Configure your internet accounts such as Google, Live, Owncloud) → Hearth Settings › Online accounts (already there) — hosts/hearth-ui/apps/settings/src/cats/accounts.rs: "\"online-accounts\""
- [ ] kcm_kded (Background Services: Configure background services) → new page kde-kded (batch 2): KDE's background services: start, stop, start at sign-in
- [x] kcm_keyboard (Keyboard: Keyboard Hardware and Layout) → Hearth Settings › Keyboard (already there) — hosts/hearth-ui/apps/settings/src/cats/bt.rs: "\"keyboard\""
- [ ] kcm_keys (Shortcuts: Configure Keyboard Shortcuts) → new page kde-shortcuts (batch 2): KDE's keyboard shortcuts: every app's and Plasma's
- [ ] kcm_krdpserver (Remote Desktop: Configure Remote Desktop) → new page kde-krdp (batch 3): Remote Desktop (RDP, KRDP): on/off, user and password, quality
- [ ] kcm_krunnersettings (KRunner Settings) → new page kde-krunner (batch 2): KRunner: where it opens, history, its plugins' own settings
- [x] kcm_kscreen (Display Configuration: Manage and configure monitors and displays) → Hearth Settings › Display (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"display\""
- [x] kcm_kwallet5 (KDE Wallet Configuration: KDE Wallet Configuration) → Hearth Settings › Passwords (already there) — hosts/hearth-ui/apps/settings/src/cats/privacy.rs: "\"passwords\""
- [x] kcm_kwin_effects (Desktop Effects: Configure compositor settings for desktop effects) → Hearth Settings › Window effects (already there) — hosts/hearth-ui/apps/settings/src/cats/pers.rs: "\"kwin-effects\""
- [ ] kcm_kwin_scripts (KWin Scripts: Manage KWin scripts) → new page kde-kwin-scripts (batch 2): KWin scripts: on or off, install, remove
- [x] kcm_kwin_virtualdesktops (Virtual Desktops: Configure navigation, number and layout of virtual desktops) → Hearth Settings › Virtual desktops (already there) — hosts/hearth-ui/apps/settings/src/cats/pers.rs: "\"kwin-desktops\""
- [ ] kcm_kwindecoration (Window Decorations: Configure window titlebars and borders) → Hearth Settings › Title bars (new, batch 1: built, waiting for its CI pictures)
- [x] kcm_kwinoptions (Window Behavior: Configure window actions and behavior) → Hearth Settings › Window behaviour (already there) — hosts/hearth-ui/apps/settings/src/cats/pers.rs: "\"kwin-behaviour\""
- [x] kcm_kwinrules (Window Rules: Individual Window Behavior) → Hearth Settings › Window rules (already there) — hosts/hearth-ui/apps/settings/src/cats/apps.rs: "\"kwin-rules\""
- [x] kcm_kwinscreenedges (Configure active screen corners and edges: Configure active screen corners and edges) → Hearth Settings › Screen edges (already there) — hosts/hearth-ui/apps/settings/src/cats/pers.rs: "\"kwin-edges\""
- [ ] kcm_kwinsupportinfo (KWin Support information: KWin Support information) → new page kde-info-kwin (batch 3): Info Center: KWin's support information
- [ ] kcm_kwintabbox (Task Switcher: Navigation Through Windows) → new page kde-tabbox (batch 2): The task switcher's other settings (which windows it lists, their order); its look is in Window effects
- [x] kcm_kwintouchscreen (Configure touch screen swipe gestures: Configure touch screen swipe gestures) → Hearth Settings › Screen edges (already there) — hosts/hearth-ui/apps/settings/src/cats/pers.rs: "\"kwin-edges\""
- [x] kcm_kwinxwayland (Legacy X11 App Support: Select which keys will be globally available to legacy X11 apps) → Hearth Settings › Full-screen games & X11 apps (already there) — hosts/hearth-ui/apps/settings/src/cats/gaming.rs: "\"kwin-games\""
- [x] kcm_landingpage (Quick Settings: Landing page with some basic settings) → Hearth Settings › Quick (already there) — hosts/hearth-ui/apps/settings/src/cats/home.rs: "\"quick\""
- [ ] kcm_lookandfeel (Global Theme: Choose global look and feel) → Hearth Settings › Global theme (new, batch 1: built, waiting for its CI pictures)
- [x] kcm_memory (Memory Information: Memory Information) → Hearth Settings › Info Center · Memory (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"info-memory\""
- [x] kcm_mobile_hotspot (Hotspot: Wi-Fi Hotspot) → Hearth Settings › Mobile hotspot (already there) — hosts/hearth-ui/apps/settings/src/cats/net.rs: "\"hotspot\""
- [x] kcm_mobile_power (Energy: Configure power management settings) → Hearth Settings › Power & battery (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"power\""
- [x] kcm_mobile_wifi (Wi-Fi: Wireless network setting) → Hearth Settings › Wi-Fi (already there) — hosts/hearth-ui/apps/settings/src/cats/net.rs: "\"wifi\""
- [x] kcm_mobile_wired (Wired Network: Wired network settings) → Hearth Settings › Connection details (already there) — hosts/hearth-ui/apps/settings/src/cats/net.rs: "\"net-details\""
- [x] kcm_mouse (Mouse: Mouse Controls) → Hearth Settings › Mouse & touchpad (already there) — hosts/hearth-ui/apps/settings/src/cats/bt.rs: "\"mouse\""
- [ ] kcm_netpref (Connection Preferences: Configure generic network preferences) → new page kde-netpref (batch 2): Connection preferences: timeouts, FTP passive mode
- [ ] kcm_network (Network interface information: Network interface information) → new page kde-info-network (batch 3): Info Center: network interfaces
- [x] kcm_networkmanagement (Wi-Fi & Networking: Edit network connections) → Hearth Settings › Wi-Fi (already there) — hosts/hearth-ui/apps/settings/src/cats/net.rs: "\"wifi\""
- [x] kcm_nightlight (Night Light: Adjust screen color temperature based on time) → Hearth Settings › Display (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"display\""
- [ ] kcm_nighttime (Day-Night Cycle: Configure the day-night cycle times) → new page kde-nighttime (batch 3): Day-night cycle: sunrise and sunset from your place or times you set
- [x] kcm_notifications (Notifications: Event Notifications and Actions) → Hearth Settings › Notifications (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"notifications\""
- [ ] kcm_opencl (OpenCL API information: OpenCL API information) → new page kde-info-opencl (batch 3): Info Center: OpenCL
- [x] kcm_pci (PCI information: PCI information) → Hearth Settings › Info Center · PCI (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"info-pci\""
- [ ] kcm_plasmakeyboard (On-Screen Keyboard: Configuration for Plasma Keyboard) → new page kde-plasmakeyboard (batch 3): Plasma Keyboard's own settings, and which virtual keyboard KWin uses
- [x] kcm_plasmalogin (Login Screen: Configure Login Manager) → Hearth Settings › Login screen (already there) — hosts/hearth-ui/apps/settings/src/cats/accounts.rs: "\"login-screen\""
- [x] kcm_plasmasearch (Plasma Search: Configure search settings) → Hearth Settings › Search (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"search\""
- [x] kcm_powerdevilprofilesconfig (Power Management: Power Management) → Hearth Settings › Plugged in, battery & lid (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"power-profiles\""
- [x] kcm_printer_manager (Printers: Configure your printers) → Hearth Settings › Printers (already there) — hosts/hearth-ui/apps/settings/src/cats/bt.rs: "\"printers\""
- [x] kcm_proxy (Proxy: Configure the proxy servers used) → Hearth Settings › Proxy (already there) — hosts/hearth-ui/apps/settings/src/cats/net.rs: "\"proxy\""
- [x] kcm_pulseaudio (Sound: Configure audio devices and volume) → Hearth Settings › Sound (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"sound\""
- [ ] kcm_push_notifications (Push Services: Configure notifications from online services) → new page kde-push (batch 2): Push services (KUnifiedPush)
- [ ] kcm_qtquicksettings (Plasma Renderer: No description available) → Hearth Settings › Plasma renderer (new, batch 1: built, waiting for its CI pictures)
- [x] kcm_recentFiles (Recent Files: Manage your file activity history preferences) → Hearth Settings › Recent files (already there) — hosts/hearth-ui/apps/settings/src/cats/privacy.rs: "\"recent-files\""
- [x] kcm_regionandlang (Region & Language: Language Setting, Numeric, Currency and Time Formats) → Hearth Settings › Region & language (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"region\""
- [ ] kcm_samba (Samba status monitor: Samba status monitor) → new page kde-info-samba (batch 3): Info Center: Samba shares and who is connected
- [ ] kcm_screenlocker (Screen Locking: Configure screen locking) → Hearth Settings › Screen locking (new, batch 1: built, waiting for its CI pictures)
- [x] kcm_sensors (Sensors: Sensors) → Hearth Settings › Info Center · Temperatures & fans (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"info-sensors\""
- [ ] kcm_smserver (Desktop Session: Desktop Session Login and Logout) → Hearth Settings › Desktop session (new, batch 1: built, waiting for its CI pictures)
- [ ] kcm_solid_actions (Device Actions: Manage actions available to the user when connecting new devices) → new page kde-device-actions (batch 2): Device actions: what can be done when a device is plugged in
- [x] kcm_soundtheme (System Sounds: Select a Sound Theme for Notifications) → Hearth Settings › Sounds (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"sounds\""
- [x] kcm_spellchecking (Spell Check: Spell Checker Dictionaries and Options) → Hearth Settings › Spell check (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"spellcheck\""
- [x] kcm_splashscreen (Splash Screen: Choose splash screen theme) → Hearth Settings › Window frames & sign-in splash (already there) — hosts/hearth-ui/apps/settings/src/cats/pers.rs: "\"look-more\""
- [ ] kcm_style (Application Style: Configure application style and behavior) → Hearth Settings › Application style (new, batch 1: built, waiting for its CI pictures)
- [x] kcm_tablet (Drawing Tablet: Configure drawing tablets and pens) → Hearth Settings › Drawing tablet & touchscreen (already there) — hosts/hearth-ui/apps/settings/src/cats/bt.rs: "\"kwin-tablet\""
- [x] kcm_touchpad (Touchpad: Touchpad settings) → Hearth Settings › Mouse & touchpad (already there) — hosts/hearth-ui/apps/settings/src/cats/bt.rs: "\"mouse\""
- [x] kcm_touchscreen (Touchscreen: No description available) → Hearth Settings › Drawing tablet & touchscreen (already there) — hosts/hearth-ui/apps/settings/src/cats/bt.rs: "\"kwin-tablet\""
- [ ] kcm_trash (Trash) → new page kde-trash (batch 2): The Trash's size limit and emptying it by itself
- [ ] kcm_updates (Software Update: Configure software update behavior) → new page kde-updates (batch 2): Discover's update settings: how often, notifications, offline updates
- [x] kcm_usb (USB devices attached to this computer: USB devices attached to this computer) → Hearth Settings › Info Center · USB (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"info-usb\""
- [x] kcm_users (Users: Manage user accounts) → Hearth Settings › Your account and other people (already there) — hosts/hearth-ui/apps/settings/src/cats/accounts.rs: "\"users\""
- [ ] kcm_vaapi (Information about the Video Acceleration API (VA-API): Information about the Video Acceleration API (VA-API)) → new page kde-info-vaapi (batch 3): Info Center: video acceleration (VA-API)
- [ ] kcm_virtualkeyboard (Virtual Keyboard: Select which virtual keyboard to use) → new page kde-plasmakeyboard (batch 3): Which virtual keyboard KWin uses (on the same page as Plasma Keyboard)
- [x] kcm_vulkan (Vulkan graphics API information: Vulkan graphics API information) → Hearth Settings › Info Center · Vulkan & OpenGL (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"info-graphics\""
- [x] kcm_wallpaper (Wallpaper: Configure the wallpaper) → Hearth Settings › Wallpaper per screen, slideshow & picture of the day (already there) — hosts/hearth-ui/apps/settings/src/cats/pers.rs: "\"wallpaper-more\""
- [ ] kcm_wayland (Information about the Wayland Compositor: Information about the Wayland Compositor) → new page kde-info-wayland (batch 3): Info Center: Wayland
- [ ] kcm_webshortcuts (Web Search Keywords: Configure Web search keywords) → new page kde-webshortcuts (batch 2): Web search keywords (gg:, wp:, ...)
- [ ] kcm_workspace (General Behavior: Configure general workspace behavior) → Hearth Settings › General behaviour (new, batch 1: built, waiting for its CI pictures)
- [ ] kcm_xserver (X-Server information: X-Server information) → new page kde-info-xserver (batch 3): Info Center: X server (Xwayland)
