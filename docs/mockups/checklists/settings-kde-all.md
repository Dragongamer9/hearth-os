# Every KDE System Settings page in Hearth Settings (1.16)

Release: 1.16
Mockups: next-update/settings-A2-display-mouse.png, next-update/settings-B2-display-controller.png

The owner (Nov 19, 2026): "Build them all into Hearth Settings": every KDE System Settings module (KCM) gets a real page
in Hearth Settings (the Rust Settings app, hosts/hearth-ui/apps/settings), in Hearth's look (the approved Settings
mockups above, both looks), so KDE's own System Settings is never needed on the Fedora edition.

**The list** is every module in Fedora 45's Plasma 6.8.3 (kcmshell6 --list in the Fedora image, CI job
kde-kcm-inventory.yml, Nov 19, 2026; KDE's website can't be read from the owner's sessions), plus the modules that are a
part of another (window frame themes, KRunner, Trash). 108 modules: 64 already had a Hearth page, 44 have a new page
(hosts/linux/desktop/shell/kde-settings.json, drawn by cats/kde.rs, saved by hearth-kde-settings), 0 are in the next
batches. A new page is ticked once its CI pictures were opened and checked (needs testing on a device: CI has no
KDE Plasma session, so KDE's own tools and D-Bus services were not run there; the files each page writes were).

**How the new pages save**: only the keys a page lists, the way the KCM does (the same file, group and key as the KCM's
kcfg in Plasma 6.8.3), through hearth_kconfig (atomic, .bak kept, every other line left alone, a file that can't be read
never written over, [$i] locks kept), then KDE's "config changed" signal and the program that reads it asked to read it
again. Where the KCM applies a choice with KDE's own tool (plasma-apply-colorscheme, -desktoptheme, -lookandfeel,
KDE's GTK settings service, logind for the firmware setup), the page runs the same tool. Opening a page writes nothing
(CI checks). On labwc the pages still read and save KDE's files and say which settings only KDE Plasma uses.

Not part of System Settings (each app's own settings window; nothing to build here): Dolphin's own pages
(kcm_dolphingeneral, kcm_dolphinviewmodes; Hearth Files has its own), KRunner plugins' own small settings windows
(kcm_krunner_charrunner, kcm_krunner_dictionary, kcm_krunner_kill, kcm_krunner_spellcheck: their switches are on the Search page, the rest on the KRunner page), KWin effects' own settings
windows (kcm_kwin4_genericscripted and each effect's: Window effects).

## Every module
- [x] systemsettings (KDE's System Settings app itself) → Hearth Settings › All KDE settings (new: every page in one list) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-settings\""
- [x] kcm_about-distro (About this System: Information About This System) → Hearth Settings › About (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"about\""
- [x] kcm_access (Accessibility: Configure Accessibility Options) → Hearth Settings › Accessibility (already there) — hosts/hearth-ui/apps/settings/src/cats/a11y.rs: "\"accessibility\""
- [x] kcm_activities (Activities: Activities help you to focus on a specific task) → Hearth Settings › Activities (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-activities\""
- [x] kcm_animations (Animations: Configure animation speed and style) → Hearth Settings › Window effects (already there) — hosts/hearth-ui/apps/settings/src/cats/pers.rs: "\"kwin-effects\""
- [x] kcm_app-permissions (Application Permissions: Configure Application Permissions) → Hearth Settings › App permissions (already there) — hosts/hearth-ui/apps/settings/src/cats/apps.rs: "\"app-permissions\""
- [x] kcm_audio_information (Audio Device Information: Audio Device Information) → Hearth Settings › More system information (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-info\""
- [x] kcm_audiocd (Audio CDs: Audiocd IO Worker Configuration) → Hearth Settings › Music CDs (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-cddb\""
- [x] kcm_auroraedecoration (Aurorae window frames) → Hearth Settings › Title bars (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-decorations\""
- [x] kcm_autostart (Autostart: Automatically Started Applications) → Hearth Settings › Autostart (already there) — hosts/hearth-ui/apps/settings/src/cats/apps.rs: "\"autostart\""
- [x] kcm_baloofile (File Search: Configure File Search) → Hearth Settings › Search (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"search\""
- [x] kcm_block_devices (Block Devices: Block Devices) → Hearth Settings › More system information (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-info\""
- [x] kcm_bluetooth (Bluetooth: Manage Bluetooth devices) → Hearth Settings › Bluetooth (already there) — hosts/hearth-ui/apps/settings/src/cats/bt.rs: "\"bluetooth\""
- [x] kcm_bolt (Thunderbolt: Thunderbolt Device Management) → Hearth Settings › Thunderbolt (already there) — hosts/hearth-ui/apps/settings/src/cats/bt.rs: "\"thunderbolt\""
- [x] kcm_breezedecoration (Breeze Window Decoration) → Hearth Settings › Title bars (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-decorations\""
- [x] kcm_cddb (CDDB Retrieval: Configure the CDDB Retrieval) → Hearth Settings › Music CDs (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-cddb\""
- [x] kcm_cellular_network (Cellular Network: Management of cellular networks) → Hearth Settings › Connection details (already there) — hosts/hearth-ui/apps/settings/src/cats/net.rs: "\"net-details\""
- [x] kcm_clock (Date & Time: Date and Time) → Hearth Settings › Date & time (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"datetime\""
- [x] kcm_colord (Color Management: Manage color profile of devices) → Hearth Settings › Colour profiles (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-colord\""
- [x] kcm_colors (Colors: Choose color scheme) → Hearth Settings › Colours (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-colors\""
- [x] kcm_componentchooser (Default Applications: Choose default Applications) → Hearth Settings › Default apps (already there) — hosts/hearth-ui/apps/settings/src/cats/apps.rs: "\"defaults\""
- [x] kcm_cpu (Advanced CPU Information: Advanced CPU Information) → Hearth Settings › Info Center (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"info-cpu\""
- [x] kcm_cursortheme (Pointers: Choose mouse pointer theme) → Hearth Settings › Fonts, cursor & icons (already there) — hosts/hearth-ui/apps/settings/src/cats/pers.rs: "\"fonts\""
- [x] kcm_desktoppaths (Locations: Location for Personal Files) → Hearth Settings › Your folders (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-locations\""
- [x] kcm_desktoptheme (Plasma Style: Choose Plasma style) → Hearth Settings › Plasma style (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-plasma-style\""
- [x] kcm_device_automounter (Configure automatic mounting of disks and volumes: Configure automatic mounting of disks and volumes) → Hearth Settings › USB devices & drives (already there) — hosts/hearth-ui/apps/settings/src/cats/bt.rs: "\"usb-devices\""
- [x] kcm_disks (SMART Status: Storage device health monitoring) → Hearth Settings › Drives & undo (already there) — hosts/hearth-ui/apps/settings/src/cats/hs.rs: "\"hs-drives\""
- [x] kcm_edid (Extended Display Identification Data: Extended Display Identification Data) → Hearth Settings › More system information (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-info\""
- [x] kcm_egl (EGL information: EGL information) → Hearth Settings › Info Center · Vulkan & OpenGL (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"info-graphics\""
- [x] kcm_energyinfo (Energy Consumption Statistics: Energy Consumption Statistics) → Hearth Settings › Info Center · Energy (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"info-energy\""
- [x] kcm_feedback (User Feedback: Configure user feedback settings) → Hearth Settings › KDE user feedback (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-feedback\""
- [x] kcm_filetypes (File Associations: Configure file associations) → Hearth Settings › File types (already there) — hosts/hearth-ui/apps/settings/src/cats/apps.rs: "\"file-types\""
- [x] kcm_firmware_security (Firmware Security Information: Firmware Security Information) → Hearth Settings › Firmware & device security (already there) — hosts/hearth-ui/apps/settings/src/cats/privacy.rs: "\"firmware\""
- [x] kcm_fontinst (Font Management: Install, manage and preview fonts) → Hearth Settings › Font management (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-fontinst\""
- [x] kcm_fonts (Fonts: Configure user interface fonts) → Hearth Settings › Fonts, cursor & icons (already there) — hosts/hearth-ui/apps/settings/src/cats/pers.rs: "\"fonts\""
- [x] kcm_gamecontroller (Game Controller: Test and Configure Game Controllers) → Hearth Settings › Controllers in KDE (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-gamecontroller\""
- [x] kcm_glx (GLX information: GLX information) → Hearth Settings › Info Center · Vulkan & OpenGL (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"info-graphics\""
- [x] kcm_icons (Icons: Choose icon theme) → Hearth Settings › Fonts, cursor & icons (already there) — hosts/hearth-ui/apps/settings/src/cats/pers.rs: "\"fonts\""
- [x] kcm_interrupts (Interrupt information: Interrupt information) → Hearth Settings › Info Center · Interrupts (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"info-interrupts\""
- [x] kcm_kaccounts (Online Accounts: Configure your internet accounts such as Google, Live, Owncloud) → Hearth Settings › Online accounts (already there) — hosts/hearth-ui/apps/settings/src/cats/accounts.rs: "\"online-accounts\""
- [x] kcm_kded (Background Services: Configure background services) → Hearth Settings › KDE background services (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-kded\""
- [x] kcm_keyboard (Keyboard: Keyboard Hardware and Layout) → Hearth Settings › Keyboard (already there) — hosts/hearth-ui/apps/settings/src/cats/bt.rs: "\"keyboard\""
- [x] kcm_keys (Shortcuts: Configure Keyboard Shortcuts) → Hearth Settings › KDE shortcuts (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-shortcuts\""
- [x] kcm_krdpserver (Remote Desktop: Configure Remote Desktop) → Hearth Settings › Remote desktop (RDP) (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-krdp\""
- [x] kcm_krunnersettings (KRunner Settings) → Hearth Settings › KRunner (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-krunner\""
- [x] kcm_kscreen (Display Configuration: Manage and configure monitors and displays) → Hearth Settings › Display (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"display\""
- [x] kcm_kwallet5 (KDE Wallet Configuration: KDE Wallet Configuration) → Hearth Settings › Passwords (already there) — hosts/hearth-ui/apps/settings/src/cats/privacy.rs: "\"passwords\""
- [x] kcm_kwin_effects (Desktop Effects: Configure compositor settings for desktop effects) → Hearth Settings › Window effects (already there) — hosts/hearth-ui/apps/settings/src/cats/pers.rs: "\"kwin-effects\""
- [x] kcm_kwin_scripts (KWin Scripts: Manage KWin scripts) → Hearth Settings › KWin scripts (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-kwin-scripts\""
- [x] kcm_kwin_virtualdesktops (Virtual Desktops: Configure navigation, number and layout of virtual desktops) → Hearth Settings › Virtual desktops (already there) — hosts/hearth-ui/apps/settings/src/cats/pers.rs: "\"kwin-desktops\""
- [x] kcm_kwindecoration (Window Decorations: Configure window titlebars and borders) → Hearth Settings › Title bars (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-decorations\""
- [x] kcm_kwinoptions (Window Behavior: Configure window actions and behavior) → Hearth Settings › Window behaviour (already there) — hosts/hearth-ui/apps/settings/src/cats/pers.rs: "\"kwin-behaviour\""
- [x] kcm_kwinrules (Window Rules: Individual Window Behavior) → Hearth Settings › Window rules (already there) — hosts/hearth-ui/apps/settings/src/cats/apps.rs: "\"kwin-rules\""
- [x] kcm_kwinscreenedges (Configure active screen corners and edges: Configure active screen corners and edges) → Hearth Settings › Screen edges (already there) — hosts/hearth-ui/apps/settings/src/cats/pers.rs: "\"kwin-edges\""
- [x] kcm_kwinsupportinfo (KWin Support information: KWin Support information) → Hearth Settings › More system information (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-info\""
- [x] kcm_kwintabbox (Task Switcher: Navigation Through Windows) → Hearth Settings › Task switcher (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-tabbox\""
- [x] kcm_kwintouchscreen (Configure touch screen swipe gestures: Configure touch screen swipe gestures) → Hearth Settings › Screen edges (already there) — hosts/hearth-ui/apps/settings/src/cats/pers.rs: "\"kwin-edges\""
- [x] kcm_kwinxwayland (Legacy X11 App Support: Select which keys will be globally available to legacy X11 apps) → Hearth Settings › Full-screen games & X11 apps (already there) — hosts/hearth-ui/apps/settings/src/cats/gaming.rs: "\"kwin-games\""
- [x] kcm_landingpage (Quick Settings: Landing page with some basic settings) → Hearth Settings › Quick (already there) — hosts/hearth-ui/apps/settings/src/cats/home.rs: "\"quick\""
- [x] kcm_lookandfeel (Global Theme: Choose global look and feel) → Hearth Settings › Global theme (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-global-theme\""
- [x] kcm_memory (Memory Information: Memory Information) → Hearth Settings › Info Center · Memory (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"info-memory\""
- [x] kcm_mobile_hotspot (Hotspot: Wi-Fi Hotspot) → Hearth Settings › Mobile hotspot (already there) — hosts/hearth-ui/apps/settings/src/cats/net.rs: "\"hotspot\""
- [x] kcm_mobile_power (Energy: Configure power management settings) → Hearth Settings › Power & battery (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"power\""
- [x] kcm_mobile_wifi (Wi-Fi: Wireless network setting) → Hearth Settings › Wi-Fi (already there) — hosts/hearth-ui/apps/settings/src/cats/net.rs: "\"wifi\""
- [x] kcm_mobile_wired (Wired Network: Wired network settings) → Hearth Settings › Connection details (already there) — hosts/hearth-ui/apps/settings/src/cats/net.rs: "\"net-details\""
- [x] kcm_mouse (Mouse: Mouse Controls) → Hearth Settings › Mouse & touchpad (already there) — hosts/hearth-ui/apps/settings/src/cats/bt.rs: "\"mouse\""
- [x] kcm_netpref (Connection Preferences: Configure generic network preferences) → Hearth Settings › Connection preferences (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-netpref\""
- [x] kcm_network (Network interface information: Network interface information) → Hearth Settings › More system information (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-info\""
- [x] kcm_networkmanagement (Wi-Fi & Networking: Edit network connections) → Hearth Settings › Wi-Fi (already there) — hosts/hearth-ui/apps/settings/src/cats/net.rs: "\"wifi\""
- [x] kcm_nightlight (Night Light: Adjust screen color temperature based on time) → Hearth Settings › Display (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"display\""
- [x] kcm_nighttime (Day-Night Cycle: Configure the day-night cycle times) → Hearth Settings › Day and night times (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-nighttime\""
- [x] kcm_notifications (Notifications: Event Notifications and Actions) → Hearth Settings › Notifications (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"notifications\""
- [x] kcm_opencl (OpenCL API information: OpenCL API information) → Hearth Settings › More system information (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-info\""
- [x] kcm_pci (PCI information: PCI information) → Hearth Settings › Info Center · PCI (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"info-pci\""
- [x] kcm_plasmakeyboard (On-Screen Keyboard: Configuration for Plasma Keyboard) → Hearth Settings › Plasma Keyboard (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-plasmakeyboard\""
- [x] kcm_plasmalogin (Login Screen: Configure Login Manager) → Hearth Settings › Login screen (already there) — hosts/hearth-ui/apps/settings/src/cats/accounts.rs: "\"login-screen\""
- [x] kcm_plasmasearch (Plasma Search: Configure search settings) → Hearth Settings › Search (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"search\""
- [x] kcm_powerdevilprofilesconfig (Power Management: Power Management) → Hearth Settings › Plugged in, battery & lid (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"power-profiles\""
- [x] kcm_printer_manager (Printers: Configure your printers) → Hearth Settings › Printers (already there) — hosts/hearth-ui/apps/settings/src/cats/bt.rs: "\"printers\""
- [x] kcm_proxy (Proxy: Configure the proxy servers used) → Hearth Settings › Proxy (already there) — hosts/hearth-ui/apps/settings/src/cats/net.rs: "\"proxy\""
- [x] kcm_pulseaudio (Sound: Configure audio devices and volume) → Hearth Settings › Sound (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"sound\""
- [x] kcm_push_notifications (Push Services: Configure notifications from online services) → Hearth Settings › Push services (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-push\""
- [x] kcm_qtquicksettings (Plasma Renderer: No description available) → Hearth Settings › Plasma renderer (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-renderer\""
- [x] kcm_recentFiles (Recent Files: Manage your file activity history preferences) → Hearth Settings › Recent files (already there) — hosts/hearth-ui/apps/settings/src/cats/privacy.rs: "\"recent-files\""
- [x] kcm_regionandlang (Region & Language: Language Setting, Numeric, Currency and Time Formats) → Hearth Settings › Region & language (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"region\""
- [x] kcm_samba (Samba status monitor: Samba status monitor) → Hearth Settings › More system information (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-info\""
- [x] kcm_screenlocker (Screen Locking: Configure screen locking) → Hearth Settings › Screen locking (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-screenlock\""
- [x] kcm_sensors (Sensors: Sensors) → Hearth Settings › Info Center · Temperatures & fans (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"info-sensors\""
- [x] kcm_smserver (Desktop Session: Desktop Session Login and Logout) → Hearth Settings › Desktop session (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-session\""
- [x] kcm_solid_actions (Device Actions: Manage actions available to the user when connecting new devices) → Hearth Settings › Device actions (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-device-actions\""
- [x] kcm_soundtheme (System Sounds: Select a Sound Theme for Notifications) → Hearth Settings › Sounds (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"sounds\""
- [x] kcm_spellchecking (Spell Check: Spell Checker Dictionaries and Options) → Hearth Settings › Spell check (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"spellcheck\""
- [x] kcm_splashscreen (Splash Screen: Choose splash screen theme) → Hearth Settings › Window frames & sign-in splash (already there) — hosts/hearth-ui/apps/settings/src/cats/pers.rs: "\"look-more\""
- [x] kcm_style (Application Style: Configure application style and behavior) → Hearth Settings › Application style (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-style\""
- [x] kcm_tablet (Drawing Tablet: Configure drawing tablets and pens) → Hearth Settings › Drawing tablet & touchscreen (already there) — hosts/hearth-ui/apps/settings/src/cats/bt.rs: "\"kwin-tablet\""
- [x] kcm_touchpad (Touchpad: Touchpad settings) → Hearth Settings › Mouse & touchpad (already there) — hosts/hearth-ui/apps/settings/src/cats/bt.rs: "\"mouse\""
- [x] kcm_touchscreen (Touchscreen: No description available) → Hearth Settings › Drawing tablet & touchscreen (already there) — hosts/hearth-ui/apps/settings/src/cats/bt.rs: "\"kwin-tablet\""
- [x] kcm_trash (Trash) → Hearth Settings › Trash (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-trash\""
- [x] kcm_updates (Software Update: Configure software update behavior) → Hearth Settings › App updates (Discover) (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-updates\""
- [x] kcm_usb (USB devices attached to this computer: USB devices attached to this computer) → Hearth Settings › Info Center · USB (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"info-usb\""
- [x] kcm_users (Users: Manage user accounts) → Hearth Settings › Your account and other people (already there) — hosts/hearth-ui/apps/settings/src/cats/accounts.rs: "\"users\""
- [x] kcm_vaapi (Information about the Video Acceleration API (VA-API): Information about the Video Acceleration API (VA-API)) → Hearth Settings › More system information (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-info\""
- [x] kcm_virtualkeyboard (Virtual Keyboard: Select which virtual keyboard to use) → Hearth Settings › Plasma Keyboard (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-plasmakeyboard\""
- [x] kcm_vulkan (Vulkan graphics API information: Vulkan graphics API information) → Hearth Settings › Info Center · Vulkan & OpenGL (already there) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"info-graphics\""
- [x] kcm_wallpaper (Wallpaper: Configure the wallpaper) → Hearth Settings › Wallpaper per screen, slideshow & picture of the day (already there) — hosts/hearth-ui/apps/settings/src/cats/pers.rs: "\"wallpaper-more\""
- [x] kcm_wayland (Information about the Wayland Compositor: Information about the Wayland Compositor) → Hearth Settings › More system information (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-info\""
- [x] kcm_webshortcuts (Web Search Keywords: Configure Web search keywords) → Hearth Settings › Web search keywords (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-webshortcuts\""
- [x] kcm_workspace (General Behavior: Configure general workspace behavior) → Hearth Settings › General behaviour (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-workspace\""
- [x] kcm_xserver (X-Server information: X-Server information) → Hearth Settings › More system information (new) — hosts/linux/desktop/shell/kde-settings.json: "\"id\": \"kde-info\""


## The parts that were open (the owner, Nov 20: "go ahead", all into 1.16)

- [x] KDE shortcuts: your own commands and scripts with a shortcut (KDE's "Add New › Command or Script": add, change, remove) — hosts/linux/desktop/shell/hearth-kde-settings: "X-KDE-GlobalAccel-CommandShortcut=true"
- [x] Push services: Gotify and NextPush, the token or app password typed on the page and handed only to KDE's push service (KUnifiedPush keeps it) — hosts/linux/desktop/shell/hearth-kde-settings: "def push_signin(provider, url, user):"
- [x] Remote desktop (RDP): other people with their own passwords, kept in KDE Wallet only (QtKeychain's KRDP folder) — hosts/linux/desktop/shell/hearth-kde-settings: "wallet_write('KRDP', user, secret)"
- [x] Global theme, Plasma style, Colours, Title bars: "Get New…" from the KDE Store (knewstuff's own descriptions and the store's API; installs in your folders only) — hosts/linux/desktop/shell/hearth-kde-settings: "def store_install(kind, cid):"
- [x] Font management: a preview of each font, drawn in itself — hosts/hearth-ui/apps/settings/src/cats/kde.rs: "The quick brown fox jumps over the lazy dog"
- [x] Device actions: editing the system's actions as your own copy (the system's file never touched) and "Reset to the system version" — hosts/linux/desktop/shell/hearth-kde-settings: "def device_action_reset(fname):"
- [x] Activities: each one's icon, its own shortcut to switch to it, and "remember what's opened in it" — hosts/linux/desktop/shell/hearth-kde-settings: "OTR = 'org.kde.ActivityManager.Resources.Scoring/isOTR/%s'"
- [x] Controllers in KDE: a live test of buttons, sticks, triggers and the D-pad, listening only while the page is open — hosts/hearth-ui/apps/settings/src/cats/kde_pad.rs: "break; // the page was closed"
- [x] Display: Calibrate HDR in Hearth's own three steps (no KDE window) — hosts/hearth-ui/apps/settings/src/cats/system_display.rs: "Msg::Go(\"kde-hdr\")"
